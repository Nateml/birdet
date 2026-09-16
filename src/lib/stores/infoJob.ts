import { writable, get } from 'svelte/store';
import { onImportProgress } from '$lib/api/import';
import { backfillBirdInfo, isBackfillRunning, cancelBackfill } from '$lib/api/info';

// Global, app-lifetime state for a species-notes run, mirroring `importJob`.
// The run is long (about a second per bird) and lives in the backend, so it
// keeps going when you leave Settings — this store is what lets the indicator
// follow it anywhere and what stops a second run being started on return.
export type InfoJob = {
    active: boolean;
    force: boolean; // a re-fetch of everything, not just the missing ones
    stopping: boolean; // a stop was asked for; the run ends after the current bird
    current: number;
    total: number;
    message: string;
    result: string | null; // the last run's summary, kept so Settings can show it
};

const empty: InfoJob = {
    active: false,
    force: false,
    stopping: false,
    current: 0,
    total: 0,
    message: '',
    result: null
};

export const infoJob = writable<InfoJob>({ ...empty });

// Register the backend progress listener exactly once, lazily.
let listening = false;
async function ensureListener() {
    if (listening) return;
    listening = true;
    await onImportProgress((p) => {
        if (p.stage !== 'info') return;
        infoJob.update((j) =>
            j.active ? { ...j, current: p.current, total: p.total, message: p.message } : j
        );
    });
}

// A run started before this window existed (a reload, or a fetch kicked off by
// an import) is still going in the backend. Adopt it so the UI doesn't offer a
// button the backend would refuse.
//
// There's no promise to await for someone else's run, so poll the flag to learn
// when it ends. An import's background fill emits no progress events, so this is
// also the only thing that clears the indicator for those.
const ADOPT_POLL_MS = 2000;
let polling = false;

export async function adoptRunningInfoJob(): Promise<void> {
    if (get(infoJob).active || polling) return;
    try {
        if (!(await isBackfillRunning())) return;
    } catch {
        return; // not in Tauri, or the command failed — leave the store idle
    }
    await ensureListener();
    infoJob.update((j) => ({ ...j, active: true, message: 'Looking up species…' }));

    polling = true;
    try {
        for (;;) {
            await new Promise((r) => setTimeout(r, ADOPT_POLL_MS));
            // A locally-started run took over; it owns `active` from here.
            if (!polling) return;
            let running = false;
            try {
                running = await isBackfillRunning();
            } catch {
                running = false; // can't tell any more — don't spin forever
            }
            if (!running) {
                infoJob.update((j) => ({ ...j, active: false, current: 0, total: 0, message: '' }));
                return;
            }
        }
    } finally {
        polling = false;
    }
}

// Run a backfill through the global guard. Throws if one is already in flight —
// the backend enforces the same rule, this just avoids the round trip.
export async function runInfoJob(force = false): Promise<number> {
    if (get(infoJob).active) {
        throw new Error('A species-notes lookup is already running — let it finish first.');
    }
    await ensureListener();
    polling = false; // this run owns the store now; stop any adopted poll
    infoJob.set({ ...empty, active: true, force, message: 'Starting…' });
    try {
        const n = await backfillBirdInfo(force);
        // A stopped run still reports what it managed — every bird is saved as
        // it's fetched, so the work up to the stop is kept.
        const stopped = get(infoJob).stopping;
        const result = stopped
            ? n > 0
                ? `Stopped — notes for ${n} species were saved.`
                : 'Stopped before any notes were found.'
            : n > 0
                ? force
                    ? `Re-fetched notes for ${n} species.`
                    : `Added notes for ${n} species.`
                : force
                    ? 'No notes found for any species.'
                    : 'No new notes found — every bird has already been looked up.';
        infoJob.update((j) => ({ ...j, result }));
        return n;
    } catch (e) {
        infoJob.update((j) => ({ ...j, result: (e as string)?.toString() ?? 'Lookup failed.' }));
        throw e;
    } finally {
        infoJob.update((j) => ({
            ...j,
            active: false,
            stopping: false,
            current: 0,
            total: 0,
            message: ''
        }));
    }
}

// Ask the backend to stop after the bird it's on. The run's own promise still
// settles normally, so `runInfoJob`'s finally clause does the cleanup.
export async function stopInfoJob(): Promise<void> {
    if (!get(infoJob).active) return;
    infoJob.update((j) => ({ ...j, stopping: true, message: 'Stopping…' }));
    try {
        await cancelBackfill();
    } catch {
        infoJob.update((j) => ({ ...j, stopping: false }));
    }
}
