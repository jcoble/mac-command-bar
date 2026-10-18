/*
 * Adapted from code-yeongyu/web-terminal src/client/ime-input.ts
 * (MIT License, Copyright (c) 2026 YeonGyu Kim - keep this notice).
 * Changes: (1) composition text is sent LIVE as a diff against what was already sent
 * (Android keyboards compose whole words), (2) ghostty's own composition handlers are
 * blocked so nothing is sent twice and its isComposing flag never sticks,
 * (3) the hidden textarea is emptied after each commit so the keyboard never re-opens an old word.
 */
function attachImeInput(container, textarea, send) {
  let composing = false;
  let sent = ''; // composition text already sent to the shell
  const reconcile = (next) => {
    const a = Array.from(sent);
    const b = Array.from(next);
    let i = 0;
    while (i < a.length && i < b.length && a[i] === b[i]) i++;
    const out = '\x7f'.repeat(a.length - i) + b.slice(i).join('');
    if (out) send(out);
    sent = next;
  };
  const clear = () => { textarea.value = ''; };
  const block = (e) => e.stopPropagation();
  const on = (type, handler) => container.addEventListener(type, handler, { capture: true });
  on('compositionstart', (e) => { composing = true; sent = ''; block(e); });
  on('compositionupdate', block);
  on('compositionend', (e) => {
    reconcile(e.data || sent); // the committed text wins (autocorrect or a suggestion)
    composing = false;
    sent = '';
    clear();
    block(e);
  });
  on('beforeinput', (e) => {
    if (e.inputType === 'insertCompositionText') { reconcile(e.data || ''); return; }
    if (composing) return;
    if (e.inputType === 'deleteContentBackward') { send('\x7f'); return; }
    if (e.inputType === 'insertLineBreak' || e.inputType === 'insertParagraph') { send('\r'); return; }
    if (e.inputType === 'insertText' && e.data) { send(e.data); clear(); }
  });
}
