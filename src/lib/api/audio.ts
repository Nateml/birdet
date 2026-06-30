import { invoke, convertFileSrc } from '@tauri-apps/api/core';

// Resolve a recording to a URL the <audio> tag can load (via Tauri's asset protocol).
export async function getRecordingUrl(recording_id: number): Promise<string> {
    const path = await invoke<string>('get_recording_path', { recording_id });
    return convertFileSrc(path);
}
