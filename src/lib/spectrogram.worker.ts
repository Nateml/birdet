// Runs the spectrogram STFT off the main thread — a long recording is
// thousands of FFTs, which would otherwise freeze the UI while "loading".
import { computeSpectrogram } from './spectrogram';

self.onmessage = (e: MessageEvent<{ samples: Float32Array; sampleRate: number }>) => {
    const { samples, sampleRate } = e.data;
    const spec = computeSpectrogram(samples, sampleRate);
    // Column buffers can be transferred back (main thread rebuilds the bitmap).
    const transfer = spec.columns.map((c) => c.buffer);
    (self as unknown as Worker).postMessage(spec, transfer);
};
