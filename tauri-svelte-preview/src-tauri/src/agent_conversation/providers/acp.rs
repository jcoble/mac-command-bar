use super::super::capabilities::{replace_config_options, validate_capabilities};
use super::super::protocol::{AgentCapabilities, AgentProviderManifest};
use super::acp_client::{AcpInbound, AcpTransport};
use super::{
    AcpClient, AgentConfigOption, AgentConfigValue, AgentPrompt, AgentRuntimeAdapter,
    AgentRuntimeError, AgentSteeringInput, GeneratedText, InitializeAgentInput, LoadAgentSession,
    NewAgentSession, PermissionResponse, ResumeAgentSession, StartedAgentSession, StartedTurn,
    UserInputResponse,
};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AcpRuntimeAdapter {
    manifest: AgentProviderManifest,
    owned_id: String,
    cwd: std::path::PathBuf,
    client: Option<AcpClient>,
    capabilities: Option<AgentCapabilities>,
}

impl AcpRuntimeAdapter {
    pub fn new(manifest: AgentProviderManifest, owned_id: String, cwd: std::path::PathBuf) -> Self {
        Self {
            manifest,
            owned_id,
            cwd,
            client: None,
            capabilities: None,
        }
    }

    fn client_mut(&mut self) -> Result<&mut AcpClient, AgentRuntimeError> {
        self.client.as_mut().ok_or_else(|| {
            AgentRuntimeError::new("not-initialized", "ACP adapter is not initialized")
        })
    }

    pub fn process_id(&self) -> Option<u32> {
        self.client.as_ref().and_then(AcpClient::process_id)
    }

    pub fn transport(&self) -> Result<Arc<AcpTransport>, AgentRuntimeError> {
        self.client
            .as_ref()
            .map(AcpClient::transport)
            .ok_or_else(|| {
                AgentRuntimeError::new("not-initialized", "ACP adapter is not initialized")
            })
    }

    pub fn take_inbound(
        &mut self,
    ) -> Result<mpsc::UnboundedReceiver<AcpInbound>, AgentRuntimeError> {
        self.client_mut()?.take_inbound()
    }
}

impl AgentRuntimeAdapter for AcpRuntimeAdapter {
    async fn initialize(
        &mut self,
        input: InitializeAgentInput,
    ) -> Result<AgentCapabilities, AgentRuntimeError> {
        let mut client = AcpClient::spawn(&self.manifest, &self.cwd, &self.owned_id)?;
        let capabilities = client.initialize(input.provider).await?;
        validate_capabilities(&capabilities)
            .map_err(|message| AgentRuntimeError::new("invalid-capabilities", message))?;
        self.client = Some(client);
        self.capabilities = Some(capabilities.clone());
        Ok(capabilities)
    }

    async fn new_session(
        &mut self,
        input: NewAgentSession,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        self.client_mut()?.new_session(&input.cwd).await
    }
    async fn load_session(
        &mut self,
        input: LoadAgentSession,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        self.client_mut()?
            .load_session(&input.cwd, &input.native_session_id)
            .await
    }
    async fn resume_session(
        &mut self,
        input: ResumeAgentSession,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        self.client_mut()?
            .resume_session(&input.cwd, &input.native_session_id)
            .await
    }
    async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
        self.client_mut()?.prompt(input).await
    }

    async fn prompt_once(
        &mut self,
        input: AgentPrompt,
    ) -> Result<GeneratedText, AgentRuntimeError> {
        self.client_mut()?.prompt_once(input).await
    }
    async fn steer(&mut self, input: AgentSteeringInput) -> Result<(), AgentRuntimeError> {
        self.client_mut()?.steer(input.text).await
    }
    async fn cancel_turn(&mut self, _turn_id: Option<&str>) -> Result<(), AgentRuntimeError> {
        self.client_mut()?.cancel().await
    }

    async fn set_config(
        &mut self,
        option_id: &str,
        value: AgentConfigValue,
    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
        let replacement = self.client_mut()?.set_config(option_id, value).await?;
        if let Some(capabilities) = &mut self.capabilities {
            replace_config_options(capabilities, replacement.clone())
                .map_err(|message| AgentRuntimeError::new("invalid-config", message))?;
        }
        Ok(replacement)
    }

    async fn respond_permission(
        &mut self,
        input: PermissionResponse,
    ) -> Result<(), AgentRuntimeError> {
        let decision = match input.decision {
            super::super::protocol::AgentApprovalDecision::Accept => "selected",
            super::super::protocol::AgentApprovalDecision::Decline => "cancelled",
            super::super::protocol::AgentApprovalDecision::Cancel => "cancelled",
        };
        self.client_mut()?
            .respond_permission(&input.identity.request_id, decision)
            .await
    }

    async fn respond_user_input(
        &mut self,
        input: UserInputResponse,
    ) -> Result<(), AgentRuntimeError> {
        self.client_mut()?
            .respond_user_input(&input.identity.request_id, input.values, input.cancelled)
            .await
    }

    async fn detach_session(&mut self) -> Result<(), AgentRuntimeError> {
        if let Some(client) = &mut self.client {
            client.detach().await?;
        }
        self.client = None;
        Ok(())
    }

    async fn close_session(&mut self) -> Result<(), AgentRuntimeError> {
        if let Some(client) = &mut self.client {
            client.close().await?;
        }
        self.client = None;
        Ok(())
    }
}

pub enum StructuredRuntimeHandle {
    Acp(AcpRuntimeAdapter),
}

impl StructuredRuntimeHandle {
    /// Return the OS process that backs this structured provider session.
    /// Resources uses this as the root of the app-owned process tree; it is
    /// intentionally read-only and never exposes the provider transport.
    pub fn process_id(&self) -> Option<u32> {
        match self {
            Self::Acp(adapter) => adapter.process_id(),
        }
    }

    pub fn transport(&self) -> Result<Arc<AcpTransport>, AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.transport(),
        }
    }

    pub fn take_inbound(
        &mut self,
    ) -> Result<mpsc::UnboundedReceiver<AcpInbound>, AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.take_inbound(),
        }
    }

    pub async fn prompt(&mut self, input: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.prompt(input).await,
        }
    }
    pub async fn prompt_once(
        &mut self,
        input: AgentPrompt,
    ) -> Result<GeneratedText, AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.prompt_once(input).await,
        }
    }
    pub async fn cancel_turn(&mut self, turn_id: Option<&str>) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.cancel_turn(turn_id).await,
        }
    }
    pub async fn steer(&mut self, input: AgentSteeringInput) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.steer(input).await,
        }
    }
    pub async fn set_config(
        &mut self,
        option_id: &str,
        value: AgentConfigValue,
    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.set_config(option_id, value).await,
        }
    }
    pub async fn respond_permission(
        &mut self,
        input: PermissionResponse,
    ) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.respond_permission(input).await,
        }
    }
    pub async fn respond_user_input(
        &mut self,
        input: UserInputResponse,
    ) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.respond_user_input(input).await,
        }
    }
    pub async fn close_session(&mut self) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.close_session().await,
        }
    }

    pub async fn detach_session(&mut self) -> Result<(), AgentRuntimeError> {
        match self {
            Self::Acp(adapter) => adapter.detach_session().await,
        }
    }
}
