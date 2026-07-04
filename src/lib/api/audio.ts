import { invoke, convertFileSrc } from '@tauri-apps/api/core';

// Resolve a recording to a URL the <audio> tag can load (via Tauri's asset protocol).
export async function getRecordingUrl(recording_id: number): Promise<string> {
    const path = await invoke<string>('get_recording_path', { recordingId: recording_id });
    return convertFileSrc(path);
}

// Load a recording's bytes and wrap them in a same-origin blob URL. Unlike the
// asset-protocol URL, a blob is same-origin, so a Web Audio MediaElementSource
// on it isn't CORS-tainted — required for the live spectrogram. Caller must
// URL.revokeObjectURL() when done.
export async function getRecordingBlobUrl(recording_id: number): Promise<string> {
    const buf = await invoke<ArrayBuffer>('get_recording_bytes', { recordingId: recording_id });
    const blob = new Blob([buf instanceof ArrayBuffer ? buf : new Uint8Array(buf)]);
    return URL.createObjectURL(blob);
}
