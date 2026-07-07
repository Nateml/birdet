import { writable, get } from 'svelte/store';
import { onImportProgress } from '$lib/api/import';

// Global, app-lifetime state for an in-flight import (region / species / pack).
// Lives outside any component so progress survives navigation and a single
// indicator can reflect it anywhere. Also enforces one-import-at-a-time.
export type ImportJob = {
    active: boolean;
    label: string; // what's importing, e.g. "GB-ENG" or "pack"
    current: number;
    total: number;
    message: string;
};

const empty: ImportJob = { active: false, label: '', current: 0, total: 0, message: '' };
export const importJob = writable<ImportJob>({ ...empty });

// Register the backend progress listener exactly once, lazily.
let listening = false;
async function ensureListener() {
    if (listening) return;
    listening = true;
    await onImportProgress((p) => {
        importJob.update((j) =>
            j.active ? { ...j, current: p.current, total: p.total, message: p.message } : j
        );
    });
}

export function importRunning(): boolean {
    return get(importJob).active;
}

// Run an import through the global guard. Throws if one is already running.
// Clears `active` when the backend call settles.
export async function runImportJob<T>(label: string, fn: () => Promise<T>): Promise<T> {
    if (get(importJob).active) {
        throw new Error('An import is already running — let it finish first.');
    }
    await ensureListener();
    importJob.set({ active: true, label, current: 0, total: 0, message: 'Starting…' });
    try {
        return await fn();
    } finally {
        importJob.update((j) => ({ ...j, active: false }));
    }
}
