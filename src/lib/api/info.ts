import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust `services::info::BirdInfo`. Every text field is optional: a
// species may have no Wikipedia article, or an article with no voice section.
export type BirdInfo = {
    bird_id: number;
    summary: string | null;
    appearance: string | null;
    habitat: string | null;
    behaviour: string | null;
    voice: string | null;
    conservation: string | null;
    image_url: string | null;
    image_path: string | null;
    image_credit: string | null;
    image_license: string | null;
    image_license_url: string | null;
    image_source_url: string | null;
    source: string | null;
    source_url: string | null;
    license: string | null;
    fetched_at: string | null;
    user_notes: string | null;
};

export async function getBirdInfo(birdId: number): Promise<BirdInfo | null> {
    return await invoke<BirdInfo | null>('get_bird_info', { birdId });
}

/// Fetch one species' blurb. Returns true if any text was stored. Without
/// `force`, a species already tried once is left alone.
export async function fetchBirdInfo(birdId: number, force = false): Promise<boolean> {
    return await invoke<boolean>('fetch_bird_info', { birdId, force });
}

/// Fill in every bird missing a blurb (or all of them, with `force`).
/// Reports progress on the `import://progress` event under the `info` stage.
export async function backfillBirdInfo(force = false): Promise<number> {
    return await invoke<number>('backfill_bird_info', { force });
}

/// Is a backfill already in flight in the backend? Survives a window reload, so
/// the UI can pick an in-progress run back up.
export async function isBackfillRunning(): Promise<boolean> {
    return await invoke<boolean>('bird_info_backfill_running');
}

/// Ask a running backfill to stop after the bird it's on. Safe when idle.
export async function cancelBackfill(): Promise<void> {
    await invoke('cancel_bird_info_backfill');
}

export async function setBirdNotes(birdId: number, notes: string | null): Promise<void> {
    await invoke('set_bird_notes', { birdId, notes });
}

/// Does this blurb have anything worth rendering?
export function hasInfoText(info: BirdInfo | null): boolean {
    return !!(
        info &&
        (info.summary || info.appearance || info.habitat || info.behaviour || info.voice)
    );
}

/// Load a bird's downloaded photo as a same-origin blob URL. Mirrors the
/// recordings path — bytes over IPC rather than the asset protocol, so the
/// images dir needs no asset scope. Caller must URL.revokeObjectURL() when done.
export async function getBirdImageBlobUrl(birdId: number): Promise<string> {
    const buf = await invoke<ArrayBuffer>('get_bird_image_bytes', { birdId });
    const bytes = new Uint8Array(buf);
    const blob = new Blob([bytes], { type: sniffImageMime(bytes) });
    return URL.createObjectURL(blob);
}

// webkit2gtk doesn't sniff types on blob URLs, so an untyped blob renders as a
// broken image. Identify the format from its magic bytes instead.
function sniffImageMime(b: Uint8Array): string {
    if (b.length >= 4) {
        if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47) return 'image/png';
        if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return 'image/jpeg';
        if (b[0] === 0x47 && b[1] === 0x49 && b[2] === 0x46) return 'image/gif';
        if (
            b.length >= 12 &&
            b[0] === 0x52 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x46 &&
            b[8] === 0x57 && b[9] === 0x45 && b[10] === 0x42 && b[11] === 0x50
        )
            return 'image/webp'; // RIFF....WEBP
    }
    return 'image/jpeg';
}
