import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust `import::RecordingInfo`.
export type RecordingInfo = {
    id: number;
    filename: string | null;
    source: string | null;
    xc_id: string | null;
    recordist: string | null;
    license_url: string | null;
    location: string | null;
    quality: string | null;
    rec_type: string | null;
    background: string[]; // other species in the clip (common names where known)
};

// Mirrors Rust `import::RecordingCandidate`.
export type RecordingCandidate = {
    xc_id: string;
    recordist: string;
    quality: string;
    length: string;
    rec_type: string;
    location: string;
    license_url: string;
};

export async function getBirdRecordings(birdId: number): Promise<RecordingInfo[]> {
    return await invoke<RecordingInfo[]>('get_bird_recordings', { birdId });
}

export async function deleteRecording(recordingId: number): Promise<void> {
    await invoke('delete_recording', { recordingId });
}

export async function deleteBird(birdId: number): Promise<void> {
    await invoke('delete_bird', { birdId });
}

// Mirrors Rust `import::RecordingSearch`.
export type RecordingSearch = {
    candidates: RecordingCandidate[];
    // Results came from the English-name fallback, not the eBird scientific
    // name — the recordings may be filed under a different species name.
    name_fallback: boolean;
    scientific_name: string; // the eBird name we queried
    xc_name: string | null; // the name Xeno-Canto files these under (fallback only)
};

export async function searchBirdRecordings(
    birdId: number,
    quality?: string,
    recType?: string
): Promise<RecordingSearch> {
    return await invoke<RecordingSearch>('search_bird_recordings', {
        birdId,
        quality,
        recType
    });
}

export async function addBirdRecordings(birdId: number, xcIds: string[]): Promise<number> {
    return await invoke<number>('add_bird_recordings', { birdId, xcIds });
}

// Mirrors Rust `import::AddByNumberResult`.
export type AddByNumberResult = {
    added: boolean; // false = already in the library
    xc_id: string;
    recordist: string;
    en: string; // XC English name of the recording's species
    xc_name: string; // XC binomial
    quality: string;
    rec_type: string;
};

// Add one specific recording by its Xeno-Canto catalogue number (accepts
// "XC123456", "123456", or an xeno-canto.org URL).
export async function addRecordingByNumber(
    birdId: number,
    catalogue: string
): Promise<AddByNumberResult> {
    return await invoke<AddByNumberResult>('add_recording_by_number', { birdId, catalogue });
}

export async function backfillRecordingMeta(): Promise<number> {
    return await invoke<number>('backfill_recording_meta');
}

// Mirrors Rust `commands::RecordingStorage`.
export type RecordingStorage = { bytes: number; file_count: number; path: string };

// Total disk usage of downloaded recordings (excludes bundled seed audio).
export async function recordingsStorage(): Promise<RecordingStorage> {
    return await invoke<RecordingStorage>('recordings_storage');
}

// Open the recordings folder in the OS file manager.
export async function openRecordingsFolder(): Promise<void> {
    await invoke('open_recordings_folder');
}

export type RepairSummary = { checked: number; repaired: number; failed: number };

// Re-download any Xeno-Canto recordings whose local audio file is missing or corrupt.
export async function repairRecordings(): Promise<RepairSummary> {
    return await invoke<RepairSummary>('repair_recordings');
}
