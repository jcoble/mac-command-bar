globalThis.isTauri = true;
let nextId = 1;
const callbacks = new Map();
window.__TAURI_INTERNALS__ = {
  transformCallback(callback) { const id = nextId++; callbacks.set(id, callback); return id; },
  unregisterCallback(id) { callbacks.delete(id); },
  convertFileSrc(path) { return path; },
  async invoke(command, args) {
    if (command === 'list_terminal_sessions') return [{ sessionId: 'pty-proof', cwd: '/Users/blackcolours/dev/work/mac-command-bar', shell: '/bin/zsh', cols: 120, rows: 40, pid: 4242, startedAt: Date.now(), exited: false, exitCode: null, signal: null }];
    if (command === 'read_terminal_session_scrollback') return '';
    if (command === 'read_agent_conversation_transcript') {
      if (args?.childSessionId) return {
        messages: [
          { itemId: 'c1', role: 'user', text: 'Inspect the composer and transcript.', timestampMs: 1 },
          { itemId: 'c2', role: 'assistant', text: 'Child transcript is read-only and remains attached to its parent session.\n\n```ts\nconst child = transcript.children[0];\n```', timestampMs: 2 }
        ],
        metadata: { model: 'gpt-5.6-sol', effort: 'medium', approvalPolicy: 'never', usedTokens: 22000, contextWindow: 258400 },
        children: []
      };
      return {
        messages: [
          { itemId: 'u1', role: 'user', text: 'Make the conversation surface easier to read.', timestampMs: 1 },
          { itemId: 'a1', role: 'assistant', text: 'The composer now uses clearer spacing, restrained color, and selectable transcript text.\n\n- Screenshot paste stays with this session.\n- Commands go through the existing PTY.', timestampMs: 2 }
        ],
        metadata: { model: 'gpt-5.6-sol', effort: 'medium', approvalPolicy: 'never', usedTokens: 72684, contextWindow: 258400 },
        children: [{ childId: 'child-proof', parentId: 'thread-proof', provider: 'codex', label: 'UI verifier', state: 'active', updatedAtMs: 3 }]
      };
    }
    if (command === 'plugin:event|listen') return nextId++;
    if (command === 'plugin:event|unlisten') return null;
    if (command === 'save_agent_conversation_attachment') return {
      id: 'attachment-proof',
      name: 'screenshot-proof.png',
      path: '/tmp/conversation-attachments/owned-proof/screenshot-proof.png',
      mimeType: 'image/png',
      byteLength: args?.bytes?.length ?? 0
    };
    if (command === 'write_terminal_session' || command === 'resize_terminal_session') return true;
    if (command === 'close_agent_conversation') return false;
    if (command.startsWith('list_')) return [];
    return null;
  }
};
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
