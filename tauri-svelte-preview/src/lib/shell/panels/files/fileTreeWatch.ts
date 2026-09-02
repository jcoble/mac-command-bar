import { watch, type UnwatchFn } from "@tauri-apps/plugin-fs";

/**
 * Own one native watcher for the visible file tree. Aborting the panel owner
 * releases it even when the native registration finishes after teardown.
 */
export async function watchFileTree(
	root: string,
	signal: AbortSignal,
	onChange: () => void,
): Promise<void> {
	if (signal.aborted) return;
	let unwatch: UnwatchFn | null = null;
	let stopped = false;
	const stop = (): void => {
		if (stopped) return;
		stopped = true;
		signal.removeEventListener("abort", stop);
		unwatch?.();
		unwatch = null;
	};
	signal.addEventListener("abort", stop, { once: true });
	try {
		const registered = await watch(
			root,
			() => {
				if (!signal.aborted) onChange();
			},
			{ recursive: true, delayMs: 350 },
		);
		if (stopped || signal.aborted) {
			registered();
			return;
		}
		unwatch = registered;
	} catch (error) {
		stop();
		if (!signal.aborted) throw error;
	}
}
