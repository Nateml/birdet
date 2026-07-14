import { writable } from 'svelte/store';

// Auto-update state, driven by @tauri-apps/plugin-updater. Everything is guarded
// with dynamic imports so `pnpm dev` in a plain browser (no Tauri) is a no-op.

export type UpdateStatus =
	| 'idle'
	| 'checking'
	| 'available'
	| 'up-to-date'
	| 'downloading'
	| 'ready'
	| 'error'
	| 'unsupported';

export type UpdateState = {
	status: UpdateStatus;
	currentVersion?: string;
	version?: string; // the newer version on offer
	notes?: string; // release notes (Update.body)
	progress?: number; // 0..1 while downloading
	error?: string;
};

export const updateState = writable<UpdateState>({ status: 'idle' });

// The Update handle is non-serializable, so it lives outside the store.
let pending: { downloadAndInstall: (cb: (e: DownloadEvent) => void) => Promise<void> } | null = null;

type DownloadEvent =
	| { event: 'Started'; data: { contentLength?: number } }
	| { event: 'Progress'; data: { chunkLength: number } }
	| { event: 'Finished' };

async function currentVersion(): Promise<string | undefined> {
	try {
		const { getVersion } = await import('@tauri-apps/api/app');
		return await getVersion();
	} catch {
		return undefined;
	}
}

/** Check GitHub for a newer release. `silent` keeps quiet on failure (used for
 *  the automatic check at startup, where a dev build without a pubkey throws). */
export async function checkForUpdate(opts: { silent?: boolean } = {}): Promise<void> {
	const cur = await currentVersion();
	updateState.set({ status: 'checking', currentVersion: cur });
	try {
		const { check } = await import('@tauri-apps/plugin-updater');
		const update = await check();
		if (update) {
			pending = update;
			updateState.set({
				status: 'available',
				currentVersion: cur ?? update.currentVersion,
				version: update.version,
				notes: update.body || undefined
			});
		} else {
			pending = null;
			updateState.set({ status: 'up-to-date', currentVersion: cur });
		}
	} catch (e) {
		pending = null;
		updateState.set(
			opts.silent
				? { status: 'unsupported', currentVersion: cur }
				: { status: 'error', currentVersion: cur, error: String(e) }
		);
	}
}

/** Download + install the pending update, then relaunch into the new version. */
export async function installUpdate(): Promise<void> {
	if (!pending) return;
	let total = 0;
	let got = 0;
	updateState.update((s) => ({ ...s, status: 'downloading', progress: 0 }));
	try {
		await pending.downloadAndInstall((e: DownloadEvent) => {
			if (e.event === 'Started') {
				total = e.data.contentLength ?? 0;
			} else if (e.event === 'Progress') {
				got += e.data.chunkLength;
				if (total > 0) updateState.update((s) => ({ ...s, progress: got / total }));
			} else if (e.event === 'Finished') {
				updateState.update((s) => ({ ...s, status: 'ready', progress: 1 }));
			}
		});
		const { relaunch } = await import('@tauri-apps/plugin-process');
		await relaunch();
	} catch (e) {
		updateState.update((s) => ({ ...s, status: 'error', error: String(e) }));
	}
}
