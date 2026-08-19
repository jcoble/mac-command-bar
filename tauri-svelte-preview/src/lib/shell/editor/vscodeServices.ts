import {
  MonacoVscodeApiWrapper,
  type MonacoVscodeApiConfig,
} from "monaco-languageclient/vscodeApiWrapper";

let startingServices: Promise<void> | null = null;

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
 * way. So the start is kept here instead, and it is kept for good: once a
 * start has begun the wrapper refuses every later one, so a failed start
 * cannot be retried and must be reported rather than quietly repeated.
 *
 * Pass the configuration to start the services; call it with nothing to wait
 * for whatever start is already under way.
 */
export function ensureVscodeServices(
  config?: MonacoVscodeApiConfig
): Promise<void> {
  if (!startingServices && config) {
    startingServices = new MonacoVscodeApiWrapper(config).start();
  }
  return startingServices ?? Promise.resolve();
}
