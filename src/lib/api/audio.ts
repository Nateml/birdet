import { invoke, convertFileSrc } from '@tauri-apps/api/core';

// Resolve a recording to a URL the <audio> tag can load (via Tauri's asset protocol).
export async function getRecordingUrl(recording_id: number): Promise<string> {
    const path = await invoke<string>('get_recording_path', { recordingId: recording_id });
    return convertFileSrc(path);
}

// Guess a media MIME type from a file's magic bytes. webkit2gtk (the Linux/WSL
// webview) does NOT sniff media on blob URLs — an untyped blob has an empty
// Content-Type, so the <audio> element rejects it with "operation not supported".
// Setting the right type makes playback work. Defaults to audio/mpeg (most
// recordings are mp3).
function sniffAudioMime(b: Uint8Array): string {
    if (b.length >= 4) {
        if (b[0] === 0x52 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x46) return 'audio/wav'; // RIFF
        if (b[0] === 0x4f && b[1] === 0x67 && b[2] === 0x67 && b[3] === 0x53) return 'audio/ogg'; // OggS
        if (b[0] === 0x66 && b[1] === 0x4c && b[2] === 0x61 && b[3] === 0x43) return 'audio/flac'; // fLaC
        if (b[0] === 0x49 && b[1] === 0x44 && b[2] === 0x33) return 'audio/mpeg'; // ID3 (mp3)
        if (b[0] === 0xff && (b[1] & 0xe0) === 0xe0) return 'audio/mpeg'; // MPEG frame sync
        if (b.length >= 8 && b[4] === 0x66 && b[5] === 0x74 && b[6] === 0x79 && b[7] === 0x70)
            return 'audio/mp4'; // ftyp (m4a/aac)
    }
    return 'audio/mpeg';
}

// Load a recording's bytes and wrap them in a same-origin blob URL. Unlike the
// asset-protocol URL, a blob is same-origin, so a Web Audio MediaElementSource
// on it isn't CORS-tainted — required for the live spectrogram. Caller must
// URL.revokeObjectURL() when done.
export async function getRecordingBlobUrl(recording_id: number): Promise<string> {
    const buf = await invoke<ArrayBuffer>('get_recording_bytes', { recordingId: recording_id });
    const bytes = buf instanceof ArrayBuffer ? new Uint8Array(buf) : new Uint8Array(buf);
    const blob = new Blob([bytes], { type: sniffAudioMime(bytes) });
    return URL.createObjectURL(blob);
}

// Fetch a recording's bytes ONCE and return both a playable blob URL and the
// raw buffer. The quiz needs both — the <audio> element plays the URL and the
// spectrogram decodes the bytes — and fetching the blob URL a second time to
// decode it makes webkit's media element fail to load (silent playback, worst
// on large clips). Sharing one read avoids that. The blob copies the bytes at
// construction, so the returned buffer is safe to hand to decodeAudioData
// (which detaches it) without affecting playback.
export async function getRecordingAudio(
    recording_id: number
): Promise<{ url: string; buffer: ArrayBuffer }> {
    const buf = await invoke<ArrayBuffer>('get_recording_bytes', { recordingId: recording_id });
    const bytes = new Uint8Array(buf);
    const blob = new Blob([bytes], { type: sniffAudioMime(bytes) });
    return { url: URL.createObjectURL(blob), buffer: buf };
}
