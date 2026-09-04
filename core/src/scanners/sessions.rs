use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const CLAUDE_GENERIC_SESSION_TITLE: &str = "Claude session";
const CLAUDE_SESSION_FILE_LIMIT: usize = 512;
const CLAUDE_SESSION_TAIL_BYTES: usize = 256 * 1024;
/// How far into a transcript to look for the entrypoint that launched it. The
/// field rides on every `user`, `assistant` and `attachment` record, so it turns
/// up early; across 1091 transcripts here 64 KB reaches it in 1050 of them, and
/// the rest are kept and settled when the file is parsed in full.
const CLAUDE_SESSION_LAUNCH_PROBE_BYTES: usize = 64 * 1024;
const CODEX_GENERIC_SESSION_TITLE: &str = "Codex session";
const CODEX_UNTITLED_INDEX_TITLE: &str = "Untitled Codex session";
const CODEX_SESSION_FILE_LIMIT: usize = 512;
/// Length of the thread id Codex ends every rollout file name with.
const CODEX_THREAD_ID_LEN: usize = 36;
/// The first user message of a Codex session sits behind the opening metadata
/// record, which carries the whole system prompt and can run past 40 KB. On this
/// machine 256 KB reaches the first typed prompt in 59 of 67 top-level sessions;
/// the rest fall back to whatever the tail window offers.
const CODEX_SESSION_HEAD_BYTES: usize = 256 * 1024;
const CODEX_SESSION_TAIL_BYTES: usize = 256 * 1024;
/// Enough to hold the opening `session_meta` line whole. Measured across 687
/// rollout files: median 27 KB, largest 44 KB.
const CODEX_SESSION_META_PROBE_BYTES: usize = 64 * 1024;
const CMUX_SESSION_RESULT_HEADROOM: usize = 256;
/// How long the one-line "last thing said" on a row may be, prefix included.
/// Long enough to recognise the turn, short enough to stay on one line.
const AGENT_SESSION_TURN_PREVIEW_CHARS: usize = 120;
const AGENT_SESSION_USER_TURN_PREFIX: &str = "You: ";
const AGENT_SESSION_AGENT_TURN_PREFIX: &str = "Agent: ";
/// How much of a kept turn is carried to the panel. The one-line preview is cut
/// to 120 characters because it has to fit on a row; the expanded card scrolls,
/// so it can hold a real answer. Bounded all the same — an agent's last turn can
/// run to tens of kilobytes, and no one reads that off a session card.
const AGENT_SESSION_TURN_TEXT_CHARS: usize = 4_000;
const AGENT_SESSION_RESULT_LIMIT: usize =
    CODEX_SESSION_FILE_LIMIT + CLAUDE_SESSION_FILE_LIMIT + CMUX_SESSION_RESULT_HEADROOM;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionRecord {
    pub provider: String,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub model: Option<String>,
    pub project_path: Option<String>,
    pub last_activity: Option<String>,
    pub resume_commands: Vec<String>,
    /// Absolute path of the transcript file this record was scanned out of, so
    /// the app can open it, show it in the file manager, or read it back. `None`
    /// when no single file describes the record — an index row, or a hook state
    /// file that is not a transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    /// What the scan worked out about the session from its own title, folder and
    /// resume command — the branch it is on, the task it belongs to, the pull
    /// request it opened, and a short "who and where" label. Filled in once, at
    /// the end of the scan, so everything that builds a record along the way can
    /// leave them empty; a row draws a chip only for the ones that are there.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_request_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
    /// How many turns of the conversation the scan saw, and the last one of
    /// them, so a row can say "12 messages" and show what was last said.
    ///
    /// Both are read out of the transcript text the scan already had in hand
    /// for the title — no extra file is opened and no read window is widened.
    /// That means the count is a floor rather than a total: the scan reads a
    /// bounded window of each file, so a long session reports the turns inside
    /// that window and no more. A row that says "12 messages" is saying "at
    /// least 12", which is the honest thing a bounded read can say.
    ///
    /// A turn is something the user typed or something the agent said back in
    /// words. Tool calls and their results are how the work gets done, not what
    /// was said, so they are not counted — counting them would put "418
    /// messages" on a row where the two of them exchanged a dozen.
    ///
    /// The two agents write their transcripts differently and the count has to
    /// mean the same thing on both kinds of row, because they sit on the same
    /// list. Claude Code writes one record per reply. Codex writes a separate
    /// record for every paragraph it narrates between tool calls, so a run of
    /// consecutive Codex agent records is counted as the ONE thing the agent
    /// said back; without that a Codex row read "691 messages" beside a Claude
    /// row reading "33" for a longer conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_turn_preview: Option<String>,
    /// The last thing each side said, newest-last, so an expanded card can show
    /// the end of the conversation rather than one truncated line of it.
    ///
    /// At most two entries: the user's most recent turn and the agent's most
    /// recent one. Which came last varies — a session left mid-answer ends on
    /// the agent, one left waiting ends on the user — so they are ordered by
    /// when they were said and the card labels them rather than assuming.
    ///
    /// This is what tells two sessions in the same folder apart. A title and a
    /// timestamp do not, when a person ran nine of them against the same
    /// repository in one afternoon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub latest_turns: Vec<AgentSessionTurn>,
    /// The repository this session's folder belongs to, as git reports it, so a
    /// project's main checkout and its worktrees land in one group under the
    /// project folder's name. Empty when the folder is gone from disk or was
    /// never in a repository; the panel then groups on the folder itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
}

/// One remembered turn: who spoke, and what they said.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionTurn {
    /// `user` or `agent`.
    pub speaker: String,
    pub text: String,
}

/// The two speakers, kept apart from the wire strings so a typo cannot make a
/// turn silently vanish from the pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnSpeaker {
    User,
    Agent,
}

impl TurnSpeaker {
    fn wire_name(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
        }
    }
}

/// The newest turn from each speaker, in the order they were said.
///
/// Every transcript is walked front to back, so a later line always replaces an
/// earlier one from the same speaker, and the position it was seen at is what
/// puts the pair back in order at the end.
#[derive(Debug, Default, Clone)]
struct LatestTurnPair {
    user: Option<(usize, String)>,
    agent: Option<(usize, String)>,
}

impl LatestTurnPair {
    fn remember(&mut self, speaker: TurnSpeaker, position: usize, text: String) {
        let slot = match speaker {
            TurnSpeaker::User => &mut self.user,
            TurnSpeaker::Agent => &mut self.agent,
        };
        *slot = Some((position, text));
    }

    fn into_turns(self) -> Vec<AgentSessionTurn> {
        let mut kept: Vec<(usize, TurnSpeaker, String)> = Vec::new();
        if let Some((position, text)) = self.user {
            kept.push((position, TurnSpeaker::User, text));
        }
        if let Some((position, text)) = self.agent {
            kept.push((position, TurnSpeaker::Agent, text));
        }
        kept.sort_by_key(|(position, _, _)| *position);
        kept.into_iter()
            .map(|(_, speaker, text)| AgentSessionTurn {
                speaker: speaker.wire_name().to_string(),
                text,
            })
            .collect()
    }
}

/// A turn's text as the expanded card shows it: trimmed, and cut to a length a
/// person will actually scroll through. Line breaks survive — they are how a
/// list of steps or a block of code stays readable.
fn agent_session_turn_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= AGENT_SESSION_TURN_TEXT_CHARS {
        return trimmed.to_string();
    }

    let cut: String = trimmed
        .chars()
        .take(AGENT_SESSION_TURN_TEXT_CHARS.saturating_sub(1))
        .collect();
    format!("{cut}…")
}

/// One line of "who said what", the way a row shows it: `You: …` or `Agent: …`,
/// whitespace squeezed to single spaces, cut to fit on one line.
fn agent_session_turn_preview(prefix: &str, text: &str) -> String {
    compact_text(&format!("{prefix}{text}"), AGENT_SESSION_TURN_PREVIEW_CHARS)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionDerivedMetadata {
    pub branch_hint: Option<String>,
    pub task_id: Option<String>,
    pub pull_request_hint: Option<String>,
    pub link_hint: Option<String>,
    pub source_label: String,
}

pub fn derive_agent_session_metadata(record: &AgentSessionRecord) -> AgentSessionDerivedMetadata {
    AgentSessionDerivedMetadata {
        branch_hint: first_agent_session_hint(record, branch_hint_from_text),
        task_id: first_agent_session_hint(record, task_id_from_text),
        pull_request_hint: first_agent_session_hint(record, pull_request_hint_from_text),
        link_hint: first_agent_session_hint(record, link_hint_from_text),
        source_label: agent_session_source_label(record),
    }
}

/// Writes the derived branch, task, pull request and source label onto every
/// record, so the shell reads them off the row it already has instead of parsing
/// titles again on the other side of the bridge.
///
/// Run once over the finished list rather than at each construction site: a
/// session is described by several files, and only the merged record has the
/// title, folder and resume command the hints are read from.
/// The repository a folder belongs to, asked of git rather than guessed from the
/// path.
///
/// Guessing was the problem. The layouts in use here disagree with each other —
/// `dev/work/worktrees/<repo>/<checkout>`, `dev/work/<repo>/worktrees/<checkout>`
/// and `.codex/worktrees/<hash>/<repo>` all mean "a checkout of <repo>" while
/// putting the repository's name in a different place — so no pattern reads all
/// three, and every one that was tried filed some of a repository's sessions
/// under a second project with the same name.
///
/// Git already knows. Walking up from the folder, the first `.git` found ends
/// the search:
///
/// - a `.git` DIRECTORY means this folder is the repository itself;
/// - a `.git` FILE means this folder is a worktree, and the file says where the
///   real repository is: `gitdir: /path/to/repo/.git/worktrees/<name>`. The part
///   before `/.git/` is the main checkout, which is the answer.
///
/// `None` when the folder is gone from disk or was never in a repository, and
/// the caller then falls back to the path it already had.
fn git_project_root(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        let marker = ancestor.join(".git");

        if marker.is_dir() {
            return Some(ancestor.to_path_buf());
        }

        if marker.is_file() {
            let text = fs::read_to_string(&marker).ok()?;
            let target = text.trim().strip_prefix("gitdir:")?.trim();
            let main = Path::new(target)
                .ancestors()
                .find(|candidate| candidate.file_name() == Some(OsStr::new(".git")))?
                .parent()?;
            return Some(main.to_path_buf());
        }
    }

    None
}

pub fn with_derived_agent_session_metadata(
    mut records: Vec<AgentSessionRecord>,
) -> Vec<AgentSessionRecord> {
    // One answer per folder. A busy repository has dozens of sessions in the
    // same checkout, and asking git the same question dozens of times is a
    // filesystem walk each time for a reply that cannot change mid-scan.
    let mut roots: HashMap<String, Option<String>> = HashMap::new();

    for record in records.iter_mut() {
        let metadata = derive_agent_session_metadata(record);
        record.branch_hint = metadata.branch_hint;
        record.task_id = metadata.task_id;
        record.pull_request_hint = metadata.pull_request_hint;
        record.source_label = Some(metadata.source_label);

        if let Some(folder) = record.project_path.clone() {
            record.project_root = roots
                .entry(folder.clone())
                .or_insert_with(|| {
                    git_project_root(Path::new(&folder))
                        .map(|root| root.to_string_lossy().into_owned())
                })
                .clone();
        }
    }

    records
}

/// Write the transcript file a batch of records was read out of onto each of
/// them.
///
/// The parsers take text, not files, so the path is stamped on here instead of
/// being threaded through every one of them. Every record in the batch came out
/// of the same file, which is what makes that correct as well as shorter.
fn with_log_path(records: Vec<AgentSessionRecord>, log_path: &Path) -> Vec<AgentSessionRecord> {
    let path = log_path.to_string_lossy().into_owned();
    records
        .into_iter()
        .map(|mut record| {
            record.log_path = Some(path.clone());
            record
        })
        .collect()
}

/// Which rollout files the codex scan reads, given every rollout file newest
/// first and the thread ids the session index listed.
///
/// The newest `CODEX_SESSION_FILE_LIMIT` files are read as before. On top of
/// those, an older file is read when the index still lists its thread, because
/// the index row alone carries no working folder: without the rollout the row
/// reaches the History panel with nothing to resume into, and both Resume and
/// Continue in New Session have no folder to hand the agent. The index is the
/// bound here, so this cannot grow past the number of rows a person can see.
fn codex_rollout_files_to_read(files: Vec<PathBuf>, indexed_ids: &HashSet<String>) -> Vec<PathBuf> {
    files
        .into_iter()
        .enumerate()
        .filter(|(position, file)| {
            *position < CODEX_SESSION_FILE_LIMIT || rollout_file_is_indexed(file, indexed_ids)
        })
        .map(|(_, file)| file)
        .collect()
}

/// True when a rollout file's name carries one of the indexed thread ids. Codex
/// names the file `rollout-<timestamp>-<id>.jsonl`, so the id is readable
/// without opening the file.
fn rollout_file_is_indexed(file: &Path, indexed_ids: &HashSet<String>) -> bool {
    let Some(name) = file.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    indexed_ids.iter().any(|id| name.ends_with(id.as_str()))
}

/// The thread id a rollout file is named for, read off the name rather than out
/// of the file. `rollout-<timestamp>-<uuid>.jsonl` ends with the 36-character
/// id, so the trailing 36 characters are it.
fn rollout_file_thread_id(file: &Path) -> Option<String> {
    let stem = file.file_stem().and_then(|stem| stem.to_str())?;
    let start = stem.len().checked_sub(CODEX_THREAD_ID_LEN)?;
    stem.get(start..).map(ToOwned::to_owned)
}

/// Index rows for threads whose rollout file is gone, dropped.
///
/// Codex keeps a `session_index.jsonl` row forever but prunes the rollout file
/// underneath it, and the row itself holds only an id, a name and a timestamp.
/// A row with no file left is unusable three times over: it carries no working
/// folder, so the History panel can only file it under "Other"; it has no
/// transcript, so nothing can be imported or resumed from it; and the sub-agent
/// marker lives in the file, so a thread Codex spawned for itself cannot be told
/// apart from one the user started. Measured on this machine, 266 of 342 rows
/// were in that state and they were the bulk of both complaints.
fn drop_codex_sessions_without_rollouts(
    records: Vec<AgentSessionRecord>,
    rollout_files: &[PathBuf],
) -> Vec<AgentSessionRecord> {
    let rollout_ids: HashSet<String> = rollout_files
        .iter()
        .filter_map(|file| rollout_file_thread_id(file))
        .collect();

    records
        .into_iter()
        .filter(|record| rollout_ids.contains(&record.id))
        .collect()
}

pub fn scan_sessions() -> Vec<AgentSessionRecord> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };

    scan_sessions_from_home(&home, None)
}

pub fn scan_sessions_for_project(project_root: &str) -> Vec<AgentSessionRecord> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };

    scan_sessions_from_home(&home, Some(project_root))
}

/// Read the bounded detail window for one transcript already named by History.
/// Project expansion uses the cached summary scan; only opening one card pays
/// for transcript text, and it opens one file rather than walking every session
/// on the machine again.
pub fn scan_session_details(log_path: &str) -> Vec<AgentSessionRecord> {
    let path = Path::new(log_path);
    let Ok(contents) =
        read_head_and_tail_utf8(path, CODEX_SESSION_HEAD_BYTES, CODEX_SESSION_TAIL_BYTES)
    else {
        return Vec::new();
    };

    let mut codex = parse_codex_rollout_jsonl(&contents);
    if !codex.is_empty() {
        if let Ok(tail) = read_tail_utf8(path, CODEX_SESSION_TAIL_BYTES) {
            let spoken = codex_turns_in(&tail);
            if !spoken.is_empty() {
                for record in codex.iter_mut() {
                    record.latest_turns = spoken.clone();
                }
            }
        }
        return with_derived_agent_session_metadata(with_log_path(codex, path));
    }

    let project_path = path
        .parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .and_then(decode_claude_project_dir)
        .unwrap_or_default();
    let Ok(tail) = read_tail_utf8(path, CLAUDE_SESSION_TAIL_BYTES) else {
        return Vec::new();
    };
    with_derived_agent_session_metadata(with_log_path(
        parse_claude_jsonl(&tail, &project_path),
        path,
    ))
}

fn scan_sessions_from_home(home: &Path, project_root: Option<&str>) -> Vec<AgentSessionRecord> {
    let mut records = Vec::new();
    let mut project_roots = HashMap::new();

    // Sort the rollout files into "the user started this" and "Codex spawned
    // this for itself" in one pass, keeping the ids of the second kind. Dropping
    // sub-agent threads here, BEFORE the file budget, is for the same reason the
    // Claude path drops its subagents first: they outnumber the user's own
    // sessions nine to one, so a budget applied first would evict the sessions
    // the rail exists to show.
    let mut codex_files = Vec::new();
    let mut codex_subagent_ids = HashSet::new();
    for (file, marker, cwd) in codex_rollout_heads(jsonl_files(&home.join(".codex/sessions"))) {
        match marker {
            Some(marker) if marker.spawned_by_codex => codex_subagent_ids.extend(marker.id),
            // No marker, or nothing readable: keep the file. Older Codex builds
            // predate the field, and losing a session is the worse mistake.
            _ if project_root.is_some_and(|root| {
                !codex_cwd_matches_project(cwd.as_deref(), root, &mut project_roots)
            }) => {}
            _ => codex_files.push(file),
        }
    }

    // Codex eventually moves a thread's rollout file into `archived_sessions`
    // but leaves its index row behind, so for those threads the archive is the
    // only evidence left of what kind of thread it was.
    for file in jsonl_files(&home.join(".codex/archived_sessions")) {
        if let Some(marker) = codex_rollout_thread_marker(&file) {
            if marker.spawned_by_codex {
                codex_subagent_ids.extend(marker.id);
            }
        }
    }

    let mut codex_records = Vec::new();
    let codex_index = home.join(".codex/session_index.jsonl");
    if let Ok(contents) = fs::read_to_string(codex_index) {
        codex_records.extend(parse_codex_index_jsonl(&contents));
    }
    codex_records = drop_codex_subagent_sessions(codex_records, &codex_subagent_ids);
    codex_records = drop_codex_sessions_without_rollouts(codex_records, &codex_files);

    codex_files.sort_by(|a, b| modified_time(b).cmp(&modified_time(a)));
    let indexed_ids: HashSet<String> = codex_records
        .iter()
        .map(|record| record.id.clone())
        .collect();
    let mut codex_metadata = Vec::new();
    for file in codex_rollout_files_to_read(codex_files, &indexed_ids) {
        let Ok(contents) =
            read_head_and_tail_utf8(&file, CODEX_SESSION_HEAD_BYTES, CODEX_SESSION_TAIL_BYTES)
        else {
            continue;
        };
        let mut parsed = parse_codex_rollout_jsonl(&contents);

        // The turns above may be the session's FIRST ones, and the card calls
        // them its latest.
        //
        // The read that produced them is the head of the file joined to its
        // tail: the head is where the session's own record and its opening
        // prompt live, and both are needed. But a Codex transcript spends most
        // of its length on tool calls and reasoning, so the tail often holds no
        // spoken turn at all — 47 of the 179 sessions here — and then the only
        // turns in hand are the ones the head brought, from the beginning of the
        // conversation.
        //
        // Read the tail on its own and let what it says win. Nothing is lost
        // when it says nothing: the head's turns stay, which is the best answer
        // available for a session whose last quarter-megabyte is all machinery.
        // The bytes were just read, so this second pass comes off the operating
        // system's cache rather than the disk.
        if let Ok(tail) = read_tail_utf8(&file, CODEX_SESSION_TAIL_BYTES) {
            let spoken = codex_turns_in(&tail);
            if !spoken.is_empty() {
                for record in parsed.iter_mut() {
                    record.latest_turns = spoken.clone();
                }
            }
        }

        codex_metadata.extend(with_log_path(parsed, &file));
    }
    records.extend(merge_codex_session_metadata(codex_records, codex_metadata));

    let cmux_term = home.join(".cmuxterm");
    for (agent, file) in cmux_hook_session_files(&cmux_term) {
        if let Ok(contents) = fs::read_to_string(file) {
            let mut parsed = parse_cmux_hook_sessions_json(&agent, &contents);
            if let Some(root) = project_root {
                parsed.retain(|record| record_matches_project(record, root, &mut project_roots));
            }
            records.extend(parsed);
        }
    }

    let claude_projects = home.join(".claude/projects");
    let mut files: Vec<(PathBuf, String)> = match project_root {
        Some(root) => claude_project_dirs_for_root(&claude_projects, root)
            .into_iter()
            .flat_map(|(directory, project_path)| {
                jsonl_files(&directory)
                    .into_iter()
                    .map(move |file| (file, project_path.clone()))
            })
            .collect(),
        None => jsonl_files(&claude_projects)
            .into_iter()
            .map(|file| {
                let project_path = file
                    .parent()
                    .and_then(|parent| parent.file_name())
                    .and_then(|name| name.to_str())
                    .and_then(decode_claude_project_dir)
                    .unwrap_or_default();
                (file, project_path)
            })
            .collect(),
    };
    // Drop subagent transcripts and helper runs BEFORE the file budget is
    // applied: they outnumber real sessions on a busy machine and would
    // otherwise evict them.
    files.retain(|(file, _)| !is_claude_subagent_transcript_path(file));
    files.retain(|(file, _)| !claude_transcript_file_is_agent_launched(file));
    files.sort_by(|(a, _), (b, _)| modified_time(b).cmp(&modified_time(a)));
    for (file, project_path) in files.into_iter().take(CLAUDE_SESSION_FILE_LIMIT) {
        if let Ok(contents) = read_tail_utf8(&file, CLAUDE_SESSION_TAIL_BYTES) {
            records.extend(with_log_path(
                parse_claude_jsonl(&contents, project_path.as_str()),
                &file,
            ));
        }
    }

    records = merge_agent_session_records(records);
    records.retain(session_said_something);
    if let Some(root) = project_root {
        // A record with no project identity belongs only in the unscoped
        // "Other" listing, never in a scoped project's results.
        records.retain(|record| record_matches_project(record, root, &mut project_roots));
    }
    records.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
    records.truncate(AGENT_SESSION_RESULT_LIMIT);
    records = with_derived_agent_session_metadata(records);
    records
}

/// Read independent rollout headers concurrently. A busy machine can hold
/// thousands of these files; doing 64 KB reads one after another made History
/// wait minutes before it could replace the lightweight rail rows.
fn codex_rollout_heads(
    files: Vec<PathBuf>,
) -> Vec<(PathBuf, Option<CodexThreadMarker>, Option<String>)> {
    if files.is_empty() {
        return Vec::new();
    }
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(4)
        .min(files.len());
    let chunk_size = files.len().div_ceil(workers);

    std::thread::scope(|scope| {
        let tasks = files
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|file| {
                            let (marker, cwd) = codex_rollout_head_meta(file);
                            (file.clone(), marker, cwd)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        tasks
            .into_iter()
            .flat_map(|task| task.join().unwrap_or_default())
            .collect()
    })
}

/// Whether a scanned session holds a conversation at all.
///
/// A window that was opened and closed still leaves a transcript behind, and it
/// reaches the panel as a row called "Codex session" or "Claude session" with no
/// count, no preview and nothing to expand. 56 of the 283 rows here were that:
/// 31 files one to three lines long, and 20 more holding only Claude's own
/// bookkeeping records with no reply ever written. They cannot be told apart
/// from each other, and resuming one opens an empty conversation.
///
/// Only a session whose transcript was actually read is judged. A provider whose
/// scanner does not collect turns — the cmux hook records carry a session list
/// and no transcript — has no `log_path`, and silence there is missing evidence
/// rather than an empty session.
fn session_said_something(record: &AgentSessionRecord) -> bool {
    record.log_path.is_none() || !record.latest_turns.is_empty() || record.message_count.is_some()
}

pub fn merge_agent_session_records(records: Vec<AgentSessionRecord>) -> Vec<AgentSessionRecord> {
    let mut merged = Vec::<AgentSessionRecord>::new();

    for record in records {
        if let Some(existing) = merged
            .iter_mut()
            .find(|candidate| candidate.provider == record.provider && candidate.id == record.id)
        {
            merge_agent_session_record(existing, record);
        } else {
            merged.push(record);
        }
    }

    merged
}

pub fn parse_codex_index_jsonl(input: &str) -> Vec<AgentSessionRecord> {
    input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|value| {
            let id = value.get("id")?.as_str()?.to_string();
            let title = value
                .get("thread_name")
                .and_then(Value::as_str)
                .unwrap_or(CODEX_UNTITLED_INDEX_TITLE)
                .to_string();
            let last_activity = value
                .get("updated_at")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);

            Some(AgentSessionRecord {
                provider: "codex".to_string(),
                id: id.clone(),
                title,
                description: None,
                model: model_from_value(&value),
                project_path: None,
                last_activity,
                resume_commands: vec![format!("codex resume {id}")],
                log_path: None,
                branch_hint: None,
                task_id: None,
                pull_request_hint: None,
                source_label: None,
                message_count: None,
                latest_turn_preview: None,
                latest_turns: Vec::new(),
                project_root: None,
            })
        })
        .collect()
}

/// The newest turn from each speaker in a stretch of a Codex transcript.
///
/// Separate from the parser above because that one only starts collecting once
/// it has seen the `session_meta` record, and that record is the first line of
/// the file. Handed the tail on its own it would find no session to attach
/// anything to and return nothing at all. This reads the turns and nothing else,
/// so it works on any slice of a transcript.
fn codex_turns_in(input: &str) -> Vec<AgentSessionTurn> {
    let mut pair = LatestTurnPair::default();
    for (position, value) in input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .enumerate()
    {
        if value.get("type").and_then(Value::as_str) != Some("response_item") {
            continue;
        }
        if let Some((speaker, said)) = codex_conversation_turn(&value) {
            pair.remember(
                speaker.turn_speaker(),
                position,
                agent_session_turn_text(&said),
            );
        }
    }

    pair.into_turns()
}

pub fn parse_codex_rollout_jsonl(input: &str) -> Vec<AgentSessionRecord> {
    let mut records: Vec<AgentSessionRecord> = Vec::new();
    let mut first_prompts: HashMap<String, String> = HashMap::new();
    let mut message_counts: HashMap<String, u32> = HashMap::new();
    let mut latest_turns: HashMap<String, String> = HashMap::new();
    let mut latest_turn_pairs: HashMap<String, LatestTurnPair> = HashMap::new();
    // Who spoke last in each session, so a run of agent narration can be
    // counted as the one thing the agent said rather than as twenty.
    let mut last_speakers: HashMap<String, CodexSpeaker> = HashMap::new();

    for (position, value) in input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .enumerate()
    {
        match value.get("type").and_then(Value::as_str) {
            Some("session_meta") => {
                let Some(payload) = value.get("payload") else {
                    continue;
                };
                if is_codex_background_thread(payload) {
                    return Vec::new();
                }
                let Some(id) = payload.get("id").and_then(Value::as_str) else {
                    continue;
                };

                let cwd = payload
                    .get("cwd")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(ToOwned::to_owned);
                let last_activity = value
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("timestamp").and_then(Value::as_str))
                    .map(ToOwned::to_owned);

                let record = AgentSessionRecord {
                    provider: "codex".to_string(),
                    id: id.to_string(),
                    title: CODEX_GENERIC_SESSION_TITLE.to_string(),
                    description: None,
                    model: model_from_value(payload),
                    project_path: cwd,
                    last_activity,
                    resume_commands: vec![format!("codex resume {id}")],
                    log_path: None,
                    branch_hint: None,
                    task_id: None,
                    pull_request_hint: None,
                    source_label: None,
                    message_count: None,
                    latest_turn_preview: None,
                    latest_turns: Vec::new(),
                    project_root: None,
                };

                if let Some(existing) = records.iter_mut().find(|candidate| {
                    candidate.provider == record.provider && candidate.id == record.id
                }) {
                    merge_codex_record(existing, record);
                } else {
                    records.push(record);
                }
            }
            Some("turn_context") => {
                let Some(payload) = value.get("payload") else {
                    continue;
                };

                let cwd = payload
                    .get("cwd")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(ToOwned::to_owned);
                let last_activity = value
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                update_latest_codex_record(
                    &mut records,
                    cwd,
                    None,
                    model_from_value(payload),
                    last_activity,
                );
            }
            Some("response_item") => {
                if let (Some(record), Some(prompt)) =
                    (records.last(), codex_user_prompt_text(&value))
                {
                    first_prompts.entry(record.id.clone()).or_insert(prompt);
                }

                if let Some(id) = records.last().map(|record| record.id.clone()) {
                    if let Some((speaker, said)) = codex_conversation_turn(&value) {
                        let turn = agent_session_turn_preview(speaker.preview_prefix(), &said);
                        latest_turn_pairs.entry(id.clone()).or_default().remember(
                            speaker.turn_speaker(),
                            position,
                            agent_session_turn_text(&said),
                        );
                        // Codex writes a separate record for every paragraph it
                        // narrates between tool calls, so a run of them is ONE
                        // thing the agent said back, not twenty. Without this a
                        // Codex row read "691 messages" where the Claude row
                        // beside it, for a longer conversation, read "33".
                        let repeat_narration = speaker == CodexSpeaker::Agent
                            && last_speakers.get(&id) == Some(&CodexSpeaker::Agent);
                        if !repeat_narration {
                            *message_counts.entry(id.clone()).or_insert(0) += 1;
                        }
                        last_speakers.insert(id.clone(), speaker);
                        latest_turns.insert(id, turn); // a later line is the newer turn
                    }
                }

                let cwd = codex_response_item_workdir(&value);
                let description = codex_response_item_description(&value);
                if cwd.is_none() && description.is_none() {
                    continue;
                };
                let last_activity = value
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                update_latest_codex_record(&mut records, cwd, description, None, last_activity);
            }
            _ => {}
        }
    }

    for record in records.iter_mut() {
        record.title = codex_display_title(
            &record.title,
            first_prompts.get(&record.id).map(String::as_str),
        );
        record.message_count = message_counts.get(&record.id).copied();
        record.latest_turn_preview = latest_turns.get(&record.id).cloned();
        record.latest_turns = latest_turn_pairs
            .remove(&record.id)
            .map(LatestTurnPair::into_turns)
            .unwrap_or_default();
    }

    records
}

/// Who said one message of a Codex conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexSpeaker {
    User,
    Agent,
}

/// One message of a Codex conversation: who said it, and how a row would show
/// it. The user side uses the same filter the title does, so the opening turns
/// Codex writes for itself — the repository instructions, the environment block
/// — are neither counted nor shown; the agent side needs actual words.
/// Who spoke on one transcript line and what they said, as written.
///
/// The text comes back raw. Two things want it at different lengths — the row's
/// one-line preview and the expanded card's scrollable block — and cutting it
/// here would mean the longer of the two could never be built.
fn codex_conversation_turn(value: &Value) -> Option<(CodexSpeaker, String)> {
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }

    match payload.get("role").and_then(Value::as_str)? {
        "user" => Some((CodexSpeaker::User, codex_typed_user_text(value)?)),
        "assistant" => {
            let text = payload.get("content").and_then(value_to_text)?;
            let text = text.trim();
            (!text.is_empty()).then(|| (CodexSpeaker::Agent, text.to_string()))
        }
        _ => None,
    }
}

impl CodexSpeaker {
    fn turn_speaker(self) -> TurnSpeaker {
        match self {
            Self::User => TurnSpeaker::User,
            Self::Agent => TurnSpeaker::Agent,
        }
    }

    fn preview_prefix(self) -> &'static str {
        match self {
            Self::User => AGENT_SESSION_USER_TURN_PREFIX,
            Self::Agent => AGENT_SESSION_AGENT_TURN_PREFIX,
        }
    }
}

/// Whether a rollout file is a thread nobody sat down and opened.
///
/// Two kinds qualify, and the opening `session_meta` record names both.
///
/// The first is a helper Codex spawned for itself, which carries
/// `thread_source: "subagent"` and/or a `source` object whose only key is
/// `subagent`. Verified 2026-07-28 across 687 rollout files on this machine:
/// 619 helper threads, 68 sessions the user started, and the two markers
/// disagreed on a single file — so both are checked and either one is enough.
///
/// The second is a run some program made through `codex exec`, which carries
/// `originator: "codex_exec"`. These were kept until now, on the reading that
/// anything not marked `subagent` was the user's. They are not: sampled across
/// 400 recent rollouts on 2026-08-17, 391 were `codex_exec` against 2 from the
/// interactive `codex-tui` and 7 from this app, and the history list stood at
/// 956 rows of a pipeline calling Codex in a loop. Their working directory is a
/// scratch folder, so each arrived named after it.
///
/// The third is a run driven through another program rather than typed into,
/// which `source` names outright: `"mcp"` for a thread opened over the Codex MCP
/// server, `"exec"` for one launched by the non-interactive command. Neither
/// carries `originator: "codex_exec"`, so both slipped through and arrived in
/// History looking like main threads — 35 of them on this machine on
/// 2026-08-17, every one titled "Codex Companion Task: Read and fully execute
/// the task described in the spec file at …", which is a dispatch, not a
/// conversation. The interactive values (`"cli"`, `"vscode"`) are untouched.
///
/// A file with neither marker is kept. Older Codex versions predate the fields
/// (15 files here), and losing one of the user's sessions is worse than listing
/// a thread they did not open.
fn is_codex_background_thread(payload: &Value) -> bool {
    if payload.get("thread_source").and_then(Value::as_str) == Some("subagent") {
        return true;
    }

    if payload.get("originator").and_then(Value::as_str) == Some("codex_exec") {
        return true;
    }

    // A Codex thread another coding agent opened to do its own work. It reaches
    // the disk looking interactive — `source: "vscode"`, no subagent marker —
    // and only `originator` gives it away. The 12 here were all titled "Codex
    // Companion Task: …". The other editors that drive Codex are left alone:
    // `t3code_desktop` and this app's own bridge are the user at a keyboard.
    if payload.get("originator").and_then(Value::as_str) == Some("Claude Code") {
        return true;
    }

    let source = payload.get("source");

    if matches!(source.and_then(Value::as_str), Some("mcp") | Some("exec")) {
        return true;
    }

    source
        .and_then(Value::as_object)
        .is_some_and(|source| source.contains_key("subagent"))
}

/// What a rollout file's opening metadata record says about its thread. The id
/// can be missing while the verdict is still known, so the two are separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexThreadMarker {
    pub id: Option<String>,
    pub spawned_by_codex: bool,
}

/// Read from a bounded head, so the scan can sort the files before it spends
/// its read budget on them.
fn codex_rollout_thread_marker(path: &Path) -> Option<CodexThreadMarker> {
    codex_rollout_head_meta(path).0
}

fn codex_rollout_head_meta(path: &Path) -> (Option<CodexThreadMarker>, Option<String>) {
    let Ok(head) = read_head_utf8(path, CODEX_SESSION_META_PROBE_BYTES) else {
        return (None, None);
    };
    codex_rollout_head_metadata(&head)
}

/// `None` when the head holds no readable metadata record — a truncated line, a
/// file that opens with something else, an empty file. The caller keeps those.
pub fn codex_rollout_head_thread_marker(head: &str) -> Option<CodexThreadMarker> {
    codex_rollout_head_metadata(head).0
}

fn codex_rollout_head_metadata(head: &str) -> (Option<CodexThreadMarker>, Option<String>) {
    let Some(line) = head.lines().next() else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return (None, None);
    };
    if value.get("type").and_then(Value::as_str) != Some("session_meta") {
        return (None, None);
    }

    let Some(payload) = value.get("payload") else {
        return (None, None);
    };
    let marker = CodexThreadMarker {
        id: payload
            .get("id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        spawned_by_codex: is_codex_background_thread(payload),
    };
    let cwd = payload
        .get("cwd")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    (Some(marker), cwd)
}

fn codex_cwd_matches_project(
    cwd: Option<&str>,
    project_root: &str,
    roots: &mut HashMap<String, Option<String>>,
) -> bool {
    let Some(cwd) = cwd else {
        return true;
    };

    let resolved = roots
        .entry(cwd.to_string())
        .or_insert_with(|| {
            git_project_root(Path::new(cwd)).map(|root| root.to_string_lossy().into_owned())
        })
        .as_deref()
        .unwrap_or(cwd);
    resolved == project_root
}

fn record_matches_project(
    record: &AgentSessionRecord,
    project_root: &str,
    roots: &mut HashMap<String, Option<String>>,
) -> bool {
    record
        .project_path
        .as_deref()
        .is_some_and(|path| codex_cwd_matches_project(Some(path), project_root, roots))
}

pub fn codex_rollout_head_is_subagent_thread(head: &str) -> bool {
    codex_rollout_head_thread_marker(head).is_some_and(|marker| marker.spawned_by_codex)
}

/// Index rows naming threads Codex spawned for itself.
///
/// `~/.codex/session_index.jsonl` is a flat list of id, name and timestamp with
/// nothing in it saying what kind of thread a row describes, so the rollout
/// files are the only evidence — and the index is read whole, with none of the
/// filtering the rollout files get. On this machine 119 of its 330 rows are
/// sub-agent threads, 117 of them already archived, and every one of those was
/// reaching the rail under a plausible name ("Audit inventory gaps", "Review
/// mobile steppers") that gave the user no way to tell it apart from their own
/// work.
///
/// A row whose thread has no rollout file left anywhere is kept: 150 of them
/// here, all genuinely the user's, and absence of evidence is not evidence.
pub fn drop_codex_subagent_sessions(
    records: Vec<AgentSessionRecord>,
    subagent_ids: &HashSet<String>,
) -> Vec<AgentSessionRecord> {
    records
        .into_iter()
        .filter(|record| !subagent_ids.contains(&record.id))
        .collect()
}

/// The first thing the user actually typed. Codex opens every session with
/// machine-written turns sent as the user — the repository's `AGENTS.md`, the
/// environment block, the plugin list, the file list — and each has a shape we
/// can recognise, so the title comes from the first turn that is none of them.
fn codex_user_prompt_text(value: &Value) -> Option<String> {
    codex_typed_user_text(value).map(|text| compact_text(&text, 140))
}

/// The same turn, at full length, for callers that do their own cutting.
fn codex_typed_user_text(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    if payload.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }

    let text = payload.get("content").and_then(value_to_text)?;
    let text = text.trim();
    if text.is_empty() || CODEX_INJECTED_PROMPT_PREFIXES.iter().any(|prefix| text.starts_with(prefix)) {
        return None;
    }

    Some(text.to_string())
}

/// Openings that mean Codex wrote this turn, not the user. `<` covers every
/// tagged block it injects (`<environment_context>`, `<recommended_plugins>`,
/// `<user_shell_command>`); the two headings are the repository instructions
/// and the attached-file list.
const CODEX_INJECTED_PROMPT_PREFIXES: &[&str] = &[
    "<",
    "# AGENTS.md instructions",
    "# Files mentioned by the user",
];

/// A Codex rollout file carries no title of its own, so a rail built from these
/// files alone reads as hundreds of rows all saying "Codex session". The first
/// typed prompt is the only description of the session in the file, so it
/// becomes the title; the generic label survives only when the scanned window
/// held no typed prompt at all.
fn codex_display_title(derived: &str, first_prompt: Option<&str>) -> String {
    if derived != CODEX_GENERIC_SESSION_TITLE {
        return derived.to_string();
    }

    match first_prompt {
        Some(prompt) => compact_text(prompt, 60),
        None => CODEX_GENERIC_SESSION_TITLE.to_string(),
    }
}

fn is_generic_codex_title(title: &str) -> bool {
    title == CODEX_GENERIC_SESSION_TITLE || title == CODEX_UNTITLED_INDEX_TITLE
}

/// One Codex session can be described twice: the index file may name it, and
/// the rollout file offers the opening prompt. Whichever record looks newer is
/// beside the point — a placeholder must never displace a real title, and the
/// index name describes the whole session where the prompt only opens it, so
/// the title already in hand wins when both are real.
fn better_codex_title(existing: String, candidate: String) -> String {
    if !is_generic_codex_title(&existing) {
        return existing;
    }
    if !is_generic_codex_title(&candidate) {
        return candidate;
    }

    existing
}

fn update_latest_codex_record(
    records: &mut [AgentSessionRecord],
    cwd: Option<String>,
    description: Option<String>,
    model: Option<String>,
    last_activity: Option<String>,
) {
    let Some(record) = records.last_mut() else {
        return;
    };

    let id = record.id.clone();
    let update = AgentSessionRecord {
        provider: "codex".to_string(),
        id: id.clone(),
        title: record.title.clone(),
        description,
        model,
        project_path: cwd,
        last_activity,
        resume_commands: vec![format!("codex resume {id}")],
        log_path: None,
        branch_hint: None,
        task_id: None,
        pull_request_hint: None,
        source_label: None,
        message_count: None,
        latest_turn_preview: None,
        latest_turns: Vec::new(),
        project_root: None,
    };
    merge_codex_record(record, update);
}

fn codex_response_item_workdir(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("function_call") {
        return None;
    }

    let arguments = payload.get("arguments").and_then(Value::as_str)?;
    let arguments = serde_json::from_str::<Value>(arguments).ok()?;
    optional_string(arguments.get("workdir"))
}

fn codex_response_item_description(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    if payload.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }

    payload
        .get("content")
        .and_then(value_to_text)
        .map(|text| compact_text(&text, 140))
}

pub fn merge_codex_session_metadata(
    mut indexed: Vec<AgentSessionRecord>,
    metadata: Vec<AgentSessionRecord>,
) -> Vec<AgentSessionRecord> {
    for record in metadata {
        if let Some(existing) = indexed
            .iter_mut()
            .find(|candidate| candidate.provider == record.provider && candidate.id == record.id)
        {
            merge_codex_record(existing, record);
        } else {
            indexed.push(record);
        }
    }

    indexed
}

pub fn parse_cmux_hook_sessions_json(agent: &str, input: &str) -> Vec<AgentSessionRecord> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Vec::new();
    };
    let Some(sessions) = value.get("sessions").and_then(Value::as_object) else {
        return Vec::new();
    };

    let agent = agent.trim().to_lowercase();
    if agent.is_empty() {
        return Vec::new();
    }

    sessions
        .iter()
        .filter_map(|(key, value)| {
            let id = value
                .get("sessionId")
                .and_then(Value::as_str)
                .unwrap_or(key)
                .trim();
            if id.is_empty() {
                return None;
            }

            let cwd = value
                .get("cwd")
                .and_then(Value::as_str)
                .or_else(|| {
                    value
                        .get("launchCommand")
                        .and_then(|launch| launch.get("workingDirectory"))
                        .and_then(Value::as_str)
                })
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned);
            let last_activity = timestampish_string(value.get("updatedAt"))
                .or_else(|| timestampish_string(value.get("startedAt")))
                .or_else(|| {
                    timestampish_string(
                        value
                            .get("launchCommand")
                            .and_then(|launch| launch.get("capturedAt")),
                    )
                });
            let status = value
                .get("runtimeStatus")
                .and_then(Value::as_str)
                .or_else(|| value.get("agentLifecycle").and_then(Value::as_str))
                .filter(|value| !value.trim().is_empty());
            let title = cmux_session_title(&agent, value, status, cwd.as_deref());

            Some(AgentSessionRecord {
                provider: format!("cmux-{agent}"),
                id: id.to_string(),
                title,
                description: cmux_session_description(value),
                model: model_from_value(value),
                project_path: cwd.clone(),
                last_activity,
                resume_commands: cmux_resume_commands(&agent, id, cwd.as_deref()),
                log_path: None,
                branch_hint: None,
                task_id: None,
                pull_request_hint: None,
                source_label: None,
                message_count: None,
                latest_turn_preview: None,
                latest_turns: Vec::new(),
                project_root: None,
            })
        })
        .collect()
}

pub fn parse_claude_jsonl(input: &str, project_path: &str) -> Vec<AgentSessionRecord> {
    let mut records: Vec<AgentSessionRecord> = Vec::new();
    let mut ai_titles: HashMap<String, String> = HashMap::new();
    let mut first_prompts: HashMap<String, String> = HashMap::new();
    let mut message_counts: HashMap<String, u32> = HashMap::new();
    let mut latest_turns: HashMap<String, String> = HashMap::new();
    let mut latest_turn_pairs: HashMap<String, LatestTurnPair> = HashMap::new();

    for (position, value) in input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .enumerate()
    {
        // One such entry condemns the whole transcript: these files are what
        // they are end to end, so anything already collected from one is an
        // agent's turn, not a session the user can resume. Both checks answer
        // the same question — "is this a top-level session?" — from different
        // evidence: a subagent's sidechain flag, or a helper's entrypoint.
        if is_claude_sidechain_entry(&value) || is_claude_agent_launched_entry(&value) {
            return Vec::new();
        }

        if let Some((session, title)) = claude_ai_title(&value) {
            ai_titles.insert(session, title); // a later line carries the newer title
        }
        if let Some((session, prompt)) = claude_user_prompt_text(&value) {
            first_prompts.entry(session).or_insert(prompt);
        }
        if let Some((session, speaker, said)) = claude_conversation_turn(&value) {
            *message_counts.entry(session.clone()).or_insert(0) += 1;
            latest_turn_pairs.entry(session.clone()).or_default().remember(
                speaker,
                position,
                agent_session_turn_text(&said),
            );
            // a later line is the newer turn
            latest_turns.insert(
                session,
                agent_session_turn_preview(speaker.preview_prefix(), &said),
            );
        }

        let Some(id) = value
            .get("sessionId")
            .or_else(|| value.get("session_id"))
            .and_then(Value::as_str)
        else {
            continue;
        };

        let timestamp = value
            .get("timestamp")
            .or_else(|| value.get("created_at"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        let cwd = value
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or(project_path)
            .to_string();

        let title = title_from_claude_message(&value)
            .unwrap_or_else(|| CLAUDE_GENERIC_SESSION_TITLE.to_string());
        let record = AgentSessionRecord {
            provider: "claude".to_string(),
            id: id.to_string(),
            title,
            description: claude_session_description(&value),
            model: value
                .get("message")
                .and_then(model_from_value)
                .or_else(|| model_from_value(&value)),
            project_path: (!cwd.is_empty()).then_some(cwd.clone()),
            last_activity: timestamp,
            resume_commands: vec![
                format!("claude --resume {id}"),
                format!("cd {} && claude --resume {id}", shell_quote(&cwd)),
            ],
            log_path: None,
            branch_hint: None,
            task_id: None,
            pull_request_hint: None,
            source_label: None,
            message_count: None,
            latest_turn_preview: None,
            latest_turns: Vec::new(),
            project_root: None,
        };

        if let Some(existing) = records
            .iter_mut()
            .find(|candidate| candidate.provider == record.provider && candidate.id == record.id)
        {
            merge_agent_session_record(existing, record);
        } else {
            records.push(record);
        }
    }

    for record in records.iter_mut() {
        record.title = claude_display_title(
            &record.title,
            ai_titles.get(&record.id).map(String::as_str),
            first_prompts.get(&record.id).map(String::as_str),
            record.project_path.as_deref(),
        );
        record.message_count = message_counts.get(&record.id).copied();
        record.latest_turn_preview = latest_turns.get(&record.id).cloned();
        record.latest_turns = latest_turn_pairs
            .remove(&record.id)
            .map(LatestTurnPair::into_turns)
            .unwrap_or_default();
    }

    records
}

/// One turn of the conversation as a row counts it, and how that row would show
/// it. Tool calls and tool results are `type: "user"` and `type: "assistant"`
/// records too, so both sides insist on actual words: a user turn goes through
/// the same filter the title uses (no slash-command wrappers, no resume
/// caveat), and an assistant turn needs at least one text block.
/// Which session a transcript line belongs to, who spoke on it, and what they
/// said — raw, for the same reason the Codex side keeps it raw: the row preview
/// and the expanded card want different lengths of the same sentence.
fn claude_conversation_turn(value: &Value) -> Option<(String, TurnSpeaker, String)> {
    match optional_string(value.get("type"))?.as_str() {
        "user" => {
            let (id, text) = claude_user_prompt_text(value)?;
            Some((id, TurnSpeaker::User, text))
        }
        "assistant" => {
            let id = optional_string(value.get("sessionId"))?;
            let text = claude_assistant_text(value)?;
            Some((id, TurnSpeaker::Agent, text))
        }
        _ => None,
    }
}

impl TurnSpeaker {
    fn preview_prefix(self) -> &'static str {
        match self {
            Self::User => AGENT_SESSION_USER_TURN_PREFIX,
            Self::Agent => AGENT_SESSION_AGENT_TURN_PREFIX,
        }
    }
}

/// What the agent said in words on one transcript line. An assistant record
/// whose content is nothing but `tool_use` blocks said nothing, and yields
/// `None`.
fn claude_assistant_text(value: &Value) -> Option<String> {
    let content = value.get("message")?.get("content")?;
    let text = match content {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter(|item| optional_string(item.get("type")).as_deref() == Some("text"))
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => return None,
    };

    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// A Claude Code SUBAGENT transcript is stored as its own `.jsonl` and is never
/// resumable — `claude --resume <id>` on one is meaningless — so it must never
/// reach the rail. Two independent discriminators, because each covers what the
/// other cannot: the path is free and also keeps subagents out of the scan's
/// file budget, the content is authoritative and still catches a flat layout.
///
/// Today Claude Code writes them under `<project>/<session>/subagents/`.
fn is_claude_subagent_transcript_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "subagents")
}

/// Every entry of a subagent transcript carries `"isSidechain": true`; every
/// entry of a real session carries `false`. Verified across 2180 transcripts on
/// a live machine: 1177 sidechain files, all of them pure, zero mixed files.
fn is_claude_sidechain_entry(value: &Value) -> bool {
    value.get("isSidechain").and_then(Value::as_bool) == Some(true)
}

/// Entrypoint values that mean "a program started this run, not the user".
/// Prefixes, so a future `sdk-node` is covered without a code change; adding a
/// new family is a one-line edit here.
const AGENT_LAUNCH_ENTRYPOINT_PREFIXES: &[&str] = &["sdk"];

/// Helper-agent transcripts — a team lead's dispatched teammates, and any other
/// SDK-driven run — land flat in the same project directory as the user's own
/// sessions, with `isSidechain: false`, `userType: "external"` and no agent name
/// anywhere, so neither discriminator above sees them. What they do carry is how
/// they were launched: every `user`, `assistant` and `attachment` record repeats
/// an `entrypoint`, and a programmatic run is always `sdk-…` (`sdk-cli`,
/// `sdk-py`) where a session the user typed into is `cli` or `claude-vscode`.
///
/// Verified 2026-07-28 across 1073 transcripts on this machine: every one of the
/// 888 dispatched helper runs was `sdk-…`, every human session was `cli` or
/// `claude-vscode`, and no transcript ever mixed the two families. 1072 of the
/// 1073 carry the field within the 256 KB tail this scanner reads.
///
/// This deliberately replaces the "does the first message read like a dispatch
/// prompt?" idea: the user's real sessions often open with pasted logs and
/// instruction-shaped text, and the scanner reads a tail window that usually
/// does not even contain the first message.
fn is_agent_launch_entrypoint(entrypoint: &str) -> bool {
    AGENT_LAUNCH_ENTRYPOINT_PREFIXES
        .iter()
        .any(|prefix| entrypoint.starts_with(prefix))
}

/// Reads that entrypoint off one transcript line. A line without the field says
/// nothing either way — older transcripts predate it — so it is not evidence of
/// a helper and the transcript is kept.
fn is_claude_agent_launched_entry(value: &Value) -> bool {
    value
        .get("entrypoint")
        .and_then(Value::as_str)
        .is_some_and(is_agent_launch_entrypoint)
}

/// The same question asked of a file instead of a parsed record, so the scan can
/// skip helper runs before it spends its read budget on them.
///
/// Excluding them at parse time was never enough. A batch of API work can write
/// hundreds of transcripts in an afternoon — 880 of the 1091 here came from one
/// document-extraction job, all in a temporary directory — and the scan reads
/// only the newest 512 files. Sorted by modification time, that batch pushed the
/// user's own sessions out of the scan entirely: 6 of 24 survived. Nothing was
/// wrong with the rule; it just ran too late to matter.
///
/// Reads a bounded head rather than the whole file. Anything unreadable, or
/// whose entrypoint sits past the probe, is kept and settled when the file is
/// parsed in full — the same direction to fail as every other check here.
fn claude_transcript_file_is_agent_launched(path: &Path) -> bool {
    let Ok(head) = read_head_utf8(path, CLAUDE_SESSION_LAUNCH_PROBE_BYTES) else {
        return false;
    };

    claude_transcript_head_is_agent_launched(&head)
}

pub fn claude_transcript_head_is_agent_launched(head: &str) -> bool {
    head.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .any(|value| is_claude_agent_launched_entry(&value))
}

/// Claude Code records its own generated session title on a `type: "ai-title"`
/// line. It is the best title available — it describes the whole session rather
/// than whichever message happened to be last — so it wins outright.
fn claude_ai_title(value: &Value) -> Option<(String, String)> {
    if optional_string(value.get("type"))? != "ai-title" {
        return None;
    }

    Some((
        optional_string(value.get("sessionId"))?,
        optional_string(value.get("aiTitle"))?,
    ))
}

/// Openings that mean Claude Code wrote this "user" message, not the user. `<`
/// covers every tagged block it injects (`<command-name>`,
/// `<local-command-caveat>`); the resume caveat and the interruption notice are
/// the two it writes as plain sentences.
///
/// The interruption matters more than it looks. Pressing Escape in the middle
/// of an answer is how a session usually ends, so it is disproportionately
/// likely to be the LAST thing in a transcript — and the row shows the last
/// thing said. In 617 real user turns sampled on this machine, 50 of them were
/// exactly "[Request interrupted by user]".
const CLAUDE_INJECTED_PROMPT_PREFIXES: &[&str] =
    &["<", "Caveat:", "[Request interrupted"];

/// The first genuine user prompt in the scanned window. Tool results are
/// `type: "user"` too, so plain `content` text is required and `tool_result`
/// items are dropped; anything Claude Code wrote for itself is skipped because
/// none of it says what the session is about.
fn claude_user_prompt_text(value: &Value) -> Option<(String, String)> {
    if optional_string(value.get("type"))? != "user" {
        return None;
    }
    if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return None;
    }

    let id = optional_string(value.get("sessionId"))?;
    let text = match value.get("message")?.get("content")? {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter(|item| optional_string(item.get("type")).as_deref() == Some("text"))
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => return None,
    };

    let text = text.trim();
    if text.is_empty()
        || CLAUDE_INJECTED_PROMPT_PREFIXES
            .iter()
            .any(|prefix| text.starts_with(prefix))
    {
        return None;
    }

    Some((id, text.to_string()))
}

/// Title preference for a Claude row: the session's own AI title, then whatever
/// a message yielded, then `<project folder> — <first user prompt>`. The generic
/// label survives only when nothing else exists — a rail full of "Claude
/// session" rows tells the user nothing about which session to resume.
fn claude_display_title(
    derived: &str,
    ai_title: Option<&str>,
    first_prompt: Option<&str>,
    project_path: Option<&str>,
) -> String {
    if let Some(title) = ai_title {
        return compact_text(title, 80);
    }
    if derived != CLAUDE_GENERIC_SESSION_TITLE {
        return derived.to_string();
    }

    match (first_prompt, project_path.and_then(path_display_name)) {
        (Some(prompt), Some(folder)) => compact_text(&format!("{folder} — {prompt}"), 60),
        (Some(prompt), None) => compact_text(prompt, 60),
        (None, _) => CLAUDE_GENERIC_SESSION_TITLE.to_string(),
    }
}

fn cmux_resume_commands(agent: &str, id: &str, cwd: Option<&str>) -> Vec<String> {
    let command = match agent {
        "codex" => format!("codex resume {id}"),
        "claude" => format!("claude --resume {id}"),
        "gemini" => format!("gemini --resume {id}"),
        "opencode" => format!("opencode --session {id}"),
        "omp" | "pi" => format!("{agent} --session {id}"),
        "amp" => format!("amp threads continue {id}"),
        "antigravity" | "agy" => format!("agy --conversation {id}"),
        "rovo" | "acli" => format!("acli rovodev run --restore {id}"),
        "cursor" | "cursor-agent" => format!("cursor-agent --resume {id}"),
        other => format!("{other} --resume {id}"),
    };

    match cwd.filter(|value| !value.trim().is_empty()) {
        Some(cwd) => vec![
            command.clone(),
            format!("cd {} && {command}", shell_quote(cwd)),
        ],
        None => vec![command],
    }
}

fn merge_codex_record(existing: &mut AgentSessionRecord, candidate: AgentSessionRecord) {
    let previous_title = existing.title.clone();
    let candidate_title = candidate.title.clone();
    merge_agent_session_record(existing, candidate);
    existing.title = better_codex_title(previous_title, candidate_title);
}

fn merge_agent_session_record(existing: &mut AgentSessionRecord, candidate: AgentSessionRecord) {
    if existing.description.is_none() {
        existing.description = candidate.description.clone();
    }

    if existing.model.is_none() {
        existing.model = candidate.model.clone();
    }

    if existing.project_path.is_none() {
        existing.project_path = candidate.project_path.clone();
    }

    // The first file to describe a session is the one the app offers to open,
    // even when a later file has more to say about the session itself. Swapping
    // it would move the transcript out from under a reader mid-scan for no gain.
    if existing.log_path.is_none() {
        existing.log_path = candidate.log_path.clone();
    }

    // The fuller read wins, whichever record it came from. One session can be
    // described by more than one file, and each count is a floor — the bigger
    // floor is the one closer to the truth. (`None` sorts below every `Some`,
    // so a record that counted nothing never overwrites one that did.)
    existing.message_count = existing.message_count.max(candidate.message_count);

    if existing.latest_turn_preview.is_none() {
        existing.latest_turn_preview = candidate.latest_turn_preview.clone();
    }

    if existing.latest_turns.is_empty() {
        existing.latest_turns = candidate.latest_turns.clone();
    }

    let candidate_is_newer = candidate
        .last_activity
        .as_ref()
        .is_some_and(|candidate_activity| {
            existing
                .last_activity
                .as_ref()
                .map_or(true, |existing_activity| {
                    candidate_activity > existing_activity
                })
        });

    if candidate_is_newer {
        existing.title = candidate.title;
        existing.description = candidate.description.or(existing.description.take());
        existing.model = candidate.model.or(existing.model.take());
        existing.project_path = candidate.project_path.or(existing.project_path.take());
        existing.last_activity = candidate.last_activity;
        existing.latest_turn_preview = candidate
            .latest_turn_preview
            .or(existing.latest_turn_preview.take());
        if !candidate.latest_turns.is_empty() {
            existing.latest_turns = candidate.latest_turns;
        }
    }

    for command in candidate.resume_commands {
        if !existing.resume_commands.contains(&command) {
            existing.resume_commands.push(command);
        }
    }
}

fn model_from_value(value: &Value) -> Option<String> {
    optional_string(value.get("model"))
        .or_else(|| optional_string(value.get("model_slug")))
        .or_else(|| optional_string(value.get("modelSlug")))
        .or_else(|| optional_string(value.get("modelName")))
        .or_else(|| optional_string(value.get("model_name")))
        .or_else(|| optional_string(value.get("modelId")))
        .or_else(|| optional_string(value.get("model_id")))
}

fn cmux_session_title(
    agent: &str,
    value: &Value,
    status: Option<&str>,
    cwd: Option<&str>,
) -> String {
    let agent_label = agent_display_label(agent);
    let detail = optional_string(value.get("title"))
        .or_else(|| optional_string(value.get("name")))
        .or_else(|| optional_string(value.get("threadName")))
        .or_else(|| optional_string(value.get("conversationTitle")))
        .or_else(|| optional_string(value.get("taskTitle")))
        .or_else(|| optional_string(value.get("lastSubtitle")))
        .or_else(|| {
            status
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .or_else(|| cwd.and_then(path_display_name));

    match detail {
        Some(detail) => format!("{agent_label} · {}", compact_text(&detail, 72)),
        None => format!("{agent_label} session"),
    }
}

fn cmux_session_description(value: &Value) -> Option<String> {
    optional_string(value.get("lastBody"))
        .or_else(|| optional_string(value.get("lastMessage")))
        .or_else(|| optional_string(value.get("summary")))
        .map(|text| compact_text(&text, 140))
}

fn claude_session_description(value: &Value) -> Option<String> {
    value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(value_to_text)
        .or_else(|| value.get("summary").and_then(value_to_text))
        .map(|text| compact_text(&text, 140))
}

fn agent_display_label(agent: &str) -> String {
    match agent {
        "codex" => "Codex".to_string(),
        "claude" => "Claude".to_string(),
        "gemini" => "Gemini".to_string(),
        "opencode" => "OpenCode".to_string(),
        "cursor" | "cursor-agent" => "Cursor".to_string(),
        "antigravity" | "agy" => "Antigravity".to_string(),
        "rovo" | "acli" => "Rovo".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => "Agent".to_string(),
            }
        }
    }
}

fn compact_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.chars().count() <= max_chars {
        return trimmed;
    }

    let mut compacted = trimmed
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    compacted.push('…');
    compacted
}

fn path_display_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn first_agent_session_hint(
    record: &AgentSessionRecord,
    mut derive: impl FnMut(&str) -> Option<String>,
) -> Option<String> {
    derive(&record.title)
        .or_else(|| record.project_path.as_deref().and_then(&mut derive))
        .or_else(|| {
            record
                .resume_commands
                .iter()
                .find_map(|command| derive(command))
        })
}

fn agent_session_source_label(record: &AgentSessionRecord) -> String {
    let provider = agent_session_provider_label(&record.provider);
    match record
        .project_path
        .as_deref()
        .and_then(path_display_name)
        .or_else(|| {
            let title = record.title.trim();
            (!title.is_empty()).then(|| compact_text(title, 48))
        }) {
        Some(detail) => format!("{provider} · {detail}"),
        None => provider,
    }
}

fn agent_session_provider_label(provider: &str) -> String {
    let provider = provider.trim();
    if let Some(agent) = provider.strip_prefix("cmux-") {
        return format!("CMUX {}", agent_display_label(agent));
    }
    agent_display_label(provider)
}

fn branch_hint_from_text(text: &str) -> Option<String> {
    let lower_text = text.to_ascii_lowercase();
    for marker in ["branch:", "branch=", "branch "] {
        let mut search_start = 0;
        while let Some(offset) = lower_text[search_start..].find(marker) {
            let index = search_start + offset;
            if index > 0 {
                let previous = lower_text.as_bytes()[index - 1] as char;
                if previous.is_ascii_alphanumeric() {
                    search_start = index + marker.len();
                    continue;
                }
            }

            let suffix = &text[index + marker.len()..];
            if let Some(branch) = git_ref_token_from_text(suffix) {
                return Some(branch);
            }
            search_start = index + marker.len();
        }
    }

    None
}

fn git_ref_token_from_text(text: &str) -> Option<String> {
    let token: String = text
        .trim_start_matches(|character: char| {
            character.is_whitespace() || matches!(character, '`' | '"' | '\'')
        })
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '/')
        })
        .collect();
    if token.is_empty()
        || token
            .chars()
            .all(|character| matches!(character, '.' | '_' | '-' | '/'))
    {
        return None;
    }

    Some(token)
}

fn task_id_from_text(text: &str) -> Option<String> {
    let lower_text = text.to_ascii_lowercase();
    for (index, _) in lower_text.match_indices("tsk") {
        if index > 0 {
            let previous = lower_text.as_bytes()[index - 1] as char;
            if previous.is_ascii_alphanumeric() {
                continue;
            }
        }

        let suffix =
            lower_text[index + 3..].trim_start_matches(['-', '_', '/', '#', '[', ' ', ':']);
        let digits: String = suffix
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return Some(format!("TSK-{digits}"));
        }
    }

    None
}

fn pull_request_hint_from_text(text: &str) -> Option<String> {
    pull_request_number_after_marker(text, "pull request")
        .or_else(|| pull_request_number_after_marker(text, "pr"))
        .or_else(|| {
            link_hint_from_text(text)
                .as_deref()
                .and_then(github_pull_request_number_from_url)
        })
        .map(|number| format!("PR #{number}"))
}

fn pull_request_number_after_marker(text: &str, marker: &str) -> Option<String> {
    let lower_text = text.to_ascii_lowercase();
    let mut search_start = 0;
    while let Some(offset) = lower_text[search_start..].find(marker) {
        let index = search_start + offset;
        if index > 0 {
            let previous = lower_text.as_bytes()[index - 1] as char;
            if previous.is_ascii_alphanumeric() {
                search_start = index + marker.len();
                continue;
            }
        }

        let suffix = &text[index + marker.len()..];
        let suffix = suffix.trim_start_matches([' ', '#', '-', ':']);
        let digits: String = suffix
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return Some(digits);
        }

        search_start = index + marker.len();
    }

    None
}

fn link_hint_from_text(text: &str) -> Option<String> {
    for scheme in ["https://", "http://"] {
        let mut search_start = 0;
        while let Some(offset) = text[search_start..].find(scheme) {
            let index = search_start + offset;
            let suffix = &text[index..];
            let end = suffix
                .char_indices()
                .find_map(|(index, character)| character.is_whitespace().then_some(index))
                .unwrap_or(suffix.len());
            let link = suffix[..end].trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '"']);
            if !link.is_empty() {
                return Some(link.to_string());
            }

            search_start = index + scheme.len();
        }
    }

    None
}

fn github_pull_request_number_from_url(url: &str) -> Option<String> {
    if !url.to_ascii_lowercase().contains("github.com/") {
        return None;
    }

    let lower_url = url.to_ascii_lowercase();
    let index = lower_url.find("/pull/")? + "/pull/".len();
    let digits: String = url[index..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    (!digits.is_empty()).then_some(digits)
}

fn optional_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn timestampish_string(value: Option<&Value>) -> Option<String> {
    let value = value?;
    match value {
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else if let Ok(number) = trimmed.parse::<f64>() {
                unix_timestamp_number_to_iso(number).or_else(|| Some(trimmed.to_string()))
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(number) => number.as_f64().and_then(unix_timestamp_number_to_iso),
        _ => None,
    }
}

fn unix_timestamp_number_to_iso(value: f64) -> Option<String> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }

    let seconds_value = if value >= 1_000_000_000_000.0 {
        value / 1000.0
    } else {
        value
    };
    let mut seconds = seconds_value.floor() as i64;
    let mut millis = ((seconds_value - seconds as f64) * 1000.0).floor() as u32;
    if millis >= 1000 {
        seconds += 1;
        millis = 0;
    }
    Some(unix_seconds_to_iso8601(seconds, millis))
}

fn unix_seconds_to_iso8601(seconds: i64, millis: u32) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    if millis > 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    }
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year, month as u32, day as u32)
}

fn title_from_claude_message(value: &Value) -> Option<String> {
    let content = value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(value_to_text)
        .or_else(|| value.get("summary").and_then(value_to_text))?;

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.chars().take(80).collect())
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Array(values) => values
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("content").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join(" ")
            .into(),
        _ => None,
    }
}

fn jsonl_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(jsonl_files(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }

    files
}

fn cmux_hook_session_files(root: &Path) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(agent) = name.strip_suffix("-hook-sessions.json") else {
            continue;
        };
        if agent.is_empty() {
            continue;
        }

        files.push((agent.to_string(), path));
    }

    files
}

pub fn read_tail_utf8(path: &Path, max_bytes: usize) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes as u64);
    file.seek(SeekFrom::Start(start))?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    if start == 0 {
        return Ok(text.into_owned());
    }

    Ok(text
        .split_once('\n')
        .map(|(_, tail)| tail.to_string())
        .unwrap_or_default())
}

pub fn read_head_utf8(path: &Path, max_bytes: usize) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(max_bytes as u64).read_to_end(&mut bytes)?;

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// A window from the start of the file and a window from the end, joined.
///
/// A file that fits inside the two windows put together is read WHOLE instead,
/// decided on its real length. The obvious-looking alternative — stitch the two
/// windows and notice afterwards that the head already ended with the tail —
/// only recognises a file that fits inside ONE window. Anything between one
/// window and two got the middle of the file handed to the caller twice, and
/// the caller counts what it is given.
fn read_head_and_tail_utf8(
    path: &Path,
    head_bytes: usize,
    tail_bytes: usize,
) -> std::io::Result<String> {
    let combined = head_bytes.saturating_add(tail_bytes);
    if fs::metadata(path)?.len() <= combined as u64 {
        return read_head_utf8(path, combined);
    }

    let head = read_head_utf8(path, head_bytes)?;
    let tail = read_tail_utf8(path, tail_bytes)?;
    if tail.is_empty() {
        return Ok(head);
    }

    Ok(format!("{}\n{}", head.trim_end_matches('\n'), tail))
}

fn modified_time(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

pub fn decode_claude_project_dir(name: &str) -> Option<String> {
    decode_claude_project_dir_with_users_root(name, Path::new("/Users"))
}

pub fn decode_claude_project_dir_with_users_root(name: &str, users_root: &Path) -> Option<String> {
    let rest = name.strip_prefix("-Users-")?;
    let segments: Vec<&str> = rest
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return None;
    }

    let mut path = users_root.to_path_buf();
    let mut index = 0;
    while index < segments.len() {
        let mut chosen_len = 1;
        for len in (1..=(segments.len() - index)).rev() {
            let candidate = segments[index..index + len].join("-");
            if path.join(&candidate).exists() {
                chosen_len = len;
                break;
            }
        }

        path.push(segments[index..index + chosen_len].join("-"));
        index += chosen_len;
    }

    Some(path.display().to_string())
}

fn claude_project_dirs_for_root(
    projects_root: &Path,
    project_root: &str,
) -> Vec<(PathBuf, String)> {
    let Ok(entries) = fs::read_dir(projects_root) else {
        return Vec::new();
    };
    let mut directories = entries
        .flatten()
        .filter_map(|entry| {
            let directory = entry.path();
            if !directory.is_dir() {
                return None;
            }
            let decoded = directory
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(decode_claude_project_dir)?;
            let resolved =
                git_project_root(Path::new(&decoded)).unwrap_or_else(|| PathBuf::from(&decoded));
            (resolved.to_string_lossy().as_ref() == project_root).then_some((directory, decoded))
        })
        .collect::<Vec<_>>();
    directories.sort_by(|(left, _), (right, _)| left.cmp(right));
    directories
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_claude_session(home: &Path, directory: &str, id: &str, cwd: &str) {
        let project_dir = home.join(".claude/projects").join(directory);
        fs::create_dir_all(&project_dir).expect("create project fixture");
        let transcript = format!(
            "{}\n{}",
            format_args!(
                r#"{{"type":"user","isSidechain":false,"sessionId":"{id}","cwd":"{cwd}","timestamp":"2026-08-20T09:00:00Z","message":{{"role":"user","content":"Open this project"}}}}"#
            ),
            format_args!(
                r#"{{"type":"assistant","isSidechain":false,"sessionId":"{id}","timestamp":"2026-08-20T09:01:00Z","message":{{"role":"assistant","content":[{{"type":"text","text":"Project opened."}}]}}}}"#
            )
        );
        fs::write(project_dir.join(format!("{id}.jsonl")), transcript)
            .expect("write session fixture");
    }

    #[test]
    fn scan_for_project_returns_only_matching_records() {
        let home = tempfile::tempdir().expect("temp home");
        write_claude_session(
            home.path(),
            "-Users-fixture-matching",
            "matching-session",
            "/Users/fixture/matching",
        );
        write_claude_session(
            home.path(),
            "-Users-fixture-other",
            "other-session",
            "/Users/fixture/other",
        );

        let records = scan_sessions_from_home(home.path(), Some("/Users/fixture/matching"));

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "matching-session");
        assert_eq!(
            records[0].project_path.as_deref(),
            Some("/Users/fixture/matching")
        );
    }

    #[test]
    fn claude_project_dirs_filter_to_requested_root() {
        let projects = tempfile::tempdir().expect("temp projects");
        let matching = projects.path().join("-Users-fixture-matching");
        let other = projects.path().join("-Users-fixture-other");
        fs::create_dir_all(&matching).expect("create matching fixture");
        fs::create_dir_all(&other).expect("create other fixture");

        let directories = claude_project_dirs_for_root(projects.path(), "/Users/fixture/matching");

        assert_eq!(
            directories,
            vec![(matching, "/Users/fixture/matching".to_string())]
        );
    }

    #[test]
    fn codex_head_cwd_filters_rollouts_before_window_reads() {
        let home = tempfile::tempdir().expect("temp home");
        let sessions = home.path().join(".codex/sessions");
        fs::create_dir_all(&sessions).expect("create sessions fixture");

        let matching_id = "019fa964-0000-0000-0000-000000000001";
        let matching = format!(
            "{}\n{}\n{}",
            format_args!(
                r#"{{"timestamp":"2026-08-20T09:00:00Z","type":"session_meta","payload":{{"id":"{matching_id}","cwd":"/Users/fixture/matching","thread_source":"user","source":"cli"}}}}"#
            ),
            r#"{"timestamp":"2026-08-20T09:00:01Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Open the matching project."}]}}"#,
            r#"{"timestamp":"2026-08-20T09:00:02Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Matching project opened."}]}}"#
        );
        fs::write(
            sessions.join(format!("rollout-2026-08-20T09-00-00-{matching_id}.jsonl")),
            matching,
        )
        .expect("write matching rollout fixture");

        let other_id = "019fa964-0000-0000-0000-000000000002";
        let other = format!(
            "{}\n{}\n{}\n{}",
            format_args!(
                r#"{{"timestamp":"2026-08-20T10:00:00Z","type":"session_meta","payload":{{"id":"{other_id}","cwd":"/Users/fixture/other","thread_source":"user","source":"cli"}}}}"#
            ),
            r#"{"timestamp":"2026-08-20T10:00:01Z","type":"response_item","payload":{"type":"function_call","name":"exec_command","arguments":"{\"cmd\":\"pwd\",\"workdir\":\"/Users/fixture/matching\"}"}}"#,
            r#"{"timestamp":"2026-08-20T10:00:02Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"This body points at matching."}]}}"#,
            r#"{"timestamp":"2026-08-20T10:00:03Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"The head must win."}]}}"#
        );
        fs::write(
            sessions.join(format!("rollout-2026-08-20T10-00-00-{other_id}.jsonl")),
            other,
        )
        .expect("write non-matching rollout fixture");

        let records = scan_sessions_from_home(home.path(), Some("/Users/fixture/matching"));

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, matching_id);
        assert!(!records.iter().any(|record| record.id == other_id));
    }

    fn session(title: &str, project_path: Option<&str>) -> AgentSessionRecord {
        AgentSessionRecord {
            provider: "codex".to_string(),
            id: "019e".to_string(),
            title: title.to_string(),
            description: None,
            model: None,
            project_path: project_path.map(ToOwned::to_owned),
            last_activity: None,
            resume_commands: vec!["codex resume 019e".to_string()],
            log_path: None,
            branch_hint: None,
            task_id: None,
            pull_request_hint: None,
            source_label: None,
            message_count: None,
            latest_turn_preview: None,
            latest_turns: Vec::new(),
            project_root: None,
        }
    }

    /// A Claude session with an actual back-and-forth in it, and the tool
    /// traffic that ran in between: two things the user typed (one of them a
    /// slash command, which is not conversation), two answers in words, one
    /// tool call and one tool result.
    ///
    /// The same fixture is written out line for line in
    /// `tauri-svelte-preview/scripts/localSourceFs.test.mjs`. Both scanners
    /// fill the same rail, so a row has to read the same whichever one produced
    /// it.
    const CONVERSATION_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"sessionId":"S9","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-29T09:00:00Z","message":{"role":"user","content":"Fix the resume rail"}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"sessionId":"S9","timestamp":"2026-07-29T09:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"Reading the scanner now."}]}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"sessionId":"S9","timestamp":"2026-07-29T09:02:00Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{}}]}}"#,
        "\n",
        r#"{"type":"user","isSidechain":false,"sessionId":"S9","timestamp":"2026-07-29T09:03:00Z","message":{"role":"user","content":[{"type":"tool_result","content":"File does not exist."}]}}"#,
        "\n",
        r#"{"type":"user","isSidechain":false,"sessionId":"S9","timestamp":"2026-07-29T09:04:00Z","message":{"role":"user","content":"<command-name>compact</command-name>"}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"sessionId":"S9","timestamp":"2026-07-29T09:05:00Z","message":{"role":"assistant","content":[{"type":"text","text":"The  rail was\n  reading the wrong file."}]}}"#,
    );

    /// The same session ending on an answer far longer than one line, so the
    /// cut is pinned. Also mirrored in the TypeScript suite.
    const LONG_ANSWER_TEXT: &str = "The scanner was reading the wrong file the whole time, which is why every row said Claude session and none of them said anything else at all";

    /// A Codex session with the agent's reply in it, on top of the opening
    /// turns Codex writes for itself.
    const CODEX_CONVERSATION_ROLLOUT_JSONL: &str = concat!(
        r#"{"timestamp":"2026-07-28T16:42:17.000Z","type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"user","source":"cli"}}"#,
        "\n",
        r##"{"timestamp":"2026-07-28T16:42:18.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"# AGENTS.md instructions for /Users/dev/work/rental-management"}]}}"##,
        "\n",
        r#"{"timestamp":"2026-07-28T16:42:30.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Please plan out an entire year of scans and entries for the 2027 simulation."}]}}"#,
        "\n",
        r#"{"timestamp":"2026-07-28T16:43:00.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Here  is the plan\n for 2027."}]}}"#,
    );

    /// The app can only offer "open the transcript" for a session whose record
    /// remembers which file it was read out of.
    #[test]
    fn log_path_is_recorded_for_a_scanned_session() {
        let file = Path::new("/Users/dev/.claude/projects/mac-command-bar/S9.jsonl");
        let records = with_log_path(
            parse_claude_jsonl(CONVERSATION_JSONL, "/Users/dev/work/mac-command-bar"),
            file,
        );

        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].log_path.as_deref(),
            Some("/Users/dev/.claude/projects/mac-command-bar/S9.jsonl")
        );
    }

    /// One session can turn up in more than one file. Merging must not blank the
    /// path already on the record, or hand back the later file instead.
    #[test]
    fn merged_records_keep_the_first_log_path() {
        let project = Some("/Users/dev/work/mac-command-bar");
        let mut first = session("Fix the resume rail", project);
        first.log_path = Some("/Users/dev/.codex/sessions/first.jsonl".to_string());
        let mut second = session("Fix the resume rail", project);
        second.log_path = Some("/Users/dev/.codex/sessions/second.jsonl".to_string());
        second.last_activity = Some("2026-07-30T10:00:00Z".to_string());

        let merged = merge_agent_session_records(vec![first, second]);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].log_path.as_deref(),
            Some("/Users/dev/.codex/sessions/first.jsonl")
        );
    }

    /// A row wants to say "12 messages · Agent: fixed the reference race…", and
    /// this is where both halves of that come from. Only words count: the tool
    /// call, its result and the slash command are how the work got done, not
    /// what was said, and counting them would put hundreds on a row where the
    /// two of them exchanged a dozen.
    #[test]
    fn claude_rows_carry_how_many_turns_were_said_and_the_last_of_them() {
        let records = parse_claude_jsonl(CONVERSATION_JSONL, "/Users/dev/work/mac-command-bar");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message_count, Some(3));
        assert_eq!(
            records[0].latest_turn_preview.as_deref(),
            Some("Agent: The rail was reading the wrong file.")
        );

        // A transcript that held no conversation says nothing rather than zero.
        let bare = r#"{"type":"file-history-snapshot","sessionId":"S3","timestamp":"2026-07-28T09:00:00Z"}"#;
        let records = parse_claude_jsonl(bare, "");
        assert_eq!(records[0].message_count, None);
        assert_eq!(records[0].latest_turn_preview, None);

        // The user's own turn is shown as the user's when it came last.
        let user_last = format!(
            "{CONVERSATION_JSONL}\n{}",
            r#"{"type":"user","isSidechain":false,"sessionId":"S9","timestamp":"2026-07-29T09:06:00Z","message":{"role":"user","content":"Try it again"}}"#
        );
        let records = parse_claude_jsonl(&user_last, "/Users/dev/work/mac-command-bar");
        assert_eq!(records[0].message_count, Some(4));
        assert_eq!(
            records[0].latest_turn_preview.as_deref(),
            Some("You: Try it again")
        );
    }

    /// One line means one line: an answer longer than the row can hold is cut
    /// and marked, and the cut lands on a character boundary.
    #[test]
    fn a_long_last_turn_is_cut_to_one_line() {
        let long = CONVERSATION_JSONL.replace(
            "The  rail was\\n  reading the wrong file.",
            LONG_ANSWER_TEXT,
        );
        let records = parse_claude_jsonl(&long, "/Users/dev/work/mac-command-bar");

        let preview = records[0].latest_turn_preview.clone().unwrap();
        assert_eq!(preview.chars().count(), 120);
        assert!(preview.starts_with("Agent: The scanner was reading the wrong file"));
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn codex_rows_carry_how_many_turns_were_said_and_the_last_of_them() {
        let records = parse_codex_rollout_jsonl(CODEX_CONVERSATION_ROLLOUT_JSONL);

        assert_eq!(records.len(), 1);
        // The repository instructions Codex sends as the user are not a turn.
        assert_eq!(records[0].message_count, Some(2));
        assert_eq!(
            records[0].latest_turn_preview.as_deref(),
            Some("Agent: Here is the plan for 2027.")
        );

        // A rollout file with nothing but its opening record says nothing.
        let meta_only = r#"{"type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev","source":"cli"}}"#;
        let records = parse_codex_rollout_jsonl(meta_only);
        assert_eq!(records[0].message_count, None);
        assert_eq!(records[0].latest_turn_preview, None);

        // The index file knows neither, and must not wipe out what the rollout
        // file found when the two records are merged.
        let index = parse_codex_index_jsonl(
            r#"{"id":"019fa964","thread_name":"Year simulation planning","updated_at":"2026-07-29T16:00:00.000000Z"}"#,
        );
        let merged = merge_codex_session_metadata(
            index,
            parse_codex_rollout_jsonl(CODEX_CONVERSATION_ROLLOUT_JSONL),
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].message_count, Some(2));
        assert_eq!(
            merged[0].latest_turn_preview.as_deref(),
            Some("Agent: Here is the plan for 2027.")
        );
    }

    #[test]
    fn one_session_detail_read_opens_only_the_named_transcript() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("rollout-019fa964.jsonl");
        fs::write(&path, CODEX_CONVERSATION_ROLLOUT_JSONL).expect("write transcript");

        let records = scan_session_details(path.to_string_lossy().as_ref());

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message_count, Some(2));
        assert_eq!(records[0].latest_turns.len(), 2);
        assert_eq!(records[0].log_path.as_deref(), path.to_str());
    }

    /// Pressing Escape in the middle of an answer is how a session usually
    /// ends, and Claude writes that as a user message reading "[Request
    /// interrupted by user]". It is not something the user said, so it is not a
    /// turn — and because the row shows the LAST turn, letting it through put
    /// "You: [Request interrupted by user]" on the most prominent line of the
    /// card, where it says nothing at all about the session. The row falls back
    /// to the last thing that really was said.
    #[test]
    fn an_interrupted_request_is_neither_counted_nor_shown() {
        let interrupted = format!(
            "{CONVERSATION_JSONL}\n{}",
            r#"{"type":"user","isSidechain":false,"sessionId":"S9","timestamp":"2026-07-29T09:06:00Z","message":{"role":"user","content":"[Request interrupted by user]"}}"#
        );
        let records = parse_claude_jsonl(&interrupted, "/Users/dev/work/mac-command-bar");

        assert_eq!(records[0].message_count, Some(3));
        assert_eq!(
            records[0].latest_turn_preview.as_deref(),
            Some("Agent: The rail was reading the wrong file.")
        );
    }

    /// Codex writes a separate record for every paragraph it narrates between
    /// tool calls, so counting records put roughly twenty times as many
    /// "messages" on a Codex row as on a Claude row for the same amount of
    /// conversation — and the two sit side by side on the same list. A run of
    /// them is one thing the agent said back, so it counts once.
    #[test]
    fn codex_narration_between_tool_calls_counts_as_one_agent_turn() {
        let narrated = concat!(
            r#"{"timestamp":"2026-07-28T16:42:17.000Z","type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"user","source":"cli"}}"#,
            "\n",
            r#"{"timestamp":"2026-07-28T16:42:30.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Plan the 2027 simulation."}]}}"#,
            "\n",
            r#"{"timestamp":"2026-07-28T16:43:00.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Reading the scanner now."}]}}"#,
            "\n",
            r#"{"timestamp":"2026-07-28T16:43:10.000Z","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{}"}}"#,
            "\n",
            r#"{"timestamp":"2026-07-28T16:43:20.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Now changing the reader."}]}}"#,
            "\n",
            r#"{"timestamp":"2026-07-28T16:43:30.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Here is the plan for 2027."}]}}"#,
        );

        let records = parse_codex_rollout_jsonl(narrated);

        // One thing the user typed, one thing the agent said back.
        assert_eq!(records[0].message_count, Some(2));
        assert_eq!(
            records[0].latest_turn_preview.as_deref(),
            Some("Agent: Here is the plan for 2027.")
        );

        // A second exchange is a second pair, so the count still grows with the
        // conversation rather than with the narration.
        let second_exchange = format!(
            "{narrated}\n{}\n{}",
            r#"{"timestamp":"2026-07-28T16:44:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Go ahead."}]}}"#,
            r#"{"timestamp":"2026-07-28T16:44:10.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Done."}]}}"#
        );
        let records = parse_codex_rollout_jsonl(&second_exchange);
        assert_eq!(records[0].message_count, Some(4));
        assert_eq!(
            records[0].latest_turn_preview.as_deref(),
            Some("Agent: Done.")
        );
    }

    /// A rollout file is read as a window from the start and a window from the
    /// end. When the file is bigger than one window but smaller than two, those
    /// two windows OVERLAP, and every line in the overlap used to be handed to
    /// the parser twice. Nothing noticed until a row started counting turns:
    /// counting is the first thing this parser does that is not idempotent, and
    /// a session in that size band reported a number inflated by the overlap —
    /// while the same file through the web preview reported the true one.
    #[test]
    fn a_file_between_one_window_and_two_is_not_read_twice() {
        // Long enough to run past one window, short enough to stay inside two.
        let padding = "x".repeat(700);
        let mut lines = vec![
            r#"{"timestamp":"2026-07-28T16:42:17.000Z","type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"user","source":"cli"}}"#
                .to_string(),
        ];
        let exchanges = 200;
        for index in 0..exchanges {
            lines.push(format!(
                r#"{{"timestamp":"2026-07-28T16:42:30.000Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"Question {index} {padding}"}}]}}}}"#
            ));
            lines.push(format!(
                r#"{{"timestamp":"2026-07-28T16:43:00.000Z","type":"response_item","payload":{{"type":"message","role":"assistant","content":[{{"type":"output_text","text":"Answer {index} {padding}"}}]}}}}"#
            ));
        }
        let body = lines.join("\n");

        let combined = CODEX_SESSION_HEAD_BYTES + CODEX_SESSION_TAIL_BYTES;
        assert!(
            body.len() > CODEX_SESSION_HEAD_BYTES && body.len() < combined,
            "the fixture has to land in the band where the two windows overlap; \
             it is {} bytes and the band is {}..{}",
            body.len(),
            CODEX_SESSION_HEAD_BYTES,
            combined
        );

        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("rollout-019fa964.jsonl");
        fs::write(&path, &body).expect("write the fixture");

        let read = read_head_and_tail_utf8(&path, CODEX_SESSION_HEAD_BYTES, CODEX_SESSION_TAIL_BYTES)
            .expect("read the fixture");
        assert_eq!(
            read.lines().count(),
            body.lines().count(),
            "the file came back with lines in it more than once"
        );

        let records = parse_codex_rollout_jsonl(&read);
        assert_eq!(records[0].message_count, Some(exchanges * 2));
    }

    #[test]
    fn derived_metadata_extracts_explicit_agent_session_hints() {
        let record = session(
            "TSK-127 branch cdx/tsk-127-agent-session-metadata PR #42 https://github.com/acme/mac-command-bar/pull/42",
            Some("/Users/blackcolours/dev/work/mac-command-bar"),
        );

        let metadata = derive_agent_session_metadata(&record);

        assert_eq!(metadata.task_id.as_deref(), Some("TSK-127"));
        assert_eq!(
            metadata.branch_hint.as_deref(),
            Some("cdx/tsk-127-agent-session-metadata")
        );
        assert_eq!(metadata.pull_request_hint.as_deref(), Some("PR #42"));
        assert_eq!(
            metadata.link_hint.as_deref(),
            Some("https://github.com/acme/mac-command-bar/pull/42")
        );
        assert_eq!(metadata.source_label, "Codex · mac-command-bar");
    }

    #[test]
    fn derived_metadata_uses_path_task_ids_and_keeps_unclear_hints_empty() {
        let record = session(
            "Claude session",
            Some("/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-128-runtime-audit"),
        );

        let metadata = derive_agent_session_metadata(&record);

        assert_eq!(metadata.task_id.as_deref(), Some("TSK-128"));
        assert_eq!(metadata.branch_hint, None);
        assert_eq!(metadata.pull_request_hint, None);
        assert_eq!(metadata.link_hint, None);
        assert_eq!(metadata.source_label, "Codex · tsk-128-runtime-audit");
    }

    /// A real Claude session: a user prompt, then an assistant turn that is
    /// nothing but a `tool_use`. The tool_use line is NEWER, so the merge takes
    /// its (empty) title — which is exactly why real rails filled up with
    /// "Claude session".
    const REAL_SESSION_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"sessionId":"S1","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T09:00:00Z","message":{"role":"user","content":"Fix the resume rail"}}"#,
        "\n",
        r#"{"type":"user","isSidechain":false,"sessionId":"S1","timestamp":"2026-07-28T09:01:00Z","message":{"role":"user","content":[{"type":"tool_result","content":"File does not exist."}]}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"sessionId":"S1","timestamp":"2026-07-28T09:02:00Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{}}]}}"#,
    );

    /// A subagent transcript: the Task tool's prompt and its structured reply,
    /// every line `isSidechain: true`, and a `sessionId` that would otherwise
    /// mint a resumable-looking row.
    const SUBAGENT_JSONL: &str = concat!(
        r#"{"parentUuid":null,"isSidechain":true,"agentId":"a0a5","type":"user","sessionId":"S2","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T10:00:00Z","message":{"role":"user","content":"You are implementing Task 6 of the Slice 1 plan."}}"#,
        "\n",
        r#"{"parentUuid":"1","isSidechain":true,"agentId":"a0a5","type":"assistant","sessionId":"S2","timestamp":"2026-07-28T10:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"Structured output provided successfully"}]}}"#,
    );

    /// A helper-agent transcript as Claude Code actually writes it: flat in the
    /// project directory, `isSidechain: false`, `userType: "external"`, no agent
    /// name anywhere — identical to a real session except for the entrypoint
    /// that launched it.
    const HELPER_AGENT_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"userType":"external","entrypoint":"sdk-cli","promptSource":"sdk","sessionId":"S4","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T11:00:00Z","message":{"role":"user","content":"You are implementing Task 1 of the v2-base plan."}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"userType":"external","entrypoint":"sdk-cli","sessionId":"S4","timestamp":"2026-07-28T11:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"Reading the brief."}]}}"#,
    );

    /// A REAL session that opens with pasted log output. The user does this
    /// often, and it is why the filter reads the entrypoint rather than
    /// guessing from the shape of the first message: nothing about this text
    /// distinguishes it from a machine-authored dispatch prompt.
    const REAL_SESSION_STARTING_WITH_A_PASTED_LOG_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"userType":"external","entrypoint":"cli","promptSource":"typed","sessionId":"S5","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T12:00:00Z","message":{"role":"user","content":"You are seeing this in the console:\n[vite] hmr update /src/routes/next/+page.svelte\nERROR  Cannot read properties of undefined (reading 'api')\n    at mount (chunk-QK6X.js:14:9)\nReview this change and tell me why. Your final message should name the file."}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"entrypoint":"cli","sessionId":"S5","timestamp":"2026-07-28T12:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"That mount is running before the store exists."}]}}"#,
    );

    /// A hook-spawned security reviewer, the exact text Claude Code writes:
    /// "Review this change for security vulnerabilities" followed by the file
    /// list and the diff. 186 of these exist here and every one is `sdk-py`.
    const SECURITY_REVIEW_AGENT_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"userType":"external","entrypoint":"sdk-py","promptSource":"sdk","sessionId":"S7","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T13:00:00Z","message":{"role":"user","content":"Review this change for security vulnerabilities.\n\nChanged files (you may Read these and any other file in the repo):\n  - tauri-svelte-preview/src/lib/shell/terminalService.ts\n\nUnified diff (only + lines are new):"}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"entrypoint":"sdk-py","sessionId":"S7","timestamp":"2026-07-28T13:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"No vulnerabilities found."}]}}"#,
    );

    /// A REAL session resumed after running out of context. Claude Code opens it
    /// with a machine-written summary, so it reads exactly like a dispatch
    /// prompt — and this one is 22770 lines of the user's own work.
    const REAL_SESSION_RESUMED_FROM_A_SUMMARY_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"userType":"external","entrypoint":"cli","sessionId":"S8","cwd":"/Users/dev/work/EdiPlatform","timestamp":"2026-07-28T14:00:00Z","message":{"role":"user","content":"This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\nSummary:\n1. Primary Request and Intent:\n   Make the rule-extraction pipeline leaner and faster."}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"entrypoint":"cli","sessionId":"S8","timestamp":"2026-07-28T14:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"Picking up where that left off."}]}}"#,
    );

    /// A thread Codex spawned for itself. Both markers are present, as they are
    /// on 618 of the 619 helper threads on this machine.
    const CODEX_SUBAGENT_ROLLOUT_JSONL: &str = concat!(
        r#"{"timestamp":"2026-07-28T17:24:36.972Z","type":"session_meta","payload":{"id":"019fa9c1","parent_thread_id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"subagent","agent_role":"executor","source":{"subagent":{"parent_thread_id":"019fa964","depth":1}}}}"#,
        "\n",
        r#"{"timestamp":"2026-07-28T17:25:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Implement the year simulation runner."}]}}"#,
    );

    /// A session the user started in the terminal. `source` is a plain string,
    /// and the first two turns are the ones Codex writes for itself before the
    /// user has typed anything.
    const CODEX_TOP_LEVEL_ROLLOUT_JSONL: &str = concat!(
        r#"{"timestamp":"2026-07-28T16:42:17.000Z","type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"user","source":"cli"}}"#,
        "\n",
        r##"{"timestamp":"2026-07-28T16:42:18.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"# AGENTS.md instructions for /Users/dev/work/rental-management\n\n<INSTRUCTIONS>\nEvery reply MUST open with a plain-English summary.\n</INSTRUCTIONS>"}]}}"##,
        "\n",
        r#"{"timestamp":"2026-07-28T16:42:19.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>\n  <cwd>/Users/dev/work/rental-management</cwd>\n</environment_context>"}]}}"#,
        "\n",
        r#"{"timestamp":"2026-07-28T16:42:30.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Please plan out an entire year of scans and entries for the 2027 simulation."}]}}"#,
    );

    /// An older Codex build that wrote no thread marker at all.
    const CODEX_UNMARKED_ROLLOUT_JSONL: &str = concat!(
        r#"{"timestamp":"2026-04-14T13:53:07.000Z","type":"session_meta","payload":{"id":"019d8d20","cwd":"/Users/dev/work/EdiPlatform","originator":"codex_cli_rs","source":"unknown"}}"#,
        "\n",
        r#"{"timestamp":"2026-04-14T13:54:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Help me trace where the 810 mapping loses the invoice date."}]}}"#,
    );

    #[test]
    fn codex_scan_excludes_threads_codex_spawned_for_itself() {
        // Either marker on its own is enough — they disagreed on one real file.
        assert!(codex_rollout_head_is_subagent_thread(
            r#"{"type":"session_meta","payload":{"thread_source":"subagent","source":"vscode"}}"#
        ));
        assert!(codex_rollout_head_is_subagent_thread(
            r#"{"type":"session_meta","payload":{"source":{"subagent":{"depth":1}}}}"#
        ));

        // A session the user started: `source` is a plain string, not an object.
        assert!(!codex_rollout_head_is_subagent_thread(
            r#"{"type":"session_meta","payload":{"thread_source":"user","source":"cli"}}"#
        ));
        // No marker, unreadable, or not a metadata line at all: keep the file.
        assert!(!codex_rollout_head_is_subagent_thread(
            r#"{"type":"session_meta","payload":{"source":"unknown"}}"#
        ));
        assert!(!codex_rollout_head_is_subagent_thread("{\"type\":\"sessi"));
        assert!(!codex_rollout_head_is_subagent_thread(""));

        assert_eq!(
            parse_codex_rollout_jsonl(CODEX_SUBAGENT_ROLLOUT_JSONL),
            Vec::new()
        );

        let records = parse_codex_rollout_jsonl(CODEX_TOP_LEVEL_ROLLOUT_JSONL);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "019fa964");

        // Older rollouts predate the marker. Keep them: losing a session the
        // user wants back is worse than listing a helper thread.
        let records = parse_codex_rollout_jsonl(CODEX_UNMARKED_ROLLOUT_JSONL);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "019d8d20");
    }

    /// The index file names sub-agent threads as plausibly as it names the
    /// user's own — "Audit inventory gaps", "Review mobile steppers" — so the
    /// name is no help at all and the rollout files have to answer.
    #[test]
    fn codex_index_rows_are_dropped_when_a_rollout_proves_them_helper_threads() {
        // The id travels with the verdict, so an archived helper thread can be
        // recognised from its rollout file and struck off the index.
        let marker = codex_rollout_head_thread_marker(CODEX_SUBAGENT_ROLLOUT_JSONL).unwrap();
        assert_eq!(marker.id.as_deref(), Some("019fa9c1"));
        assert!(marker.spawned_by_codex);

        let marker = codex_rollout_head_thread_marker(CODEX_TOP_LEVEL_ROLLOUT_JSONL).unwrap();
        assert_eq!(marker.id.as_deref(), Some("019fa964"));
        assert!(!marker.spawned_by_codex);

        // A metadata record with no id still answers the question it can.
        let marker = codex_rollout_head_thread_marker(
            r#"{"type":"session_meta","payload":{"thread_source":"subagent"}}"#,
        )
        .unwrap();
        assert_eq!(marker.id, None);
        assert!(marker.spawned_by_codex);

        // Nothing readable: no verdict either way.
        assert_eq!(codex_rollout_head_thread_marker("{\"type\":\"sessi"), None);
        assert_eq!(codex_rollout_head_thread_marker(""), None);

        let index = parse_codex_index_jsonl(concat!(
            r#"{"id":"019fa9c1","thread_name":"Audit inventory gaps","updated_at":"2026-07-28T17:24:00.000000Z"}"#,
            "\n",
            r#"{"id":"019fa964","thread_name":"Year simulation planning","updated_at":"2026-07-28T16:42:00.000000Z"}"#,
            "\n",
            r#"{"id":"019c230d","thread_name":"Document EDI flow review","updated_at":"2026-03-13T21:48:02.611673Z"}"#,
        ));
        assert_eq!(index.len(), 3);

        let helper_threads = HashSet::from(["019fa9c1".to_string()]);
        let kept = drop_codex_subagent_sessions(index, &helper_threads);

        // The helper thread goes. The session the user started stays, and so
        // does the one whose rollout file is gone from disk entirely — no
        // evidence is not evidence.
        assert_eq!(
            kept.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(),
            vec!["019fa964", "019c230d"]
        );
    }

    /// An index row on its own has no working folder, so a thread whose rollout
    /// file falls outside the newest-files budget used to reach the History
    /// panel with nowhere to resume into. The budget still bounds how much is
    /// read; it just no longer excludes a file the index is still pointing at.
    #[test]
    fn codex_rollout_files_the_index_still_names_are_read_past_the_budget() {
        let newest: Vec<PathBuf> = (0..CODEX_SESSION_FILE_LIMIT)
            .map(|position| {
                PathBuf::from(format!(
                    "/sessions/rollout-2026-08-13T00-00-{position:04}-newest{position}.jsonl"
                ))
            })
            .collect();
        let indexed_old =
            PathBuf::from("/sessions/rollout-2026-03-13T21-48-02-019c230d-2835-7b23.jsonl");
        let forgotten_old =
            PathBuf::from("/sessions/rollout-2026-03-12T10-00-00-019c0000-0000-0000.jsonl");

        let mut files = newest.clone();
        files.push(indexed_old.clone());
        files.push(forgotten_old.clone());

        let indexed_ids = HashSet::from(["019c230d-2835-7b23".to_string()]);
        let selected = codex_rollout_files_to_read(files, &indexed_ids);

        assert_eq!(selected.len(), CODEX_SESSION_FILE_LIMIT + 1);
        assert!(selected.contains(&indexed_old));
        assert!(!selected.contains(&forgotten_old));
        assert_eq!(selected[..CODEX_SESSION_FILE_LIMIT], newest[..]);
    }

    #[test]
    fn codex_titles_come_from_the_first_prompt_the_user_typed() {
        let records = parse_codex_rollout_jsonl(CODEX_TOP_LEVEL_ROLLOUT_JSONL);
        assert_eq!(
            records[0].title,
            "Please plan out an entire year of scans and entries for the…"
        );

        // The repository instructions and the environment block are written by
        // Codex, not the user, so neither may become the title.
        assert!(!records[0].title.contains("AGENTS.md"));
        assert!(!records[0].title.contains("environment_context"));

        // Nothing typed inside the scanned window leaves the generic label.
        let meta_only = r#"{"type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev","source":"cli"}}"#;
        assert_eq!(
            parse_codex_rollout_jsonl(meta_only)[0].title,
            CODEX_GENERIC_SESSION_TITLE
        );

        // The index file names some sessions. That name must survive a merge
        // with the rollout file whichever record carries the later timestamp.
        let index = parse_codex_index_jsonl(
            r#"{"id":"019fa964","thread_name":"Year simulation planning","updated_at":"2026-07-28T16:00:00.000000Z"}"#,
        );
        let merged =
            merge_codex_session_metadata(index, parse_codex_rollout_jsonl(CODEX_TOP_LEVEL_ROLLOUT_JSONL));
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].title, "Year simulation planning");

        // And an unnamed index row must not overwrite a real rollout title.
        let index = parse_codex_index_jsonl(
            r#"{"id":"019fa964","updated_at":"2026-07-29T16:00:00.000000Z"}"#,
        );
        let merged =
            merge_codex_session_metadata(index, parse_codex_rollout_jsonl(CODEX_TOP_LEVEL_ROLLOUT_JSONL));
        assert_eq!(
            merged[0].title,
            "Please plan out an entire year of scans and entries for the…"
        );
    }

    #[test]
    fn claude_scan_excludes_helper_agent_transcripts() {
        // The entrypoint is the whole rule: `sdk-*` launched it programmatically.
        assert!(is_agent_launch_entrypoint("sdk-cli"));
        assert!(is_agent_launch_entrypoint("sdk-py"));
        assert!(!is_agent_launch_entrypoint("cli"));
        assert!(!is_agent_launch_entrypoint("claude-vscode"));
        assert!(!is_agent_launch_entrypoint(""));

        assert_eq!(
            parse_claude_jsonl(HELPER_AGENT_JSONL, "/Users/dev/work/mac-command-bar"),
            Vec::new()
        );

        // A real session that opens with pasted log output and the very phrases
        // a dispatch prompt uses ("You are ", "Review this change", "Your final
        // message") must survive. Excluding one of the user's own sessions is
        // strictly worse than listing a helper.
        let records = parse_claude_jsonl(
            REAL_SESSION_STARTING_WITH_A_PASTED_LOG_JSONL,
            "/Users/dev/work/mac-command-bar",
        );
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "S5");

        // Older transcripts predate the field. Keep them: a missing entrypoint
        // is not evidence of a helper, and the cost of guessing wrong is losing
        // a session the user wanted.
        let records = parse_claude_jsonl(REAL_SESSION_JSONL, "/Users/dev/work/mac-command-bar");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "S1");
    }

    /// The shape that flooded the rail: an API job's transcripts, hundreds of
    /// them, written into a temporary directory. Every record carries the
    /// launch entrypoint — but the opening records do not, so the probe has to
    /// read past them.
    const API_BATCH_JOB_JSONL: &str = concat!(
        r#"{"type":"queue-operation","operation":"enqueue","timestamp":"2026-07-27T15:25:58.319Z","sessionId":"S6","content":"Extract the fields below from the following document text."}"#,
        "\n",
        r#"{"type":"queue-operation","operation":"dequeue","timestamp":"2026-07-27T15:25:58.319Z","sessionId":"S6"}"#,
        "\n",
        r#"{"type":"user","isSidechain":false,"userType":"external","entrypoint":"sdk-cli","promptSource":"sdk","sessionId":"S6","cwd":"/private/var/folders/rp/T","timestamp":"2026-07-27T15:25:59.000Z","message":{"role":"user","content":"You are a friendly assistant for extracting rental documents."}}"#,
    );

    /// Locks in why there is no "does this read like a dispatch prompt?" rule.
    ///
    /// The security reviewers that fill the rail are launched programmatically
    /// like every other helper, so the entrypoint already answers for them —
    /// checked here at both the probe and the parse. Guessing from the text
    /// instead would cost real sessions: the second fixture opens with a
    /// machine-written summary and IS the user's own work, and the third opens
    /// with the words "Review this change" typed by the user.
    #[test]
    fn claude_scan_reads_how_a_run_was_launched_not_what_it_says() {
        assert!(claude_transcript_head_is_agent_launched(
            SECURITY_REVIEW_AGENT_JSONL
        ));
        assert_eq!(
            parse_claude_jsonl(SECURITY_REVIEW_AGENT_JSONL, "/Users/dev/work/mac-command-bar"),
            Vec::new()
        );

        for (fixture, id) in [
            (REAL_SESSION_RESUMED_FROM_A_SUMMARY_JSONL, "S8"),
            (REAL_SESSION_STARTING_WITH_A_PASTED_LOG_JSONL, "S5"),
        ] {
            assert!(!claude_transcript_head_is_agent_launched(fixture));
            let records = parse_claude_jsonl(fixture, "/Users/dev/work/EdiPlatform");
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].id, id);
        }
    }

    #[test]
    fn claude_scan_skips_helper_runs_before_the_file_budget() {
        // Reading past the opening records is the whole point: the launch
        // entrypoint appears only once the first real turn is written.
        assert!(claude_transcript_head_is_agent_launched(API_BATCH_JOB_JSONL));
        assert!(claude_transcript_head_is_agent_launched(HELPER_AGENT_JSONL));

        // A session the user typed into, and one old enough to carry no
        // entrypoint at all, both stay in the scan.
        assert!(!claude_transcript_head_is_agent_launched(
            REAL_SESSION_STARTING_WITH_A_PASTED_LOG_JSONL
        ));
        assert!(!claude_transcript_head_is_agent_launched(REAL_SESSION_JSONL));

        // A head cut mid-line, and one holding nothing but the opening records,
        // are both kept — the parse of the full file settles them.
        assert!(!claude_transcript_head_is_agent_launched(
            r#"{"type":"user","entrypoint":"sdk-c"#
        ));
        assert!(!claude_transcript_head_is_agent_launched(
            r#"{"type":"queue-operation","operation":"enqueue","sessionId":"S6"}"#
        ));
        assert!(!claude_transcript_head_is_agent_launched(""));

        // And the rule the probe applies is the one the parse applies.
        assert_eq!(
            parse_claude_jsonl(API_BATCH_JOB_JSONL, "/private/var/folders/rp/T"),
            Vec::new()
        );
    }

    #[test]
    fn claude_scan_excludes_subagent_sidechain_transcripts() {
        // Path discriminator — free, and it also keeps subagents from evicting
        // real sessions from the scan's file budget.
        assert!(is_claude_subagent_transcript_path(Path::new(
            "/Users/dev/.claude/projects/-Users-dev/71dd/subagents/agent-a0a5.jsonl"
        )));
        assert!(is_claude_subagent_transcript_path(Path::new(
            "/Users/dev/.claude/projects/-Users-dev/71dd/subagents/workflows/wf_02/agent-a2.jsonl"
        )));
        assert!(!is_claude_subagent_transcript_path(Path::new(
            "/Users/dev/.claude/projects/-Users-dev/71dd4d8f.jsonl"
        )));

        // Content discriminator — authoritative, and it still catches a
        // subagent transcript written flat into the project directory.
        assert_eq!(
            parse_claude_jsonl(SUBAGENT_JSONL, "/Users/dev/work/mac-command-bar"),
            Vec::new()
        );

        let records = parse_claude_jsonl(REAL_SESSION_JSONL, "/Users/dev/work/mac-command-bar");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "S1");
    }

    #[test]
    fn claude_titles_fall_back_to_project_folder_and_first_prompt() {
        let records = parse_claude_jsonl(REAL_SESSION_JSONL, "/Users/dev/work/mac-command-bar");

        // Without the fallback this row reads "Claude session": the newest line
        // is a tool_use with no text, and a tool_result is not a user prompt.
        assert_eq!(records[0].title, "mac-command-bar — Fix the resume rail");
    }

    #[test]
    fn claude_ai_title_wins_and_long_fallbacks_are_truncated() {
        let with_ai_title = format!(
            "{REAL_SESSION_JSONL}\n{}",
            r#"{"type":"ai-title","sessionId":"S1","aiTitle":"Resume rail subagent filter"}"#
        );
        let records = parse_claude_jsonl(&with_ai_title, "/Users/dev/work/mac-command-bar");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "Resume rail subagent filter");

        let long_prompt = REAL_SESSION_JSONL.replace(
            "Fix the resume rail",
            "Fix the resume rail so it stops listing subagent transcripts and generic titles",
        );
        let records = parse_claude_jsonl(&long_prompt, "/Users/dev/work/mac-command-bar");
        assert_eq!(records[0].title.chars().count(), 60);
        assert!(records[0].title.starts_with("mac-command-bar — Fix the resume rail"));
        assert!(records[0].title.ends_with('…'));

        // Nothing usable anywhere: the generic label is still the last resort.
        let bare = r#"{"type":"file-history-snapshot","sessionId":"S3","timestamp":"2026-07-28T09:00:00Z"}"#;
        let records = parse_claude_jsonl(bare, "");
        assert_eq!(records[0].title, "Claude session");
    }

    /// A row can only show what the record carries. The scan fills these in at
    /// the end of its pass, so a record built anywhere else has them empty, and
    /// an empty one must not put a blank chip on the row — it is left out of the
    /// JSON entirely.
    #[test]
    fn agent_session_record_json_carries_the_hints_the_scan_derived() {
        let record = session("TSK-127 metadata", Some("/repo"));

        let value = serde_json::to_value(&record).unwrap();

        assert_eq!(value.get("taskId"), None);
        assert_eq!(value.get("branchHint"), None);
        assert_eq!(value.get("pullRequestHint"), None);
        assert_eq!(value.get("sourceLabel"), None);
        assert_eq!(value.get("messageCount"), None);
        assert_eq!(value.get("latestTurnPreview"), None);
        assert_eq!(
            value.get("projectPath").and_then(Value::as_str),
            Some("/repo")
        );

        let mut record = session(
            "TSK-127 branch cdx/tsk-127-agent-session-metadata PR #42",
            Some("/repo"),
        );
        let metadata = derive_agent_session_metadata(&record);
        record.branch_hint = metadata.branch_hint;
        record.task_id = metadata.task_id;
        record.pull_request_hint = metadata.pull_request_hint;
        record.source_label = Some(metadata.source_label);
        record.message_count = Some(12);
        record.latest_turn_preview = Some("Agent: fixed the reference race".to_string());

        let value = serde_json::to_value(&record).unwrap();

        assert_eq!(value.get("taskId").and_then(Value::as_str), Some("TSK-127"));
        assert_eq!(
            value.get("branchHint").and_then(Value::as_str),
            Some("cdx/tsk-127-agent-session-metadata")
        );
        assert_eq!(
            value.get("pullRequestHint").and_then(Value::as_str),
            Some("PR #42")
        );
        assert_eq!(
            value.get("sourceLabel").and_then(Value::as_str),
            Some("Codex · repo")
        );
        assert_eq!(value.get("messageCount").and_then(Value::as_u64), Some(12));
        assert_eq!(
            value.get("latestTurnPreview").and_then(Value::as_str),
            Some("Agent: fixed the reference race")
        );
        // Nothing renders the link yet, so it is not sent.
        assert_eq!(value.get("linkHint"), None);
    }

    /// The whole point of the fields: the scan hands the shell the branch, task
    /// and pull request it already worked out, on the same record the row draws.
    #[test]
    fn finished_records_carry_the_metadata_the_scan_derived() {
        let records = with_derived_agent_session_metadata(vec![
            session(
                "TSK-127 branch cdx/tsk-127-agent-session-metadata PR #42",
                Some("/Users/blackcolours/dev/work/mac-command-bar"),
            ),
            session("Claude session", None),
        ]);

        assert_eq!(records[0].task_id.as_deref(), Some("TSK-127"));
        assert_eq!(
            records[0].branch_hint.as_deref(),
            Some("cdx/tsk-127-agent-session-metadata")
        );
        assert_eq!(records[0].pull_request_hint.as_deref(), Some("PR #42"));
        assert_eq!(
            records[0].source_label.as_deref(),
            Some("Codex · mac-command-bar")
        );

        // Nothing to say is said as nothing, not as an empty chip.
        assert_eq!(records[1].task_id, None);
        assert_eq!(records[1].branch_hint, None);
        assert_eq!(records[1].pull_request_hint, None);
        assert_eq!(
            records[1].source_label.as_deref(),
            Some("Codex · Claude session")
        );
    }
}
