import {
  MonacoVscodeApiWrapper,
  type MonacoVscodeApiConfig,
} from "monaco-languageclient/vscodeApiWrapper";
import { servicesInitialized } from "@codingame/monaco-vscode-api/lifecycle";
import { retryRejectedStart } from "$lib/shell/components/editor/editorStartup";

let startServices: (() => Promise<void>) | null = null;

/**
 * The one place the VS Code service layer is started, and the one promise
 * everyone waits on to know it is up.
 *
 * The order matters more than it looks. Building a Monaco editor is the last
 * step of that start, not something that can happen alongside it: creating an
 * editor asks the markdown renderer service to take a code-block renderer, and
 * the stand-in that answers before the real services are registered refuses,
 * so the editor does not open at all. Creating one early also settles the
 * service container for the rest of the page, after which the real start
 * refuses to run — the file that failed to open takes the language server with
 * it.
 *
 * The wrapper cannot enforce this on its own. Asked to start while another
 * start is still running it returns immediately, having done nothing, which
 * tells the second caller the services are ready while they are still on their
 * way. So a successful start is kept here, while a rejected one is cleared for
 * one safe retry. If the vendor container was partly initialized first, retry
 * is refused here instead of calling its one-shot initializer again.
 *
 * Pass the configuration to start the services; call it with nothing to wait
 * for whatever start is already under way.
 */
export function ensureVscodeServices(
  config?: MonacoVscodeApiConfig
): Promise<void> {
  if (!startServices && config) {
    startServices = retryRejectedStart(async () => {
      if (servicesInitialized) {
        throw new Error("Code services are partly initialized and cannot be restarted safely.");
      }
      await new MonacoVscodeApiWrapper(config).start();
    });
  }
  return startServices?.() ?? Promise.resolve();
}
