//! Counting how many lines of a project mention each of a set of symbols, in
//! one pass over the files.
//!
//! This lives in the core crate rather than in the desktop app crate for one
//! reason: speed in a development build. The desktop app crate is compiled
//! unoptimized while it is being worked on, and this pass touches every byte of
//! every source file in the project — unoptimized it takes seconds instead of
//! a fraction of one. The app crate compiles all of its dependencies, this
//! crate included, at full optimization even in a development build, so putting
//! the loop here means it runs fast whichever way the app was built.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

/// Most symbols one batched margin-count request may ask about. The editor
/// draws at most 120 margin counts per file, so this leaves room to spare.
pub const MAX_REFERENCE_COUNT_SYMBOLS: usize = 256;
/// Most cores one counting pass will use. The pass is bounded by how fast the
/// disk hands over files, so beyond a handful of readers there is nothing left
/// to win — and the rest of the machine has work to do.
pub const MAX_REFERENCE_COUNT_WORKERS: usize = 8;

/// One file a counting pass may read: where it is and how big it is.
///
/// Deliberately plain. The desktop app carries a much richer record per file
/// (display name, language, path relative to the project) and none of it
/// matters here, so the app maps its records down to this on the way in.
pub struct ReferenceCountFile<'a> {
    pub path: &'a Path,
    pub byte_count: u64,
}

/// The symbols one counting pass is looking for, arranged so that a file can be
/// checked for all of them at once.
///
/// Ordinary symbol names are plain identifiers, so they are matched by pulling
/// each identifier out of a line and looking it up — one pass over the text no
/// matter how many symbols were asked about. A name with punctuation in it
/// (rare, but nothing stops the editor asking) cannot be found that way and
/// falls back to searching the line for it directly.
pub struct ReferenceCountPlan<'a> {
    names: &'a [String],
    identifiers: HashMap<&'a str, usize>,
    searched: Vec<(usize, &'a str)>,
}

impl<'a> ReferenceCountPlan<'a> {
    pub fn new(names: &'a [String]) -> Self {
        let mut identifiers = HashMap::new();
        let mut searched = Vec::new();

        for (index, name) in names.iter().enumerate() {
            if name.bytes().all(is_source_identifier_byte) {
                identifiers.insert(name.as_str(), index);
            } else {
                searched.push((index, name.as_str()));
            }
        }

        Self {
            names,
            identifiers,
            searched,
        }
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Running totals for one counting pass.
///
/// A line that mentions a symbol twice still counts once, which is what the
/// reference list the margin count replaces would have reported. `seen_on_line`
/// remembers, per symbol, the last line that already counted for it; line
/// numbers keep climbing across files so there is nothing to reset.
struct ReferenceCountTally {
    counts: Vec<u32>,
    seen_on_line: Vec<u64>,
    lines_read: u64,
}

impl ReferenceCountTally {
    fn new(symbol_count: usize) -> Self {
        Self {
            counts: vec![0; symbol_count],
            seen_on_line: vec![0; symbol_count],
            lines_read: 0,
        }
    }

    fn record(&mut self, symbol_index: usize, line_id: u64) {
        if self.seen_on_line[symbol_index] == line_id {
            return;
        }
        self.seen_on_line[symbol_index] = line_id;
        self.counts[symbol_index] = self.counts[symbol_index].saturating_add(1);
    }
}

/// What one counting pass found: a total per symbol in the order the plan's
/// names were given, how many files were actually read, and whether the pass
/// gave up before reaching the end of the list.
pub struct ReferenceCountPass {
    pub counts: Vec<u32>,
    pub scanned_files: usize,
    pub ran_out_of_time: bool,
}

/// Read every file once and count all of the symbols in it, spread across the
/// machine's cores.
///
/// Reading the files is most of the cost — around three quarters of it on a
/// four-thousand-file project — and it is the part that parallelizes cleanly:
/// each file is counted on its own, and the per-file totals add up. Every
/// worker keeps its own tally so nothing is shared while the pass runs.
///
/// Files bigger than `max_file_bytes` are skipped without being read, the same
/// size ceiling the caller uses for showing a file's contents at all.
pub fn count_reference_lines_across_files(
    files: &[ReferenceCountFile<'_>],
    plan: &ReferenceCountPlan,
    deadline: Instant,
    max_file_bytes: u64,
) -> ReferenceCountPass {
    let worker_count = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .clamp(1, MAX_REFERENCE_COUNT_WORKERS);
    let next_file = AtomicUsize::new(0);
    let ran_out_of_time = AtomicBool::new(false);
    let mut counts = vec![0u32; plan.len()];
    let mut scanned_files = 0;

    std::thread::scope(|scope| {
        let workers = (0..worker_count)
            .map(|_| {
                let next_file = &next_file;
                let ran_out_of_time = &ran_out_of_time;
                scope.spawn(move || {
                    let mut tally = ReferenceCountTally::new(plan.len());
                    let mut scanned = 0;

                    loop {
                        let index = next_file.fetch_add(1, Ordering::Relaxed);
                        let Some(file) = files.get(index) else {
                            break;
                        };
                        // Reading one file costs far more than reading the
                        // clock, so this checks on every file rather than
                        // overshooting the deadline by a batch of them.
                        if Instant::now() >= deadline {
                            ran_out_of_time.store(true, Ordering::Relaxed);
                            break;
                        }

                        if file.byte_count > max_file_bytes {
                            continue;
                        }
                        let Ok(bytes) = std::fs::read(file.path) else {
                            continue;
                        };
                        scanned += 1;
                        count_reference_lines(&String::from_utf8_lossy(&bytes), plan, &mut tally);
                    }

                    (tally.counts, scanned)
                })
            })
            .collect::<Vec<_>>();

        for worker in workers {
            let (worker_counts, worker_scanned) = worker
                .join()
                .unwrap_or_else(|_| (vec![0; plan.len()], 0usize));
            for (total, counted) in counts.iter_mut().zip(worker_counts) {
                *total = total.saturating_add(counted);
            }
            scanned_files += worker_scanned;
        }
    });

    ReferenceCountPass {
        counts,
        scanned_files,
        ran_out_of_time: ran_out_of_time.load(Ordering::Relaxed),
    }
}

/// Trim, drop blanks and repeats, and keep the request to a sane size.
pub fn normalized_reference_count_symbols(symbol_names: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut names = Vec::new();

    for symbol_name in symbol_names {
        let trimmed = symbol_name.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        names.push(trimmed.to_string());
        if names.len() >= MAX_REFERENCE_COUNT_SYMBOLS {
            break;
        }
    }

    names
}

fn count_reference_lines(
    content: &str,
    plan: &ReferenceCountPlan,
    tally: &mut ReferenceCountTally,
) {
    for line in content.lines() {
        tally.lines_read += 1;
        let line_id = tally.lines_read;

        if !plan.identifiers.is_empty() {
            let bytes = line.as_bytes();
            let mut index = 0;
            while index < bytes.len() {
                if !is_source_identifier_byte(bytes[index]) {
                    index += 1;
                    continue;
                }

                let start = index;
                while index < bytes.len() && is_source_identifier_byte(bytes[index]) {
                    index += 1;
                }
                if let Some(&symbol_index) = plan.identifiers.get(&line[start..index]) {
                    tally.record(symbol_index, line_id);
                }
            }
        }

        for &(symbol_index, symbol_name) in &plan.searched {
            if find_case_sensitive_source_reference_column(line, symbol_name).is_some() {
                tally.record(symbol_index, line_id);
            }
        }
    }
}

/// Where in this line the symbol is mentioned as a whole word, matching upper
/// and lower case exactly.
///
/// The app also has a case-insensitive search for the same job; this one exists
/// because margin counts run over the whole project, and lowercasing every line
/// first was allocating a fresh copy of the project's text as it went.
pub fn find_case_sensitive_source_reference_column(line: &str, symbol_name: &str) -> Option<usize> {
    let mut search_start = 0;

    while search_start < line.len() {
        let relative_index = line[search_start..].find(symbol_name)?;
        let index = search_start + relative_index;
        let end_index = index + symbol_name.len();
        if is_source_token_boundary(line, index, end_index) {
            return Some(index);
        }
        search_start = end_index;
    }

    None
}

/// True when the given slice of the line is a whole word rather than part of a
/// longer name — `Detector` inside `FormatDetectorFactory` is not a mention of
/// `Detector`.
pub fn is_source_token_boundary(line: &str, start: usize, end: usize) -> bool {
    let before = if start == 0 {
        None
    } else {
        line[..start].chars().next_back()
    };
    let after = line[end..].chars().next();

    !before.is_some_and(is_source_identifier_character)
        && !after.is_some_and(is_source_identifier_character)
}

fn is_source_identifier_character(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn is_source_identifier_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(content: &str, symbol_names: &[&str]) -> Vec<u32> {
        let names = symbol_names
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>();
        let plan = ReferenceCountPlan::new(&names);
        let mut tally = ReferenceCountTally::new(plan.len());
        count_reference_lines(content, &plan, &mut tally);
        tally.counts
    }

    #[test]
    fn a_line_mentioning_a_symbol_twice_still_counts_once() {
        let counts = count(
            "private readonly FormatDetector _other = new FormatDetector();",
            &["FormatDetector"],
        );

        assert_eq!(counts, vec![1]);
    }

    #[test]
    fn a_symbol_inside_a_longer_name_is_not_a_mention() {
        let counts = count(
            ["var a = new FormatDetectorFactory();", "var b = 1;"]
                .join("\n")
                .as_str(),
            &["FormatDetector"],
        );

        assert_eq!(counts, vec![0]);
    }

    #[test]
    fn counting_matches_upper_and_lower_case_exactly() {
        let counts = count(
            ["var formatdetector = 1;", "var FormatDetector = 2;"]
                .join("\n")
                .as_str(),
            &["FormatDetector"],
        );

        assert_eq!(counts, vec![1]);
    }

    #[test]
    fn a_name_with_punctuation_in_it_is_searched_for_directly() {
        let counts = count(
            ["Format.Detector.Read();", "unrelated();"]
                .join("\n")
                .as_str(),
            &["Format.Detector"],
        );

        assert_eq!(counts, vec![1]);
    }

    #[test]
    fn blank_and_repeated_symbols_are_dropped() {
        let names = normalized_reference_count_symbols(vec![
            "  Detector  ".to_string(),
            "Detector".to_string(),
            "   ".to_string(),
        ]);

        assert_eq!(names, vec!["Detector".to_string()]);
    }
}
