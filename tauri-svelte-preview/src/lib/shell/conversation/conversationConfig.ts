import { invoke } from '@tauri-apps/api/core';

export type AgentConversationConfigState = {
  model: string | null;
  availableModels: string[];
  reasoningEffort: string | null;
  availableEfforts: string[];
  approvalPolicy: string | null;
  availableApprovalPolicies: string[];
};

export type AgentConversationConfigRequest = {
  ownedId: string;
  generation: number;
  model?: string;
  reasoningEffort?: string;
  approvalPolicy?: string;
};

export type AgentConversationConfigField = 'model' | 'reasoningEffort' | 'approvalPolicy';

export const emptyAgentConversationConfigState = (): AgentConversationConfigState => ({
  model: null,
  availableModels: [],
  reasoningEffort: null,
  availableEfforts: [],
  approvalPolicy: null,
  availableApprovalPolicies: []
});

export function readAgentConversationConfig(ownedId: string): Promise<AgentConversationConfigState> {
  return invoke<AgentConversationConfigState>('read_agent_conversation_config', { ownedId });
}

export function setAgentConversationConfig(
  request: AgentConversationConfigRequest
): Promise<AgentConversationConfigState> {
  return invoke<AgentConversationConfigState>('set_agent_conversation_config', { request });
}
