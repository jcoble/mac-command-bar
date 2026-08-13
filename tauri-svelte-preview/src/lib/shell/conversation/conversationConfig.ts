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

/** Freeze provider-advertised choices for one open menu while events keep streaming. */
export function snapshotAgentConversationConfig(
  state: AgentConversationConfigState
): AgentConversationConfigState {
  return {
    model: state.model,
    availableModels: [...state.availableModels],
    reasoningEffort: state.reasoningEffort,
    availableEfforts: [...state.availableEfforts],
    approvalPolicy: state.approvalPolicy,
    availableApprovalPolicies: [...state.availableApprovalPolicies]
  };
}

/** Whether the composer has any current value or advertised choice to present. */
export function hasAgentConversationConfig(state: AgentConversationConfigState): boolean {
  return Boolean(
    state.model ||
      state.reasoningEffort ||
      state.approvalPolicy ||
      state.availableModels.length ||
      state.availableEfforts.length ||
      state.availableApprovalPolicies.length
  );
}

export function readAgentConversationConfig(ownedId: string): Promise<AgentConversationConfigState> {
  return invoke<AgentConversationConfigState>('read_agent_conversation_config', { ownedId });
}

export function setAgentConversationConfig(
  request: AgentConversationConfigRequest
): Promise<AgentConversationConfigState> {
  return invoke<AgentConversationConfigState>('set_agent_conversation_config', { request });
}
