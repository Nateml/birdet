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

export async function searchBirdRecordings(
    birdId: number,
    quality?: string,
    recType?: string
): Promise<RecordingCandidate[]> {
    return await invoke<RecordingCandidate[]>('search_bird_recordings', {
        birdId,
        quality,
        recType
    });
}

export async function addBirdRecordings(birdId: number, xcIds: string[]): Promise<number> {
    return await invoke<number>('add_bird_recordings', { birdId, xcIds });
}

export async function backfillRecordingMeta(): Promise<number> {
    return await invoke<number>('backfill_recording_meta');
}
