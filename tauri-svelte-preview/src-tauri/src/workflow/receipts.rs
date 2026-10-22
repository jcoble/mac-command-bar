//! What a workflow step reports through `report_result` (TSK-1609): the typed result its `result:` names, the
//! schema the agent is shown, and the checks the engine runs itself before the report is recorded.
#![allow(dead_code)] // Some report fields are only for the schema and the stored report; the checks read the rest.
use super::*;
use rmcp::schemars::{self, JsonSchema};

/// The contract each `result:` value names. `judge` arrives with the proof step.
pub(super) fn contract_for(result: &str) -> Option<WorkflowOutputContract> {
    Some(match result {
        "build" => WorkflowOutputContract::ImplementationReceipt,
        "spec" => WorkflowOutputContract::SpecReceipt,
        "plan" => WorkflowOutputContract::PlanReceipt,
        "recon" => WorkflowOutputContract::ReconReceipt,
        "review" => WorkflowOutputContract::ReviewReceipt,
        "proof" => WorkflowOutputContract::ProofReceipt,
        _ => return None,
    })
}

/// A build step's result.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ImplementationReceipt {
    /// Every commit this step made on the branch; none only when there was nothing to change.
    pub commits: Vec<String>,
    /// Each test added for these commits, by file or name.
    pub tests_added: Vec<String>,
    pub summary: String,
}

/// One thing the change must do, and how it is proved.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AcceptanceItem {
    pub id: String,
    pub text: String,
    /// A failure path or edge case.
    #[serde(default)]
    pub edge: bool,
    /// The fixture an edge item is proved with.
    pub fixture: Option<String>,
    /// True when an edge item is proved by restarting the app.
    pub restart: Option<bool>,
    /// `app`, `test:<path>` or `owner-check`.
    pub proved_by: String,
}

/// A spec step's result.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SpecReceipt {
    /// The spec file: relative to the checkout, or absolute for one written outside it.
    pub path: String,
    pub summary: String,
    pub requirements: Vec<String>,
    /// At least one item is an edge item.
    pub acceptance: Vec<AcceptanceItem>,
    /// Existing code to reuse, each `path:line` in the checkout.
    pub reuse: Vec<String>,
    /// The expected diff size.
    pub budget: String,
    pub assumptions: Vec<String>,
}

/// A recon step's result.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ReconReceipt {
    pub summary: String,
    /// Existing code this task builds on, each `path:line` in the checkout.
    #[serde(default)]
    pub existing: Vec<String>,
    /// Why nothing existing applies, when `existing` is empty.
    pub none_found_because: Option<String>,
    /// The code the change touches, each `path:line` in the checkout.
    pub touches: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PlanSlice {
    pub number: u32,
    pub title: String,
    /// The slice numbers this one waits for.
    pub needs: Vec<u32>,
    pub budget: String,
    pub files: Vec<String>,
    pub acceptance: Vec<AcceptanceItem>,
}

/// A plan step's result.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PlanReceipt {
    pub slices: Vec<PlanSlice>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Verdict { Pass, Fail }

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Severity { Blocking, Minor }

/// How an earlier round's finding stands now.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct EarlierGrade {
    pub text: String,
    /// `fixed` or `open`.
    pub grade: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct Finding {
    pub severity: Severity,
    /// True for a problem that was already there in an earlier round and not raised then.
    pub late: bool,
    pub why_late: Option<String>,
    /// Where it is, `path:line`.
    pub path_line: String,
    pub text: String,
    pub fix: String,
    #[serde(default)]
    pub grades_of_earlier: Vec<EarlierGrade>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct NamedVerdict {
    pub name: String,
    pub verdict: Verdict,
}

/// A review step's result: pass with no findings, or fail with each one.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ReviewReceipt {
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    /// Earlier findings or bot threads accepted as answered.
    #[serde(default)]
    pub accepted: Vec<String>,
    #[serde(default)]
    pub new_names: Vec<NamedVerdict>,
    /// The commit reviewed.
    pub head: String,
}

/// Today's proof step's result, until the proof step reports shots it took through the engine.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ProofReceipt {
    pub summary: String,
    /// Each proof file saved.
    pub files: Vec<String>,
}

fn rejected(message: impl Into<String>) -> WorkflowError {
    WorkflowError::OutputContract(message.into())
}

fn check_acceptance(field: &str, items: &[AcceptanceItem]) -> Result<(), WorkflowError> {
    if !items.iter().any(|item| item.edge) {
        return Err(rejected(format!("{field} needs at least one edge item (a failure path or edge case)")));
    }
    for (index, item) in items.iter().enumerate() {
        if item.edge && item.fixture.as_deref().is_none_or(|fixture| fixture.trim().is_empty()) && item.restart != Some(true) {
            return Err(rejected(format!("{field}[{index}] is an edge item, so it names its fixture or sets restart")));
        }
        let test = item.proved_by.strip_prefix("test:").is_some_and(|path| !path.trim().is_empty());
        if !test && !matches!(item.proved_by.as_str(), "app" | "owner-check") {
            return Err(rejected(format!("{field}[{index}].proved_by must be app, test:<path> or owner-check")));
        }
    }
    Ok(())
}

/// The checks that need nothing but the report itself.
pub(super) fn check_shape(contract: WorkflowOutputContract, value: &Value) -> Result<(), WorkflowError> {
    let parse = |error: serde_json::Error| rejected(format!("{} is invalid: {error}", contract.name()));
    match contract {
        WorkflowOutputContract::SpecReceipt => {
            let spec: SpecReceipt = serde_json::from_value(value.clone()).map_err(parse)?;
            if spec.path.trim().is_empty() || spec.requirements.is_empty() {
                return Err(rejected("SpecReceipt requires a path and requirements"));
            }
            check_acceptance("acceptance", &spec.acceptance)
        }
        WorkflowOutputContract::ReconReceipt => {
            let recon: ReconReceipt = serde_json::from_value(value.clone()).map_err(parse)?;
            if recon.existing.is_empty() && recon.none_found_because.as_deref().is_none_or(|why| why.trim().is_empty()) {
                return Err(rejected("existing lists the code this builds on, or none_found_because says why there is none"));
            }
            Ok(())
        }
        WorkflowOutputContract::PlanReceipt => {
            let plan: PlanReceipt = serde_json::from_value(value.clone()).map_err(parse)?;
            let numbers: Vec<String> = plan.slices.iter().map(|slice| slice.number.to_string()).collect();
            let mut needs = Vec::new();
            for slice in &plan.slices {
                for need in &slice.needs {
                    if !plan.slices.iter().any(|other| other.number == *need) {
                        return Err(rejected(format!("slices: slice {} needs unknown slice {need}", slice.number)));
                    }
                    needs.push((need.to_string(), slice.number.to_string()));
                }
            }
            if !acyclic(numbers.iter().map(String::as_str), needs.iter().map(|(a, b)| (a.as_str(), b.as_str()))) {
                return Err(rejected("slices: the slice graph has a cycle"));
            }
            Ok(())
        }
        WorkflowOutputContract::ReviewReceipt => {
            let review: ReviewReceipt = serde_json::from_value(value.clone()).map_err(parse)?;
            for (index, finding) in review.findings.iter().enumerate() {
                if finding.path_line.trim().is_empty() {
                    return Err(rejected(format!("findings[{index}].path_line is required")));
                }
                if finding.late && finding.why_late.as_deref().is_none_or(|why| why.trim().is_empty()) {
                    return Err(rejected(format!("findings[{index}] is late, so why_late is required")));
                }
            }
            Ok(())
        }
        WorkflowOutputContract::ImplementationReceipt => serde_json::from_value::<ImplementationReceipt>(value.clone()).map(drop).map_err(parse),
        WorkflowOutputContract::ProofReceipt => serde_json::from_value::<ProofReceipt>(value.clone()).map(drop).map_err(parse),
        WorkflowOutputContract::PullRequestReceipt => serde_json::from_value::<PullRequestReceipt>(value.clone()).map(drop).map_err(parse),
        WorkflowOutputContract::PullRequestMergeReceipt => serde_json::from_value::<PullRequestMergeReceipt>(value.clone()).map(drop).map_err(parse),
        WorkflowOutputContract::SpecComplianceReceipt | WorkflowOutputContract::VerificationReceipt =>
            Err(rejected(format!("{} is no longer reported", contract.name()))),
    }
}

fn git(cwd: &str, args: &[&str]) -> Result<std::process::Output, WorkflowError> {
    crate::bounded_process::output(std::process::Command::new("git").args(args).current_dir(cwd), "Report check",
        crate::bounded_process::LOCAL_COMMAND_TIMEOUT).map_err(|error| WorkflowError::Runtime(error.to_string()))
}

/// Each entry is `path:line` naming a line that exists in the checkout.
fn check_lines(cwd: &str, field: &str, entries: &[String]) -> Result<(), WorkflowError> {
    for (index, entry) in entries.iter().enumerate() {
        let missing = || rejected(format!("{field}[{index}]: {entry} is not an existing path:line in the checkout"));
        let (path, line) = entry.rsplit_once(':').ok_or_else(missing)?;
        let line: usize = line.parse().ok().filter(|line| *line > 0).ok_or_else(missing)?;
        let file = contained_file(Path::new(cwd), Path::new(path)).map_err(|_| missing())?;
        if std::fs::read_to_string(file).map_err(|_| missing())?.lines().count() < line { return Err(missing()); }
    }
    Ok(())
}

/// The checks against the checkout. A build's commits must be on the branch and new since the step started.
fn check_evidence(run: &WorkflowRunRecord, node_id: &str, contract: WorkflowOutputContract, value: &Value) -> Result<(), WorkflowError> {
    let cwd = run.input_snapshot["cwd"].as_str().unwrap_or_default();
    match contract {
        WorkflowOutputContract::ImplementationReceipt => {
            let build: ImplementationReceipt = serde_json::from_value(value.clone()).map_err(|error| rejected(error.to_string()))?;
            let start = run.input_snapshot["heads"][node_id].as_str().ok_or_else(|| rejected("commits: this step's starting commit was not recorded"))?;
            for commit in &build.commits {
                if commit.starts_with('-') || !git(cwd, &["merge-base", "--is-ancestor", commit, "HEAD"])?.status.success() {
                    return Err(rejected(format!("commits: {commit} is not on this branch")));
                }
                if git(cwd, &["merge-base", "--is-ancestor", commit, start])?.status.success() {
                    return Err(rejected(format!("commits: {commit} is not new since this step started")));
                }
            }
            // Every commit the step made on the branch must be listed, so an empty list means no commits were made.
            let made = git(cwd, &["rev-list", "--first-parent", &format!("{start}..HEAD")])?;
            for made in String::from_utf8_lossy(&made.stdout).lines() {
                if !build.commits.iter().any(|commit| !commit.is_empty() && made.starts_with(commit.as_str())) {
                    return Err(rejected(format!("commits: {made} was made in this step but is not listed")));
                }
            }
            if !build.commits.is_empty() && build.tests_added.is_empty() {
                return Err(rejected("tests_added is empty; list the tests these commits add"));
            }
            let status = git(cwd, &["status", "--porcelain"])?;
            if !status.status.success() || !status.stdout.is_empty() {
                return Err(rejected(format!("The tree must be clean when the build reports:\n{}", String::from_utf8_lossy(&status.stdout))));
            }
            Ok(())
        }
        WorkflowOutputContract::SpecReceipt => {
            let spec: SpecReceipt = serde_json::from_value(value.clone()).map_err(|error| rejected(error.to_string()))?;
            if !Path::new(cwd).join(&spec.path).is_file() {
                return Err(rejected(format!("path: {} does not exist", spec.path)));
            }
            check_lines(cwd, "reuse", &spec.reuse)
        }
        WorkflowOutputContract::ReconReceipt => {
            let recon: ReconReceipt = serde_json::from_value(value.clone()).map_err(|error| rejected(error.to_string()))?;
            check_lines(cwd, "existing", &recon.existing)?;
            check_lines(cwd, "touches", &recon.touches)
        }
        _ => Ok(()),
    }
}

const REMINDER: &str = "Your step is not finished: call report_result with this step's result. If it is rejected, fix the field it names and call it again. A turn that ends without a recorded report fails the step.";
const NO_REPORT: &str = "The step ended without a valid report_result, after one reminder";

impl WorkflowEngine {
    /// A turn that ended without a recorded report: the session is told once, then the step fails `no-report`.
    pub(super) async fn remind_or_fail(&self, run_id: &str, node_id: &str, key: &str) -> Result<WorkflowRunRecord, WorkflowError> {
        let mut run = self.require_run(run_id)?;
        let node = run.nodes.iter().find(|node| node.node_id == node_id).unwrap();
        let (attempt_id, owned_id) = (json!(node.id), node.owned_id.clone());
        if run.input_snapshot["reminded"].as_array().is_some_and(|reminded| reminded.contains(&attempt_id)) {
            return self.fail_agent_node(run_id, node_id, "no-report", NO_REPORT, key).await;
        }
        let mut reminded = run.input_snapshot["reminded"].as_array().cloned().unwrap_or_default();
        reminded.push(attempt_id);
        run.input_snapshot["reminded"] = json!(reminded);
        self.append(&mut run, "workflow.node.reminded", key, Some(node_id))?;
        self.store.append_message_as("", &format!("workflow:{}", run.id), &owned_id, mcb_core::broker::MessageKind::Message,
            REMINDER, Some(&format!("workflow · {}", run.definition.name))).map_err(|error| WorkflowError::Ledger(error.to_string()))?;
        self.runtime.deliver_messages(&owned_id).await?;
        Ok(run)
    }

    /// Checks a step's report against its contract and the checkout; an error names the failing field.
    pub(crate) fn check_report(&self, run_id: &str, attempt_id: &str, value: &Value) -> Result<(), WorkflowError> {
        let run = self.require_run(run_id)?;
        let node = run.nodes.iter().find(|node| node.id == attempt_id && matches!(node.state, WorkflowNodeState::Starting | WorkflowNodeState::Running))
            .ok_or_else(|| WorkflowError::InvalidTransition("That step is no longer running".into()))?;
        WorkflowPolicy::validate_output(node.output_contract, value)?;
        check_evidence(&run, &node.node_id, node.output_contract, value)
    }
}
