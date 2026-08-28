use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::agent_conversation::manager::AgentRuntimeManager;
use crate::agent_conversation::protocol::{
    AgentConversationProvider, EnsureAgentConversationRequest,
};
use crate::agent_conversation::providers::AgentPrompt;
use crate::orchestration::{append_workflow_event_value, read_workflow_event_values};
use mcb_core::session_store::SessionStore;

pub(crate) const WORKFLOW_DEFINITION_VERSION: u16 = 1;

fn default_definition_version() -> u16 {
    WORKFLOW_DEFINITION_VERSION
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowDefinitionV1 {
    #[serde(default = "default_definition_version")]
    pub version: u16,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub trigger: Value,
    #[serde(default)]
    pub inputs: Vec<Value>,
    #[serde(default)]
    pub roles: Vec<AgentRoleDefinition>,
    #[serde(default)]
    pub nodes: Vec<WorkflowNodeDefinition>,
    #[serde(default)]
    pub edges: Vec<WorkflowEdge>,
    #[serde(default)]
    pub concurrency: WorkflowConcurrencyPolicy,
    #[serde(default)]
    pub budgets: WorkflowBudgetPolicy,
    #[serde(default)]
    pub completion: WorkflowCompletionPolicy,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentRoleDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub provider_policy: ProviderSelectionPolicy,
    #[serde(default)]
    pub model_policy: ConfigSelectionPolicy,
    #[serde(default)]
    pub effort_policy: ConfigSelectionPolicy,
    #[serde(default)]
    pub permission_policy: ConfigSelectionPolicy,
    #[serde(default)]
    pub prompt_template_id: String,
    pub output_contract: WorkflowOutputContract,
    #[serde(default)]
    pub workspace_policy: WorkflowWorkspacePolicy,
    #[serde(default)]
    pub retry_policy: WorkflowRetryPolicy,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNodeDefinition {
    pub id: String,
    pub title: String,
    pub role_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub condition: Option<Value>,
    #[serde(default)]
    pub fan_out: Option<Value>,
    #[serde(default)]
    pub approval_gate: Option<WorkflowApprovalGate>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_attempts")]
    pub max_attempts: u32,
}

fn default_timeout() -> u64 {
    3600
}
fn default_attempts() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowApprovalGate {
    pub id: String,
    #[serde(default)]
    pub prompt: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderSelectionPolicy {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub allowed_providers: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigSelectionPolicy {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum WorkflowWorkspacePolicy {
    ReadOnlyCurrent,
    SharedCurrent {
        #[serde(default)]
        file_allow_list: Vec<String>,
    },
    DedicatedExisting {
        worktree_id: String,
    },
    DedicatedNew {
        branch_template: String,
    },
    None,
}

impl Default for WorkflowWorkspacePolicy {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowRetryPolicy {
    #[serde(default = "default_attempts")]
    pub max_attempts: u32,
    #[serde(default)]
    pub retryable_codes: Vec<String>,
}

impl Default for WorkflowRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            retryable_codes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowConcurrencyPolicy {
    #[serde(default = "default_global_agents")]
    pub global: usize,
    #[serde(default = "default_workflow_agents")]
    pub workflow: usize,
    #[serde(default)]
    pub providers: BTreeMap<String, usize>,
}

fn default_global_agents() -> usize {
    8
}
fn default_workflow_agents() -> usize {
    4
}

impl Default for WorkflowConcurrencyPolicy {
    fn default() -> Self {
        Self {
            global: default_global_agents(),
            workflow: default_workflow_agents(),
            providers: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowBudgetPolicy {
    #[serde(default = "default_global_agents")]
    pub maximum_active_agents: usize,
    #[serde(default = "default_depth")]
    pub maximum_child_depth: u32,
    #[serde(default = "default_attempts")]
    pub maximum_attempts_per_node: u32,
    #[serde(default = "default_wall_time")]
    pub maximum_wall_time_seconds: u64,
    #[serde(default)]
    pub maximum_tokens: Option<u64>,
    #[serde(default = "default_tool_terminals")]
    pub maximum_tool_terminals: u32,
    #[serde(default = "default_worktrees")]
    pub maximum_worktrees: u32,
}

fn default_depth() -> u32 {
    3
}
fn default_wall_time() -> u64 {
    86_400
}
fn default_tool_terminals() -> u32 {
    8
}
fn default_worktrees() -> u32 {
    4
}

impl Default for WorkflowBudgetPolicy {
    fn default() -> Self {
        Self {
            maximum_active_agents: default_global_agents(),
            maximum_child_depth: default_depth(),
            maximum_attempts_per_node: default_attempts(),
            maximum_wall_time_seconds: default_wall_time(),
            maximum_tokens: None,
            maximum_tool_terminals: default_tool_terminals(),
            maximum_worktrees: default_worktrees(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowCompletionPolicy {
    #[serde(default)]
    pub cancel_descendants_on_failure: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) enum WorkflowOutputContract {
    ImplementationReceipt,
    ReviewReceipt,
    SpecComplianceReceipt,
    VerificationReceipt,
    PlanReceipt,
}

impl WorkflowOutputContract {
    fn name(self) -> &'static str {
        match self {
            Self::ImplementationReceipt => "ImplementationReceipt",
            Self::ReviewReceipt => "ReviewReceipt",
            Self::SpecComplianceReceipt => "SpecComplianceReceipt",
            Self::VerificationReceipt => "VerificationReceipt",
            Self::PlanReceipt => "PlanReceipt",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum WorkflowRunState {
    Draft,
    Queued,
    Running,
    WaitingApproval,
    WaitingInput,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum WorkflowNodeState {
    Blocked,
    Ready,
    Queued,
    Starting,
    Running,
    WaitingApproval,
    WaitingInput,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum WorkflowLoopPhase {
    Draft,
    Scheduling,
    Implementing,
    Reviewing,
    Verifying,
    Approval,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowArtifactRef {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowGateRecord {
    pub id: String,
    pub node_run_id: String,
    pub approved: Option<bool>,
    pub structured_input: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNodeRunRecord {
    pub id: String,
    pub node_id: String,
    pub role_id: String,
    pub state: WorkflowNodeState,
    pub attempt: u32,
    pub depth: u32,
    pub owned_id: String,
    pub provider: String,
    pub provider_instance_id: Option<String>,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
    pub output_contract: WorkflowOutputContract,
    pub structured_output: Option<Value>,
    pub artifacts: Vec<WorkflowArtifactRef>,
    pub gate: Option<WorkflowGateRecord>,
    pub lease_id: Option<String>,
    pub failure: Option<WorkflowFailure>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowRunRecord {
    pub id: String,
    pub workflow_id: String,
    pub definition: WorkflowDefinitionV1,
    pub state: WorkflowRunState,
    pub phase: WorkflowLoopPhase,
    pub input_snapshot: Value,
    pub input_hash: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub nodes: Vec<WorkflowNodeRunRecord>,
    pub tokens_used: u64,
    pub tool_terminals_used: u32,
    pub worktrees_allocated: u32,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowFailure {
    pub code: String,
    pub message: String,
    pub budget: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImplementationReceipt {
    pub changed_files: Vec<String>,
    pub summary: String,
    pub tests: Vec<Value>,
    pub known_risks: Vec<String>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub worktree: Option<String>,
    pub artifacts: Vec<WorkflowArtifactRef>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewReceipt {
    pub severity: String,
    pub file: String,
    pub line: u64,
    pub evidence: String,
    pub recommendation: String,
    pub confidence: f64,
    pub blocking: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpecComplianceReceipt {
    pub requirement_id: String,
    pub status: String,
    pub evidence: Value,
    pub gap: Option<String>,
    pub recommended_action: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerificationReceipt {
    pub command: String,
    pub exit: i32,
    pub duration: u64,
    pub evidence_artifacts: Vec<WorkflowArtifactRef>,
    pub cleanup_receipt: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanReceipt {
    pub ordered_steps: Vec<Value>,
    pub dependencies: Vec<Value>,
    pub risk: Value,
    pub estimated_parallel_lanes: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "detail", rename_all = "kebab-case")]
pub(crate) enum WorkflowError {
    InvalidDefinition(String),
    NotFound(String),
    InvalidTransition(String),
    PolicyDenied(String),
    LeaseConflict(String),
    OutputContract(String),
    BudgetExhausted(WorkflowFailure),
    Runtime(String),
    Ledger(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            serde_json::to_string(self).unwrap_or_else(|_| format!("{self:?}"))
        )
    }
}

type PortFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, WorkflowError>> + Send + 'a>>;

#[derive(Debug, Clone)]
pub(crate) struct AgentDispatchRequest {
    pub owned_id: String,
    pub provider: String,
    pub cwd: String,
    pub prompt: String,
    pub role_id: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub permission: Option<String>,
    pub output_contract: WorkflowOutputContract,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentDispatchReceipt {
    pub provider_instance_id: String,
}

pub(crate) trait AgentRuntimePort: Send + Sync {
    fn dispatch<'a>(
        &'a self,
        request: AgentDispatchRequest,
    ) -> PortFuture<'a, AgentDispatchReceipt>;
    fn cancel<'a>(&'a self, owned_id: &'a str) -> PortFuture<'a, ()>;
}

impl AgentRuntimePort for AgentRuntimeManager {
    fn dispatch<'a>(
        &'a self,
        request: AgentDispatchRequest,
    ) -> PortFuture<'a, AgentDispatchReceipt> {
        Box::pin(async move {
            let provider = match request.provider.to_ascii_lowercase().as_str() {
                "codex" => AgentConversationProvider::Codex,
                "claude" => AgentConversationProvider::Claude,
                "antigravity" => AgentConversationProvider::Antigravity,
                other => {
                    return Err(WorkflowError::PolicyDenied(format!(
                        "Unsupported provider {other}"
                    )))
                }
            };
            let spawn_reasoning_effort = (provider == AgentConversationProvider::Claude)
                .then(|| request.effort.clone())
                .flatten();
            let connection = self
                .ensure_async(EnsureAgentConversationRequest {
                    owned_id: request.owned_id.clone(),
                    execution_environment:
                        crate::agent_conversation::protocol::ExecutionEnvironment::Local,
                    provider,
                    cwd: request.cwd.clone(),
                    native_session_id: None,
                    native_session_mode:
                        crate::agent_conversation::protocol::AgentNativeSessionMode::Resume,
                    reasoning_effort: spawn_reasoning_effort,
                })
                .await
                .map_err(WorkflowError::Runtime)?;
            if self.providers().manifest(provider).is_err() {
                return Err(WorkflowError::Runtime(format!(
                    "Provider {} is not installed",
                    request.provider
                )));
            }
            self.activate(&connection.owned_id, connection.generation)
                .await
                .map_err(WorkflowError::Runtime)?;
            let capabilities = self
                .capabilities(&connection.owned_id, connection.generation)
                .map_err(WorkflowError::Runtime)?;
            for (categories, selected) in [
                (&["model"][..], request.model.as_deref()),
                (&["thought_level", "effort"][..], request.effort.as_deref()),
                (
                    &["mode", "permission", "permissions"][..],
                    request.permission.as_deref(),
                ),
            ] {
                if provider == AgentConversationProvider::Claude
                    && categories.contains(&"thought_level")
                {
                    continue;
                }
                if let Some(value) = selected {
                    let option = capabilities
                        .config_options
                        .iter()
                        .find(|option| {
                            categories
                                .iter()
                                .any(|category| option.category.eq_ignore_ascii_case(category))
                        })
                        .ok_or_else(|| {
                            WorkflowError::PolicyDenied(format!(
                                "Provider {} did not advertise the requested {} capability",
                                request.provider, categories[0]
                            ))
                        })?;
                    self.set_config(
                        &connection.owned_id,
                        connection.generation,
                        &option.id,
                        json!(value),
                    )
                    .await
                    .map_err(WorkflowError::Runtime)?;
                }
            }
            let envelope = json!({
                "roleId": request.role_id, "model": request.model, "effort": request.effort,
                "permission": request.permission, "outputContract": request.output_contract.name(),
                "task": request.prompt
            });
            self.prompt(
                &connection.owned_id,
                connection.generation,
                AgentPrompt {
                    text: envelope.to_string(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                },
            )
            .await
            .map_err(WorkflowError::Runtime)?;
            Ok(AgentDispatchReceipt {
                provider_instance_id: format!("{}-{}", request.provider, connection.generation),
            })
        })
    }

    fn cancel<'a>(&'a self, owned_id: &'a str) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let Some(snapshot) = self.snapshot(owned_id).map_err(WorkflowError::Runtime)? else {
                return Ok(());
            };
            self.cancel_turn(owned_id, snapshot.connection.generation)
                .await
                .map_err(WorkflowError::Runtime)
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WorktreeLeaseRequest {
    pub run_id: String,
    pub node_run_id: String,
    pub workspace: WorkflowWorkspacePolicy,
    pub cwd: String,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorktreeLeaseReceipt {
    pub lease_id: String,
    pub cwd: String,
}

pub(crate) trait WorktreeLeasePort: Send + Sync {
    fn acquire(&self, request: WorktreeLeaseRequest)
        -> Result<WorktreeLeaseReceipt, WorkflowError>;
    fn release(&self, lease_id: &str) -> Result<(), WorkflowError>;
    fn rebuild(&self, active: Vec<WorktreeLeaseRequest>) -> Result<(), WorkflowError>;
}

#[derive(Default)]
pub(crate) struct InMemoryWorktreeLeasePort {
    leases: Mutex<HashMap<String, WorktreeLeaseRequest>>,
}

impl WorktreeLeasePort for InMemoryWorktreeLeasePort {
    fn acquire(
        &self,
        request: WorktreeLeaseRequest,
    ) -> Result<WorktreeLeaseReceipt, WorkflowError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| WorkflowError::LeaseConflict("Lease authority unavailable".into()))?;
        if matches!(
            request.workspace,
            WorkflowWorkspacePolicy::DedicatedNew { .. }
        ) {
            return Err(WorkflowError::PolicyDenied("Dedicated-new must be allocated by the existing worktree authority before dispatch".into()));
        }
        let writes = !matches!(
            request.workspace,
            WorkflowWorkspacePolicy::ReadOnlyCurrent | WorkflowWorkspacePolicy::None
        );
        if writes
            && leases.values().any(|held| {
                held.cwd == request.cwd && paths_overlap(&held.file_paths, &request.file_paths)
            })
        {
            return Err(WorkflowError::LeaseConflict(format!(
                "Overlapping file lease in {}",
                request.cwd
            )));
        }
        let lease_id = format!(
            "lease-{}",
            stable_hash(&format!(
                "{}:{}:{}",
                request.run_id, request.node_run_id, request.cwd
            ))
        );
        leases.entry(lease_id.clone()).or_insert(request.clone());
        Ok(WorktreeLeaseReceipt {
            lease_id,
            cwd: request.cwd,
        })
    }

    fn release(&self, lease_id: &str) -> Result<(), WorkflowError> {
        self.leases
            .lock()
            .map_err(|_| WorkflowError::LeaseConflict("Lease authority unavailable".into()))?
            .remove(lease_id);
        Ok(())
    }

    fn rebuild(&self, active: Vec<WorktreeLeaseRequest>) -> Result<(), WorkflowError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| WorkflowError::LeaseConflict("Lease authority unavailable".into()))?;
        for request in active {
            let writes = !matches!(
                request.workspace,
                WorkflowWorkspacePolicy::ReadOnlyCurrent | WorkflowWorkspacePolicy::None
            );
            let lease_id = format!(
                "lease-{}",
                stable_hash(&format!(
                    "{}:{}:{}",
                    request.run_id, request.node_run_id, request.cwd
                ))
            );
            if leases.contains_key(&lease_id) {
                continue;
            }
            if writes
                && leases.values().any(|held| {
                    held.cwd == request.cwd && paths_overlap(&held.file_paths, &request.file_paths)
                })
            {
                return Err(WorkflowError::LeaseConflict(format!(
                    "Persisted overlapping file lease in {}",
                    request.cwd
                )));
            }
            leases.insert(lease_id, request);
        }
        Ok(())
    }
}

fn paths_overlap(left: &[String], right: &[String]) -> bool {
    left.is_empty()
        || right.is_empty()
        || left.iter().any(|a| {
            right.iter().any(|b| {
                a == b || a.starts_with(&format!("{b}/")) || b.starts_with(&format!("{a}/"))
            })
        })
}

pub(crate) trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}
pub(crate) trait IdGenerator: Send + Sync {
    fn next_id(&self, prefix: &str) -> String;
}

#[derive(Default)]
pub(crate) struct SystemClock;
impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or_default()
    }
}

#[derive(Default)]
pub(crate) struct UuidGenerator;
impl IdGenerator for UuidGenerator {
    fn next_id(&self, prefix: &str) -> String {
        format!("{prefix}-{}", uuid::Uuid::new_v4())
    }
}

pub(crate) struct WorkflowReducer;

impl WorkflowReducer {
    pub fn reduce(events: Vec<Value>) -> Result<Vec<WorkflowRunRecord>, WorkflowError> {
        let mut ordered: Vec<(u64, String, WorkflowRunRecord)> = events
            .into_iter()
            .filter_map(|event| {
                let sequence = event.get("sequence")?.as_u64()?;
                let run_id = event.get("runId")?.as_str()?.to_string();
                let run = serde_json::from_value(event.get("workflowPayload")?.get("run")?.clone())
                    .ok()?;
                Some((sequence, run_id, run))
            })
            .collect();
        ordered.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        let mut runs = BTreeMap::new();
        for (sequence, run_id, mut run) in ordered {
            run.last_sequence = sequence;
            runs.insert(run_id, run);
        }
        Ok(runs.into_values().collect())
    }
}

pub(crate) struct WorkflowScheduler;
impl WorkflowScheduler {
    pub fn ready_nodes(run: &WorkflowRunRecord) -> Vec<String> {
        let states: HashMap<_, _> = run
            .nodes
            .iter()
            .map(|node| (node.node_id.as_str(), node.state))
            .collect();
        let mut ready: Vec<_> = run
            .definition
            .nodes
            .iter()
            .filter(|definition| {
                let mut dependencies = definition.depends_on.clone();
                dependencies.extend(
                    run.definition
                        .edges
                        .iter()
                        .filter(|edge| edge.to == definition.id)
                        .map(|edge| edge.from.clone()),
                );
                states.get(definition.id.as_str()) == Some(&WorkflowNodeState::Blocked)
                    && dependencies.iter().all(|dependency| {
                        matches!(
                            states.get(dependency.as_str()),
                            Some(WorkflowNodeState::Completed | WorkflowNodeState::Skipped)
                        )
                    })
            })
            .map(|node| node.id.clone())
            .collect();
        ready.sort();
        ready
    }
}

pub(crate) struct WorkflowPolicy;
impl WorkflowPolicy {
    pub fn validate_definition(definition: &WorkflowDefinitionV1) -> Result<(), WorkflowError> {
        if definition.version != WORKFLOW_DEFINITION_VERSION {
            return Err(WorkflowError::InvalidDefinition(
                "Only workflow definition version 1 is supported".into(),
            ));
        }
        if definition.id.trim().is_empty() || definition.nodes.is_empty() {
            return Err(WorkflowError::InvalidDefinition(
                "Workflow id and at least one node are required".into(),
            ));
        }
        let roles: HashSet<_> = definition
            .roles
            .iter()
            .map(|role| role.id.as_str())
            .collect();
        if roles.len() != definition.roles.len() {
            return Err(WorkflowError::InvalidDefinition(
                "Role ids must be unique".into(),
            ));
        }
        let nodes: HashSet<_> = definition
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        if nodes.len() != definition.nodes.len() {
            return Err(WorkflowError::InvalidDefinition(
                "Node ids must be unique".into(),
            ));
        }
        for node in &definition.nodes {
            if !roles.contains(node.role_id.as_str()) {
                return Err(WorkflowError::InvalidDefinition(format!(
                    "Node {} references unknown role {}",
                    node.id, node.role_id
                )));
            }
            if node
                .depends_on
                .iter()
                .any(|dependency| !nodes.contains(dependency.as_str()))
            {
                return Err(WorkflowError::InvalidDefinition(format!(
                    "Node {} has an unknown dependency",
                    node.id
                )));
            }
            if node
                .depends_on
                .iter()
                .any(|dependency| dependency == &node.id)
            {
                return Err(WorkflowError::InvalidDefinition(format!(
                    "Node {} depends on itself",
                    node.id
                )));
            }
        }
        for edge in &definition.edges {
            if !nodes.contains(edge.from.as_str()) || !nodes.contains(edge.to.as_str()) {
                return Err(WorkflowError::InvalidDefinition(
                    "Edge references an unknown node".into(),
                ));
            }
        }
        let mut incoming: HashMap<&str, usize> = definition
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), 0))
            .collect();
        let mut children: HashMap<&str, BTreeSet<&str>> = HashMap::new();
        for node in &definition.nodes {
            for dependency in &node.depends_on {
                if children.entry(dependency).or_default().insert(&node.id) {
                    *incoming.get_mut(node.id.as_str()).unwrap() += 1;
                }
            }
        }
        for edge in &definition.edges {
            if children.entry(&edge.from).or_default().insert(&edge.to) {
                *incoming.get_mut(edge.to.as_str()).unwrap() += 1;
            }
        }
        let mut queue: BTreeSet<&str> = incoming
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0;
        while let Some(id) = queue.pop_first() {
            visited += 1;
            if let Some(next) = children.get(id) {
                for child in next {
                    let count = incoming.get_mut(child).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        queue.insert(child);
                    }
                }
            }
        }
        if visited != definition.nodes.len() {
            return Err(WorkflowError::InvalidDefinition(
                "Workflow graph contains a cycle".into(),
            ));
        }
        Ok(())
    }

    pub fn validate_output(
        contract: WorkflowOutputContract,
        value: &Value,
    ) -> Result<(), WorkflowError> {
        value.as_object().ok_or_else(|| {
            WorkflowError::OutputContract(format!(
                "{} must be a structured object; prose is never control input",
                contract.name()
            ))
        })?;
        let parsed = match contract {
            WorkflowOutputContract::ImplementationReceipt => {
                serde_json::from_value::<ImplementationReceipt>(value.clone()).map(|_| ())
            }
            WorkflowOutputContract::ReviewReceipt => {
                serde_json::from_value::<ReviewReceipt>(value.clone()).map(|_| ())
            }
            WorkflowOutputContract::SpecComplianceReceipt => {
                serde_json::from_value::<SpecComplianceReceipt>(value.clone()).map(|_| ())
            }
            WorkflowOutputContract::VerificationReceipt => {
                serde_json::from_value::<VerificationReceipt>(value.clone()).map(|_| ())
            }
            WorkflowOutputContract::PlanReceipt => {
                serde_json::from_value::<PlanReceipt>(value.clone()).map(|_| ())
            }
        };
        parsed.map_err(|error| {
            WorkflowError::OutputContract(format!(
                "{} is invalid structured data: {error}",
                contract.name()
            ))
        })
    }

    fn provider_for(role: &AgentRoleDefinition) -> Result<String, WorkflowError> {
        let provider = role.provider_policy.provider.trim().to_ascii_lowercase();
        if provider.is_empty() {
            return Err(WorkflowError::PolicyDenied(format!(
                "Role {} has no provider",
                role.id
            )));
        }
        if !role.provider_policy.allowed_providers.is_empty()
            && !role
                .provider_policy
                .allowed_providers
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(&provider))
        {
            return Err(WorkflowError::PolicyDenied(format!(
                "Provider {provider} is not allowed for role {}",
                role.id
            )));
        }
        Ok(provider)
    }
}

pub(crate) struct WorkflowEngine {
    runtime: Arc<dyn AgentRuntimePort>,
    leases: Arc<dyn WorktreeLeasePort>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    store: Arc<SessionStore>,
}

impl WorkflowEngine {
    pub fn new(
        runtime: Arc<dyn AgentRuntimePort>,
        leases: Arc<dyn WorktreeLeasePort>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        store: Arc<SessionStore>,
    ) -> Self {
        Self {
            runtime,
            leases,
            clock,
            ids,
            store,
        }
    }

    pub fn managed(runtime: AgentRuntimeManager, store: Arc<SessionStore>) -> Self {
        Self::new(
            Arc::new(runtime),
            Arc::new(InMemoryWorktreeLeasePort::default()),
            Arc::new(SystemClock),
            Arc::new(UuidGenerator),
            store,
        )
    }

    pub fn create_run(
        &self,
        definition: WorkflowDefinitionV1,
        input: Value,
        idempotency_key: String,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        WorkflowPolicy::validate_definition(&definition)?;
        if let Some(existing) = self.idempotent_workflow_run(&definition.id, &idempotency_key)? {
            return Ok(existing);
        }
        let canonical = canonical_json(&input);
        let input_hash = stable_hash(&canonical);
        let now = self.clock.now_ms();
        let run_id = self.ids.next_id("workflow-run");
        let mut nodes: Vec<_> = definition
            .nodes
            .iter()
            .map(|node| {
                let role = definition
                    .roles
                    .iter()
                    .find(|role| role.id == node.role_id)
                    .expect("validated role");
                WorkflowNodeRunRecord {
                    id: format!("{}:{}:1", run_id, node.id),
                    node_id: node.id.clone(),
                    role_id: node.role_id.clone(),
                    state: WorkflowNodeState::Blocked,
                    attempt: 1,
                    depth: 0,
                    owned_id: self.ids.next_id("workflow-agent"),
                    provider: role.provider_policy.provider.to_ascii_lowercase(),
                    provider_instance_id: None,
                    started_at_ms: None,
                    finished_at_ms: None,
                    output_contract: role.output_contract,
                    structured_output: None,
                    artifacts: Vec::new(),
                    gate: None,
                    lease_id: None,
                    failure: None,
                }
            })
            .collect();
        nodes.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        let mut run = WorkflowRunRecord {
            id: run_id,
            workflow_id: definition.id.clone(),
            definition,
            state: WorkflowRunState::Draft,
            phase: WorkflowLoopPhase::Draft,
            input_snapshot: input,
            input_hash,
            created_at_ms: now,
            updated_at_ms: now,
            nodes,
            tokens_used: 0,
            tool_terminals_used: 0,
            worktrees_allocated: 0,
            last_sequence: 0,
        };
        self.append(&mut run, "workflow.run.created", &idempotency_key, None)?;
        Ok(run)
    }

    pub async fn start(
        &self,
        run_id: &str,
        idempotency_key: &str,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let mut run = self.require_run(run_id)?;
        if !matches!(
            run.state,
            WorkflowRunState::Draft | WorkflowRunState::Queued
        ) {
            return Ok(run);
        }
        run.state = WorkflowRunState::Running;
        refresh_ready_and_phase(&mut run);
        self.append(&mut run, "workflow.run.started", idempotency_key, None)?;
        self.dispatch_ready(run_id).await
    }

    pub fn pause(
        &self,
        run_id: &str,
        idempotency_key: &str,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let mut run = self.require_run(run_id)?;
        if run.state == WorkflowRunState::Paused {
            return Ok(run);
        }
        if run.state != WorkflowRunState::Running {
            return Err(WorkflowError::InvalidTransition(
                "Only a running workflow can be paused".into(),
            ));
        }
        run.state = WorkflowRunState::Paused;
        self.append(&mut run, "workflow.run.paused", idempotency_key, None)?;
        Ok(run)
    }

    pub async fn resume(
        &self,
        run_id: &str,
        idempotency_key: &str,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let mut run = self.require_run(run_id)?;
        if run.state == WorkflowRunState::Running {
            return Ok(run);
        }
        if run.state != WorkflowRunState::Paused {
            return Err(WorkflowError::InvalidTransition(
                "Only a paused workflow can be resumed".into(),
            ));
        }
        run.state = WorkflowRunState::Running;
        refresh_ready_and_phase(&mut run);
        self.append(&mut run, "workflow.run.resumed", idempotency_key, None)?;
        self.dispatch_ready(run_id).await
    }

    pub async fn cancel(
        &self,
        run_id: &str,
        idempotency_key: &str,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let mut run = self.require_run(run_id)?;
        if run.state == WorkflowRunState::Cancelled {
            return Ok(run);
        }
        for node in &mut run.nodes {
            if matches!(
                node.state,
                WorkflowNodeState::Running
                    | WorkflowNodeState::Starting
                    | WorkflowNodeState::Queued
            ) {
                self.runtime.cancel(&node.owned_id).await?;
            }
            if !matches!(
                node.state,
                WorkflowNodeState::Completed | WorkflowNodeState::Skipped
            ) {
                node.state = WorkflowNodeState::Cancelled;
                node.finished_at_ms = Some(self.clock.now_ms());
            }
            if let Some(lease_id) = node.lease_id.take() {
                self.leases.release(&lease_id)?;
            }
        }
        run.state = WorkflowRunState::Cancelled;
        run.phase = WorkflowLoopPhase::Cancelled;
        self.append(&mut run, "workflow.run.cancelled", idempotency_key, None)?;
        Ok(run)
    }

    pub fn retry_node(
        &self,
        run_id: &str,
        node_id: &str,
        idempotency_key: &str,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let mut run = self.require_run(run_id)?;
        let definition = run
            .definition
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| WorkflowError::NotFound(node_id.into()))?
            .clone();
        let role = run
            .definition
            .roles
            .iter()
            .find(|role| {
                role.id
                    == run
                        .nodes
                        .iter()
                        .find(|node| node.node_id == node_id)
                        .unwrap()
                        .role_id
            })
            .unwrap()
            .clone();
        let max = definition
            .max_attempts
            .min(run.definition.budgets.maximum_attempts_per_node)
            .min(role.retry_policy.max_attempts)
            .max(1);
        let node = run
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .unwrap();
        if node.state != WorkflowNodeState::Failed {
            return Err(WorkflowError::InvalidTransition(
                "Only a failed node can be retried".into(),
            ));
        }
        if !role.retry_policy.retryable_codes.is_empty()
            && !node.failure.as_ref().is_some_and(|failure| {
                role.retry_policy
                    .retryable_codes
                    .iter()
                    .any(|code| code == &failure.code)
            })
        {
            return Err(WorkflowError::PolicyDenied(
                "Node failure is not retryable by role policy".into(),
            ));
        }
        if node.attempt >= max {
            return Err(WorkflowError::BudgetExhausted(budget_failure(
                "attempts",
                "Node attempt budget exhausted",
            )));
        }
        node.attempt += 1;
        node.id = format!("{}:{}:{}", run.id, node.node_id, node.attempt);
        node.owned_id = self.ids.next_id("workflow-agent");
        node.state = WorkflowNodeState::Ready;
        node.failure = None;
        node.structured_output = None;
        node.finished_at_ms = None;
        run.state = WorkflowRunState::Running;
        refresh_ready_and_phase(&mut run);
        self.append(
            &mut run,
            "workflow.node.retried",
            idempotency_key,
            Some(node_id),
        )?;
        Ok(run)
    }

    pub fn skip_node(
        &self,
        run_id: &str,
        node_id: &str,
        idempotency_key: &str,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let mut run = self.require_run(run_id)?;
        let node = run
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .ok_or_else(|| WorkflowError::NotFound(node_id.into()))?;
        if matches!(
            node.state,
            WorkflowNodeState::Completed | WorkflowNodeState::Running | WorkflowNodeState::Starting
        ) {
            return Err(WorkflowError::InvalidTransition(
                "Active or completed nodes cannot be skipped".into(),
            ));
        }
        node.state = WorkflowNodeState::Skipped;
        node.finished_at_ms = Some(self.clock.now_ms());
        refresh_ready_and_phase(&mut run);
        self.append(
            &mut run,
            "workflow.node.skipped",
            idempotency_key,
            Some(node_id),
        )?;
        Ok(run)
    }

    pub fn approve_gate(
        &self,
        run_id: &str,
        node_id: &str,
        approval: Value,
        idempotency_key: &str,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let approved = approval
            .get("approved")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                WorkflowError::OutputContract(
                    "Gate input must be structured data containing approved: boolean".into(),
                )
            })?;
        let mut run = self.require_run(run_id)?;
        let node = run
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .ok_or_else(|| WorkflowError::NotFound(node_id.into()))?;
        if node.state != WorkflowNodeState::WaitingApproval {
            return Err(WorkflowError::InvalidTransition(
                "Node is not waiting for approval".into(),
            ));
        }
        let gate = node
            .gate
            .as_mut()
            .ok_or_else(|| WorkflowError::InvalidTransition("Node has no gate".into()))?;
        gate.approved = Some(approved);
        gate.structured_input = Some(approval);
        node.state = if approved {
            WorkflowNodeState::Completed
        } else {
            WorkflowNodeState::Failed
        };
        node.finished_at_ms = Some(self.clock.now_ms());
        if !approved && run.definition.completion.cancel_descendants_on_failure {
            cancel_descendants(&mut run, node_id);
        }
        refresh_ready_and_phase(&mut run);
        self.append(
            &mut run,
            "workflow.gate.decided",
            idempotency_key,
            Some(node_id),
        )?;
        Ok(run)
    }

    pub async fn submit_result(
        &self,
        run_id: &str,
        node_id: &str,
        result: Value,
        idempotency_key: &str,
    ) -> Result<WorkflowRunRecord, WorkflowError> {
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let mut run = self.require_run(run_id)?;
        let index = run
            .nodes
            .iter()
            .position(|node| node.node_id == node_id)
            .ok_or_else(|| WorkflowError::NotFound(node_id.into()))?;
        if run.nodes[index].state == WorkflowNodeState::Completed {
            return Ok(run);
        }
        if run.nodes[index].state != WorkflowNodeState::Running {
            return Err(WorkflowError::InvalidTransition(
                "Only a running node accepts a result".into(),
            ));
        }
        enforce_node_timeout(&run, index, self.clock.now_ms())?;
        WorkflowPolicy::validate_output(run.nodes[index].output_contract, &result)?;
        run.tokens_used = run.tokens_used.saturating_add(
            result
                .get("tokensUsed")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
        );
        run.tool_terminals_used = run.tool_terminals_used.saturating_add(
            result
                .get("toolTerminalsUsed")
                .and_then(Value::as_u64)
                .unwrap_or_default() as u32,
        );
        if let Err(error) = enforce_budgets(&run, self.clock.now_ms()) {
            if let WorkflowError::BudgetExhausted(failure) = &error {
                run.nodes[index].state = WorkflowNodeState::Failed;
                run.nodes[index].failure = Some(failure.clone());
                run.nodes[index].finished_at_ms = Some(self.clock.now_ms());
                if run.definition.completion.cancel_descendants_on_failure {
                    cancel_descendants(&mut run, node_id);
                }
                run.state = WorkflowRunState::Failed;
                run.phase = WorkflowLoopPhase::Failed;
                let budget_key = format!(
                    "budget:{}:{}",
                    run.nodes[index].id,
                    failure.budget.as_deref().unwrap_or("unknown")
                );
                self.append(
                    &mut run,
                    "workflow.node.budget-exhausted",
                    &budget_key,
                    Some(node_id),
                )?;
            }
            return Err(error);
        }
        let artifacts = result
            .get("artifacts")
            .or_else(|| result.get("evidenceArtifacts"))
            .cloned()
            .unwrap_or_else(|| json!([]));
        run.nodes[index].artifacts = serde_json::from_value(artifacts).unwrap_or_default();
        run.nodes[index].structured_output = Some(result);
        if let Some(gate_definition) = run
            .definition
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .and_then(|node| node.approval_gate.clone())
        {
            run.nodes[index].state = WorkflowNodeState::WaitingApproval;
            run.nodes[index].gate = Some(WorkflowGateRecord {
                id: gate_definition.id,
                node_run_id: run.nodes[index].id.clone(),
                approved: None,
                structured_input: None,
            });
            run.state = WorkflowRunState::WaitingApproval;
        } else {
            run.nodes[index].state = WorkflowNodeState::Completed;
            run.nodes[index].finished_at_ms = Some(self.clock.now_ms());
            if let Some(lease_id) = run.nodes[index].lease_id.take() {
                self.leases.release(&lease_id)?;
            }
        }
        refresh_ready_and_phase(&mut run);
        self.append(
            &mut run,
            "workflow.node.result-submitted",
            idempotency_key,
            Some(node_id),
        )?;
        self.dispatch_ready(run_id).await
    }

    pub async fn dispatch_ready(&self, run_id: &str) -> Result<WorkflowRunRecord, WorkflowError> {
        loop {
            let mut run = self.require_run(run_id)?;
            if run.state != WorkflowRunState::Running {
                return Ok(run);
            }
            if let Err(error) = enforce_budgets(&run, self.clock.now_ms()) {
                if let WorkflowError::BudgetExhausted(failure) = &error {
                    run.state = WorkflowRunState::Failed;
                    run.phase = WorkflowLoopPhase::Failed;
                    let budget_key = format!(
                        "budget:{}:{}",
                        run.id,
                        failure.budget.as_deref().unwrap_or("unknown")
                    );
                    self.append(&mut run, "workflow.run.budget-exhausted", &budget_key, None)?;
                }
                return Err(error);
            }
            if let Some(index) = run.nodes.iter().enumerate().find_map(|(index, node)| {
                (node.state == WorkflowNodeState::Running
                    && enforce_node_timeout(&run, index, self.clock.now_ms()).is_err())
                .then_some(index)
            }) {
                let failure = budget_failure("node-timeout", "Workflow node timeout exhausted");
                run.nodes[index].state = WorkflowNodeState::Failed;
                run.nodes[index].failure = Some(failure.clone());
                run.nodes[index].finished_at_ms = Some(self.clock.now_ms());
                if let Some(lease_id) = run.nodes[index].lease_id.take() {
                    self.leases.release(&lease_id)?;
                }
                let node_id = run.nodes[index].node_id.clone();
                if run.definition.completion.cancel_descendants_on_failure {
                    cancel_descendants(&mut run, &node_id);
                }
                refresh_ready_and_phase(&mut run);
                let timeout_key = format!(
                    "timeout:{}:{}",
                    run.nodes[index].id, run.nodes[index].attempt
                );
                self.append(
                    &mut run,
                    "workflow.node.timed-out",
                    &timeout_key,
                    Some(&node_id),
                )?;
                return Err(WorkflowError::BudgetExhausted(failure));
            }
            refresh_ready_and_phase(&mut run);
            let Some(node_id) = run
                .nodes
                .iter()
                .filter(|node| node.state == WorkflowNodeState::Ready)
                .map(|node| node.node_id.clone())
                .min()
            else {
                let settled_key = format!("settled:{}:{}", run_id, run.last_sequence);
                self.append(&mut run, "workflow.scheduler.settled", &settled_key, None)?;
                return Ok(run);
            };
            let all_runs = self.list_runs()?;
            self.rebuild_active_leases(&all_runs)?;
            let global_active = all_runs
                .iter()
                .flat_map(|candidate| &candidate.nodes)
                .filter(|node| {
                    matches!(
                        node.state,
                        WorkflowNodeState::Starting | WorkflowNodeState::Running
                    )
                })
                .count();
            let workflow_active = run
                .nodes
                .iter()
                .filter(|node| {
                    matches!(
                        node.state,
                        WorkflowNodeState::Starting | WorkflowNodeState::Running
                    )
                })
                .count();
            let index = run
                .nodes
                .iter()
                .position(|node| node.node_id == node_id)
                .unwrap();
            let role = run
                .definition
                .roles
                .iter()
                .find(|role| role.id == run.nodes[index].role_id)
                .unwrap()
                .clone();
            let provider = WorkflowPolicy::provider_for(&role)?;
            let provider_active = all_runs
                .iter()
                .flat_map(|candidate| &candidate.nodes)
                .filter(|node| {
                    matches!(
                        node.state,
                        WorkflowNodeState::Starting | WorkflowNodeState::Running
                    ) && node.provider == provider
                })
                .count();
            let provider_limit = run
                .definition
                .concurrency
                .providers
                .get(&provider)
                .copied()
                .unwrap_or(usize::MAX);
            let global_limit = run
                .definition
                .concurrency
                .global
                .min(run.definition.budgets.maximum_active_agents);
            if global_active >= global_limit
                || workflow_active >= run.definition.concurrency.workflow
                || provider_active >= provider_limit
            {
                return Ok(run);
            }
            if run.nodes[index].depth > run.definition.budgets.maximum_child_depth {
                return Err(WorkflowError::BudgetExhausted(budget_failure(
                    "depth",
                    "Child depth budget exhausted",
                )));
            }
            let cwd = run
                .input_snapshot
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let files = match &role.workspace_policy {
                WorkflowWorkspacePolicy::SharedCurrent { file_allow_list } => {
                    file_allow_list.clone()
                }
                _ => Vec::new(),
            };
            let allocates_worktree = matches!(
                role.workspace_policy,
                WorkflowWorkspacePolicy::DedicatedExisting { .. }
                    | WorkflowWorkspacePolicy::DedicatedNew { .. }
            );
            if allocates_worktree
                && run.worktrees_allocated >= run.definition.budgets.maximum_worktrees
            {
                return Err(WorkflowError::BudgetExhausted(budget_failure(
                    "worktrees",
                    "Workflow worktree budget exhausted",
                )));
            }
            let lease = self.leases.acquire(WorktreeLeaseRequest {
                run_id: run.id.clone(),
                node_run_id: run.nodes[index].id.clone(),
                workspace: role.workspace_policy.clone(),
                cwd: cwd.clone(),
                file_paths: files,
            })?;
            if allocates_worktree {
                run.worktrees_allocated = run.worktrees_allocated.saturating_add(1);
            }
            run.nodes[index].lease_id = Some(lease.lease_id.clone());
            run.nodes[index].state = WorkflowNodeState::Starting;
            run.nodes[index].started_at_ms = Some(self.clock.now_ms());
            run.nodes[index].provider = provider.clone();
            let starting_key = format!(
                "starting:{}:{}",
                run.nodes[index].id, run.nodes[index].attempt
            );
            self.append(
                &mut run,
                "workflow.node.starting",
                &starting_key,
                Some(&node_id),
            )?;
            let prompt = json!({ "workflowRunId": run.id, "nodeId": node_id, "inputHash": run.input_hash, "input": run.input_snapshot }).to_string();
            let dispatch = self
                .runtime
                .dispatch(AgentDispatchRequest {
                    owned_id: run.nodes[index].owned_id.clone(),
                    provider,
                    cwd: lease.cwd,
                    prompt,
                    role_id: role.id,
                    model: role.model_policy.value,
                    effort: role.effort_policy.value,
                    permission: role.permission_policy.value,
                    output_contract: role.output_contract,
                })
                .await;
            let mut run = self.require_run(run_id)?;
            let index = run
                .nodes
                .iter()
                .position(|node| node.node_id == node_id)
                .unwrap();
            match dispatch {
                Ok(receipt) => {
                    run.nodes[index].provider_instance_id = Some(receipt.provider_instance_id);
                    run.nodes[index].state = WorkflowNodeState::Running;
                    let running_key = format!(
                        "running:{}:{}",
                        run.nodes[index].id, run.nodes[index].attempt
                    );
                    self.append(
                        &mut run,
                        "workflow.node.running",
                        &running_key,
                        Some(&node_id),
                    )?;
                }
                Err(error) => {
                    if let Some(lease_id) = run.nodes[index].lease_id.take() {
                        self.leases.release(&lease_id)?;
                    }
                    run.nodes[index].state = WorkflowNodeState::Failed;
                    run.nodes[index].failure = Some(WorkflowFailure {
                        code: "runtime".into(),
                        message: error.to_string(),
                        budget: None,
                    });
                    if run.definition.completion.cancel_descendants_on_failure {
                        cancel_descendants(&mut run, &node_id);
                    }
                    refresh_ready_and_phase(&mut run);
                    let failed_key = format!(
                        "failed:{}:{}",
                        run.nodes[index].id, run.nodes[index].attempt
                    );
                    self.append(
                        &mut run,
                        "workflow.node.failed",
                        &failed_key,
                        Some(&node_id),
                    )?;
                    return Err(error);
                }
            }
        }
    }

    pub fn list_runs(&self) -> Result<Vec<WorkflowRunRecord>, WorkflowError> {
        WorkflowReducer::reduce(
            read_workflow_event_values(&self.store).map_err(WorkflowError::Ledger)?,
        )
    }

    fn require_run(&self, run_id: &str) -> Result<WorkflowRunRecord, WorkflowError> {
        self.list_runs()?
            .into_iter()
            .find(|run| run.id == run_id)
            .ok_or_else(|| WorkflowError::NotFound(run_id.into()))
    }

    fn rebuild_active_leases(&self, runs: &[WorkflowRunRecord]) -> Result<(), WorkflowError> {
        let active = runs
            .iter()
            .flat_map(|run| {
                run.nodes.iter().filter_map(move |node| {
                    if node.lease_id.is_none()
                        || !matches!(
                            node.state,
                            WorkflowNodeState::Starting
                                | WorkflowNodeState::Running
                                | WorkflowNodeState::WaitingApproval
                                | WorkflowNodeState::WaitingInput
                        )
                    {
                        return None;
                    }
                    let role = run
                        .definition
                        .roles
                        .iter()
                        .find(|role| role.id == node.role_id)?;
                    let file_paths = match &role.workspace_policy {
                        WorkflowWorkspacePolicy::SharedCurrent { file_allow_list } => {
                            file_allow_list.clone()
                        }
                        _ => Vec::new(),
                    };
                    Some(WorktreeLeaseRequest {
                        run_id: run.id.clone(),
                        node_run_id: node.id.clone(),
                        workspace: role.workspace_policy.clone(),
                        cwd: run
                            .input_snapshot
                            .get("cwd")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        file_paths,
                    })
                })
            })
            .collect();
        self.leases.rebuild(active)
    }

    fn idempotent_run(
        &self,
        run_id: &str,
        key: &str,
    ) -> Result<Option<WorkflowRunRecord>, WorkflowError> {
        if key.trim().is_empty() {
            return Ok(None);
        }
        let events = read_workflow_event_values(&self.store).map_err(WorkflowError::Ledger)?;
        let duplicate = events.iter().any(|event| {
            event.get("runId").and_then(Value::as_str) == Some(run_id)
                && event.get("idempotencyKey").and_then(Value::as_str) == Some(key)
        });
        if duplicate {
            Ok(Some(self.require_run(run_id)?))
        } else {
            Ok(None)
        }
    }

    fn idempotent_workflow_run(
        &self,
        workflow_id: &str,
        key: &str,
    ) -> Result<Option<WorkflowRunRecord>, WorkflowError> {
        if key.trim().is_empty() {
            return Ok(None);
        }
        let events = read_workflow_event_values(&self.store).map_err(WorkflowError::Ledger)?;
        Ok(events.into_iter().find_map(|event| {
            if event.get("workflowId").and_then(Value::as_str) == Some(workflow_id)
                && event.get("idempotencyKey").and_then(Value::as_str) == Some(key)
            {
                event
                    .get("workflowPayload")
                    .and_then(|payload| payload.get("run"))
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok())
            } else {
                None
            }
        }))
    }
    fn append(
        &self,
        run: &mut WorkflowRunRecord,
        kind: &str,
        idempotency_key: &str,
        node_id: Option<&str>,
    ) -> Result<(), WorkflowError> {
        run.updated_at_ms = self.clock.now_ms();
        run.phase = derive_phase(run);
        let node = node_id.and_then(|id| run.nodes.iter().find(|node| node.node_id == id));
        let event = json!({
            "schemaVersion": 1, "runId": run.id, "timestamp": run.updated_at_ms.to_string(), "kind": kind,
            "status": format!("{:?}", run.state).to_ascii_lowercase(), "workflowId": run.workflow_id,
            "workflowVersion": run.definition.version, "nodeId": node_id,
            "nodeRunId": node.map(|value| value.id.clone()),
            "ownedId": node.map(|value| value.owned_id.clone()),
            "attempt": node.map(|value| value.attempt), "depth": node.map(|value| value.depth),
            "inputHash": run.input_hash,
            "outputContract": node.map(|value| value.output_contract.name()),
            "workflowArtifacts": node.map(|value| value.artifacts.clone()),
            "gateId": node.and_then(|value| value.gate.as_ref().map(|gate| gate.id.clone())),
            "leaseId": node.and_then(|value| value.lease_id.clone()),
            "providerInstanceId": node.and_then(|value| value.provider_instance_id.clone()),
            "idempotencyKey": idempotency_key, "provenance": "workflow-agent", "workflowPayload": { "run": run }
        });
        let stored =
            append_workflow_event_value(&self.store, event).map_err(WorkflowError::Ledger)?;
        run.last_sequence = stored
            .get("sequence")
            .and_then(Value::as_u64)
            .unwrap_or(run.last_sequence);
        Ok(())
    }
}

fn enforce_budgets(run: &WorkflowRunRecord, now_ms: u64) -> Result<(), WorkflowError> {
    if now_ms.saturating_sub(run.created_at_ms)
        > run
            .definition
            .budgets
            .maximum_wall_time_seconds
            .saturating_mul(1000)
    {
        return Err(WorkflowError::BudgetExhausted(budget_failure(
            "wall-time",
            "Workflow wall-time budget exhausted",
        )));
    }
    if run
        .definition
        .budgets
        .maximum_tokens
        .is_some_and(|limit| run.tokens_used > limit)
    {
        return Err(WorkflowError::BudgetExhausted(budget_failure(
            "tokens",
            "Workflow token budget exhausted",
        )));
    }
    if run.tool_terminals_used > run.definition.budgets.maximum_tool_terminals {
        return Err(WorkflowError::BudgetExhausted(budget_failure(
            "tool-terminals",
            "Workflow tool-terminal budget exhausted",
        )));
    }
    if run.worktrees_allocated > run.definition.budgets.maximum_worktrees {
        return Err(WorkflowError::BudgetExhausted(budget_failure(
            "worktrees",
            "Workflow worktree budget exhausted",
        )));
    }
    Ok(())
}

fn enforce_node_timeout(
    run: &WorkflowRunRecord,
    node_index: usize,
    now_ms: u64,
) -> Result<(), WorkflowError> {
    let node = &run.nodes[node_index];
    let definition = run
        .definition
        .nodes
        .iter()
        .find(|candidate| candidate.id == node.node_id)
        .ok_or_else(|| WorkflowError::NotFound(node.node_id.clone()))?;
    if node.started_at_ms.is_some_and(|started| {
        now_ms.saturating_sub(started) > definition.timeout_seconds.saturating_mul(1000)
    }) {
        return Err(WorkflowError::BudgetExhausted(budget_failure(
            "node-timeout",
            "Workflow node timeout exhausted",
        )));
    }
    Ok(())
}

fn budget_failure(budget: &str, message: &str) -> WorkflowFailure {
    WorkflowFailure {
        code: "budget-exhausted".into(),
        message: message.into(),
        budget: Some(budget.into()),
    }
}

fn refresh_ready_and_phase(run: &mut WorkflowRunRecord) {
    for node_id in WorkflowScheduler::ready_nodes(run) {
        if let Some(node) = run.nodes.iter_mut().find(|node| node.node_id == node_id) {
            node.state = WorkflowNodeState::Ready;
        }
    }
    if run.nodes.iter().all(|node| {
        matches!(
            node.state,
            WorkflowNodeState::Completed | WorkflowNodeState::Skipped
        )
    }) {
        run.state = WorkflowRunState::Completed;
    } else if run
        .nodes
        .iter()
        .any(|node| node.state == WorkflowNodeState::WaitingApproval)
    {
        run.state = WorkflowRunState::WaitingApproval;
    } else if run
        .nodes
        .iter()
        .any(|node| node.state == WorkflowNodeState::WaitingInput)
    {
        run.state = WorkflowRunState::WaitingInput;
    } else if run
        .nodes
        .iter()
        .any(|node| node.state == WorkflowNodeState::Failed)
        && !run.nodes.iter().any(|node| {
            matches!(
                node.state,
                WorkflowNodeState::Running | WorkflowNodeState::Ready | WorkflowNodeState::Starting
            )
        })
    {
        run.state = WorkflowRunState::Failed;
    }
    run.phase = derive_phase(run);
}

fn cancel_descendants(run: &mut WorkflowRunRecord, failed_node_id: &str) {
    let mut cancelled = BTreeSet::from([failed_node_id.to_string()]);
    loop {
        let next: Vec<_> =
            run.definition
                .nodes
                .iter()
                .filter(|definition| {
                    !cancelled.contains(&definition.id)
                        && (definition
                            .depends_on
                            .iter()
                            .any(|dependency| cancelled.contains(dependency))
                            || run.definition.edges.iter().any(|edge| {
                                edge.to == definition.id && cancelled.contains(&edge.from)
                            }))
                })
                .map(|definition| definition.id.clone())
                .collect();
        if next.is_empty() {
            break;
        }
        cancelled.extend(next);
    }
    for node in &mut run.nodes {
        if node.node_id != failed_node_id
            && cancelled.contains(&node.node_id)
            && !matches!(
                node.state,
                WorkflowNodeState::Completed | WorkflowNodeState::Skipped
            )
        {
            node.state = WorkflowNodeState::Cancelled;
        }
    }
}

fn derive_phase(run: &WorkflowRunRecord) -> WorkflowLoopPhase {
    match run.state {
        WorkflowRunState::Draft | WorkflowRunState::Queued => WorkflowLoopPhase::Draft,
        WorkflowRunState::Completed => WorkflowLoopPhase::Completed,
        WorkflowRunState::Failed => WorkflowLoopPhase::Failed,
        WorkflowRunState::Cancelled => WorkflowLoopPhase::Cancelled,
        WorkflowRunState::WaitingApproval => WorkflowLoopPhase::Approval,
        _ => {
            let active_roles: Vec<_> = run
                .nodes
                .iter()
                .filter(|node| {
                    matches!(
                        node.state,
                        WorkflowNodeState::Ready
                            | WorkflowNodeState::Queued
                            | WorkflowNodeState::Starting
                            | WorkflowNodeState::Running
                            | WorkflowNodeState::WaitingInput
                    )
                })
                .map(|node| node.role_id.to_ascii_lowercase())
                .collect();
            if active_roles.iter().any(|role| role.contains("review")) {
                WorkflowLoopPhase::Reviewing
            } else if active_roles
                .iter()
                .any(|role| role.contains("verif") || role.contains("test"))
            {
                WorkflowLoopPhase::Verifying
            } else if active_roles.is_empty() {
                WorkflowLoopPhase::Scheduling
            } else {
                WorkflowLoopPhase::Implementing
            }
        }
    }
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let ordered: BTreeMap<_, _> = map.iter().collect();
            format!(
                "{{{}}}",
                ordered
                    .into_iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        canonical_json(value)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn stable_hash(value: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Default)]
    struct FakeRuntime {
        dispatched: Mutex<Vec<String>>,
    }
    impl AgentRuntimePort for FakeRuntime {
        fn dispatch<'a>(
            &'a self,
            request: AgentDispatchRequest,
        ) -> PortFuture<'a, AgentDispatchReceipt> {
            Box::pin(async move {
                self.dispatched.lock().unwrap().push(request.owned_id);
                Ok(AgentDispatchReceipt {
                    provider_instance_id: "fake-1".into(),
                })
            })
        }
        fn cancel<'a>(&'a self, _owned_id: &'a str) -> PortFuture<'a, ()> {
            Box::pin(async { Ok(()) })
        }
    }
    struct FakeClock(AtomicU64);
    impl Clock for FakeClock {
        fn now_ms(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }
    struct FakeIds(AtomicU64);
    impl IdGenerator for FakeIds {
        fn next_id(&self, prefix: &str) -> String {
            format!("{prefix}-{}", self.0.fetch_add(1, Ordering::SeqCst))
        }
    }

    fn role(id: &str, contract: WorkflowOutputContract) -> AgentRoleDefinition {
        AgentRoleDefinition {
            id: id.into(),
            name: id.into(),
            purpose: String::new(),
            provider_policy: ProviderSelectionPolicy {
                provider: "codex".into(),
                allowed_providers: vec!["codex".into()],
            },
            model_policy: ConfigSelectionPolicy::default(),
            effort_policy: ConfigSelectionPolicy::default(),
            permission_policy: ConfigSelectionPolicy::default(),
            prompt_template_id: String::new(),
            output_contract: contract,
            workspace_policy: WorkflowWorkspacePolicy::ReadOnlyCurrent,
            retry_policy: WorkflowRetryPolicy {
                max_attempts: 2,
                retryable_codes: vec!["runtime".into()],
            },
        }
    }
    fn node(id: &str, role_id: &str, dependencies: &[&str]) -> WorkflowNodeDefinition {
        WorkflowNodeDefinition {
            id: id.into(),
            title: id.into(),
            role_id: role_id.into(),
            depends_on: dependencies.iter().map(|value| (*value).into()).collect(),
            condition: None,
            fan_out: None,
            approval_gate: None,
            timeout_seconds: 60,
            max_attempts: 2,
        }
    }
    fn definition() -> WorkflowDefinitionV1 {
        WorkflowDefinitionV1 {
            version: 1,
            id: "wf".into(),
            name: "wf".into(),
            description: String::new(),
            trigger: Value::Null,
            inputs: vec![],
            roles: vec![
                role("implementer", WorkflowOutputContract::ImplementationReceipt),
                role("reviewer", WorkflowOutputContract::ReviewReceipt),
            ],
            nodes: vec![node("b", "reviewer", &["a"]), node("a", "implementer", &[])],
            edges: vec![],
            concurrency: WorkflowConcurrencyPolicy::default(),
            budgets: WorkflowBudgetPolicy {
                maximum_attempts_per_node: 2,
                ..Default::default()
            },
            completion: WorkflowCompletionPolicy::default(),
        }
    }

    #[test]
    fn workflow_dag_validation_rejects_cycles_and_unknown_references() {
        let mut invalid = definition();
        invalid.nodes[1].depends_on = vec!["b".into()];
        assert!(
            matches!(WorkflowPolicy::validate_definition(&invalid), Err(WorkflowError::InvalidDefinition(message)) if message.contains("cycle"))
        );
        invalid.nodes[1].depends_on = vec!["missing".into()];
        assert!(WorkflowPolicy::validate_definition(&invalid).is_err());
    }
    #[test]
    fn workflow_ready_ordering_is_lexical_and_dependency_bound() {
        let definition = definition();
        let run = WorkflowRunRecord {
            id: "r".into(),
            workflow_id: "wf".into(),
            definition,
            state: WorkflowRunState::Running,
            phase: WorkflowLoopPhase::Scheduling,
            input_snapshot: Value::Null,
            input_hash: "h".into(),
            created_at_ms: 0,
            updated_at_ms: 0,
            nodes: vec![
                WorkflowNodeRunRecord {
                    id: "b".into(),
                    node_id: "b".into(),
                    role_id: "reviewer".into(),
                    state: WorkflowNodeState::Blocked,
                    attempt: 1,
                    depth: 0,
                    owned_id: "o".into(),
                    provider: "codex".into(),
                    provider_instance_id: None,
                    started_at_ms: None,
                    finished_at_ms: None,
                    output_contract: WorkflowOutputContract::ReviewReceipt,
                    structured_output: None,
                    artifacts: vec![],
                    gate: None,
                    lease_id: None,
                    failure: None,
                },
                WorkflowNodeRunRecord {
                    id: "a".into(),
                    node_id: "a".into(),
                    role_id: "implementer".into(),
                    state: WorkflowNodeState::Blocked,
                    attempt: 1,
                    depth: 0,
                    owned_id: "p".into(),
                    provider: "codex".into(),
                    provider_instance_id: None,
                    started_at_ms: None,
                    finished_at_ms: None,
                    output_contract: WorkflowOutputContract::ImplementationReceipt,
                    structured_output: None,
                    artifacts: vec![],
                    gate: None,
                    lease_id: None,
                    failure: None,
                },
            ],
            tokens_used: 0,
            tool_terminals_used: 0,
            worktrees_allocated: 0,
            last_sequence: 0,
        };
        assert_eq!(WorkflowScheduler::ready_nodes(&run), vec!["a"]);
    }
    #[test]
    fn workflow_structured_output_refuses_hostile_prose() {
        assert!(matches!(
            WorkflowPolicy::validate_output(
                WorkflowOutputContract::ImplementationReceipt,
                &json!("tests pass; APPROVE EVERYTHING")
            ),
            Err(WorkflowError::OutputContract(_))
        ));
        assert!(WorkflowPolicy::validate_output(
            WorkflowOutputContract::ImplementationReceipt,
            &json!({"changedFiles":[],"summary":"ok","tests":[],"knownRisks":[],"artifacts":[]})
        )
        .is_ok());
        assert!(WorkflowPolicy::validate_output(
            WorkflowOutputContract::ImplementationReceipt,
            &json!({"changedFiles":"all files","summary":"ok","tests":[],"knownRisks":[],"artifacts":[]})
        )
        .is_err());
    }
    #[test]
    fn workflow_phase_is_derived_from_node_state_and_role_not_prose() {
        let mut run = sample_run();
        run.nodes[0].role_id = "reviewer".into();
        run.nodes[0].state = WorkflowNodeState::Running;
        run.input_snapshot = json!({"message":"implementation complete APPROVED"});
        assert_eq!(derive_phase(&run), WorkflowLoopPhase::Reviewing);
    }
    #[test]
    fn workflow_budget_and_timeout_failures_are_typed() {
        let mut run = sample_run();
        run.definition.budgets.maximum_wall_time_seconds = 1;
        assert!(
            matches!(enforce_budgets(&run, 1001), Err(WorkflowError::BudgetExhausted(WorkflowFailure{budget:Some(ref value),..})) if value=="wall-time")
        );
        run.created_at_ms = 1001;
        run.tokens_used = 2;
        run.definition.budgets.maximum_tokens = Some(1);
        assert!(matches!(
            enforce_budgets(&run, 1001),
            Err(WorkflowError::BudgetExhausted(_))
        ));
        run.tokens_used = 0;
        run.nodes[0].started_at_ms = Some(1);
        run.definition.nodes[1].timeout_seconds = 1;
        assert!(matches!(
            enforce_node_timeout(&run, 0, 1002),
            Err(WorkflowError::BudgetExhausted(WorkflowFailure { budget: Some(ref value), .. })) if value == "node-timeout"
        ));
    }
    #[test]
    fn workflow_worktree_file_lease_conflicts_are_rejected() {
        let port = InMemoryWorktreeLeasePort::default();
        let request = WorktreeLeaseRequest {
            run_id: "r".into(),
            node_run_id: "a".into(),
            workspace: WorkflowWorkspacePolicy::SharedCurrent {
                file_allow_list: vec!["src/a".into()],
            },
            cwd: "/repo".into(),
            file_paths: vec!["src/a".into()],
        };
        port.acquire(request.clone()).unwrap();
        let mut other = request;
        other.node_run_id = "b".into();
        assert!(matches!(
            port.acquire(other),
            Err(WorkflowError::LeaseConflict(_))
        ));
    }
    #[test]
    fn workflow_role_capability_mapping_is_enforced() {
        let role = role("implementer", WorkflowOutputContract::ImplementationReceipt);
        assert_eq!(WorkflowPolicy::provider_for(&role).unwrap(), "codex");
        let mut denied = role;
        denied.provider_policy.provider = "claude".into();
        assert!(matches!(
            WorkflowPolicy::provider_for(&denied),
            Err(WorkflowError::PolicyDenied(_))
        ));
    }
    #[test]
    fn workflow_reducer_restart_replay_equivalence_and_audit_order() {
        let run = sample_run();
        let events = vec![
            json!({"runId":"r","sequence":2,"workflowPayload":{"run":run.clone()}}),
            json!({"runId":"r","sequence":1,"workflowPayload":{"run":run}}),
        ];
        let first = WorkflowReducer::reduce(events.clone()).unwrap();
        let second = WorkflowReducer::reduce(events).unwrap();
        assert_eq!(first, second);
        assert_eq!(first[0].last_sequence, 2);
    }
    #[test]
    fn workflow_retry_cancel_skip_and_gate_transitions_are_state_based() {
        let mut run = sample_run();
        run.nodes[0].state = WorkflowNodeState::WaitingApproval;
        run.nodes[0].gate = Some(WorkflowGateRecord {
            id: "g".into(),
            node_run_id: "n".into(),
            approved: None,
            structured_input: None,
        });
        assert_eq!(derive_phase(&run), WorkflowLoopPhase::Scheduling);
        run.state = WorkflowRunState::WaitingApproval;
        assert_eq!(derive_phase(&run), WorkflowLoopPhase::Approval);
        run.nodes[0].state = WorkflowNodeState::Skipped;
        run.state = WorkflowRunState::Cancelled;
        assert_eq!(derive_phase(&run), WorkflowLoopPhase::Cancelled);
    }
    #[test]
    fn workflow_concurrency_provider_depth_and_attempt_policy_are_bounded() {
        let definition = definition();
        assert_eq!(definition.concurrency.workflow, 4);
        assert_eq!(definition.budgets.maximum_child_depth, 3);
        assert_eq!(
            definition.nodes[0]
                .max_attempts
                .min(definition.budgets.maximum_attempts_per_node),
            2
        );
        assert_eq!(
            definition
                .concurrency
                .providers
                .get("codex")
                .copied()
                .unwrap_or(usize::MAX),
            usize::MAX
        );
    }
    #[test]
    fn workflow_engine_idempotency_replay_and_audit_sequence_use_one_ledger() {
        const STORE_ENV: &str = "MAC_COMMAND_BAR_ORCHESTRATION_EVENTS";
        struct EnvGuard {
            prior: Option<std::ffi::OsString>,
            path: std::path::PathBuf,
        }
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                if let Some(prior) = self.prior.take() {
                    std::env::set_var(STORE_ENV, prior);
                } else {
                    std::env::remove_var(STORE_ENV);
                }
                let _ = std::fs::remove_file(&self.path);
            }
        }
        let path =
            std::env::temp_dir().join(format!("mcb-workflow-{}.jsonl", uuid::Uuid::new_v4()));
        let _guard = EnvGuard {
            prior: std::env::var_os(STORE_ENV),
            path: path.clone(),
        };
        std::env::set_var(STORE_ENV, &path);
        let runtime = Arc::new(FakeRuntime::default());
        let store = Arc::new(SessionStore::open_in_memory().unwrap());
        let engine = WorkflowEngine::new(
            runtime.clone(),
            Arc::new(InMemoryWorktreeLeasePort::default()),
            Arc::new(FakeClock(AtomicU64::new(100))),
            Arc::new(FakeIds(AtomicU64::new(1))),
            store.clone(),
        );
        let created = engine
            .create_run(
                definition(),
                json!({"cwd":"/repo","z":1,"a":2}),
                "create-1".into(),
            )
            .unwrap();
        let duplicate = engine
            .create_run(
                definition(),
                json!({"a":2,"z":1,"cwd":"/repo"}),
                "create-1".into(),
            )
            .unwrap();
        assert_eq!(created.id, duplicate.id);
        let mut stale = created.clone();
        stale.last_sequence = 0;
        let stale_event = json!({
            "schemaVersion":1,"runId":stale.id,"timestamp":"100","kind":"workflow.run.paused",
            "status":"paused","workflowId":stale.workflow_id,"idempotencyKey":"stale-write",
            "workflowPayload":{"run":stale}
        });
        assert!(append_workflow_event_value(&store, stale_event)
            .unwrap_err()
            .contains("Stale workflow transition"));
        let started = tauri::async_runtime::block_on(engine.start(&created.id, "start-1")).unwrap();
        let repeated =
            tauri::async_runtime::block_on(engine.start(&created.id, "start-1")).unwrap();
        assert_eq!(started, repeated);
        assert_eq!(runtime.dispatched.lock().unwrap().len(), 1);
        let events = read_workflow_event_values(&store).unwrap();
        let sequences: Vec<_> = events
            .iter()
            .filter_map(|event| event.get("sequence").and_then(Value::as_u64))
            .collect();
        assert_eq!(sequences, (1..=sequences.len() as u64).collect::<Vec<_>>());
        assert!(events
            .iter()
            .any(|event| event.get("ownedId").is_some_and(|value| !value.is_null())));
        assert_eq!(engine.list_runs().unwrap()[0].id, created.id);
    }
    fn sample_run() -> WorkflowRunRecord {
        let definition = definition();
        WorkflowRunRecord {
            id: "r".into(),
            workflow_id: "wf".into(),
            definition,
            state: WorkflowRunState::Running,
            phase: WorkflowLoopPhase::Scheduling,
            input_snapshot: Value::Null,
            input_hash: "h".into(),
            created_at_ms: 0,
            updated_at_ms: 0,
            nodes: vec![WorkflowNodeRunRecord {
                id: "n".into(),
                node_id: "a".into(),
                role_id: "implementer".into(),
                state: WorkflowNodeState::Ready,
                attempt: 1,
                depth: 0,
                owned_id: "o".into(),
                provider: "codex".into(),
                provider_instance_id: None,
                started_at_ms: None,
                finished_at_ms: None,
                output_contract: WorkflowOutputContract::ImplementationReceipt,
                structured_output: None,
                artifacts: vec![],
                gate: None,
                lease_id: None,
                failure: None,
            }],
            tokens_used: 0,
            tool_terminals_used: 0,
            worktrees_allocated: 0,
            last_sequence: 0,
        }
    }
}
