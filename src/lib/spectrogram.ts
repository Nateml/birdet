// Spectrogram rendering + offline STFT, based on the BirdEar Figma prototype.
// The prototype tapped a live AnalyserNode; we instead precompute columns from
// the decoded AudioBuffer so the visualization works even when the platform has
// no audio output device (e.g. WSL) and stays independent of playback timing.

const FFT_SIZE = 2048;
const HOP = 512;
const MAX_FREQ_BIN = 380; // ~0–8kHz with fftSize 2048 @ 44100 Hz

// In-place iterative radix-2 FFT (Cooley–Tukey). re/im are length n (power of 2).
function fft(re: Float32Array, im: Float32Array): void {
    const n = re.length;
    for (let i = 1, j = 0; i < n; i++) {
        let bit = n >> 1;
        for (; j & bit; bit >>= 1) j ^= bit;
        j ^= bit;
        if (i < j) {
            [re[i], re[j]] = [re[j], re[i]];
            [im[i], im[j]] = [im[j], im[i]];
        }
    }
    for (let len = 2; len <= n; len <<= 1) {
        const ang = (-2 * Math.PI) / len;
        const wr = Math.cos(ang);
        const wi = Math.sin(ang);
        for (let i = 0; i < n; i += len) {
            let cr = 1;
            let ci = 0;
            for (let k = 0; k < len / 2; k++) {
                const a = i + k;
                const b = a + len / 2;
                const tr = re[b] * cr - im[b] * ci;
                const ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                const ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
    }
}

export interface Spectrogram {
    columns: Uint8Array[]; // one per STFT frame; bins 0..FFT_SIZE/2
    msPerColumn: number; // real playback ms represented by each column
}

// Precompute a magnitude spectrogram for the whole clip.
export function computeSpectrogram(samples: Float32Array, sampleRate: number): Spectrogram {
    const n = FFT_SIZE;
    // Hann window
    const win = new Float32Array(n);
    for (let i = 0; i < n; i++) win[i] = 0.5 - 0.5 * Math.cos((2 * Math.PI * i) / (n - 1));

    const minDb = -95;
    const maxDb = -25;
    const columns: Uint8Array[] = [];
    const re = new Float32Array(n);
    const im = new Float32Array(n);

    for (let start = 0; start + n <= samples.length; start += HOP) {
        for (let i = 0; i < n; i++) {
            re[i] = samples[start + i] * win[i];
            im[i] = 0;
        }
        fft(re, im);
        const col = new Uint8Array(n / 2);
        for (let b = 0; b < n / 2; b++) {
            const mag = Math.sqrt(re[b] * re[b] + im[b] * im[b]) / (n / 2);
            const db = 20 * Math.log10(mag + 1e-12);
            let v = (db - minDb) / (maxDb - minDb);
            v = v < 0 ? 0 : v > 1 ? 1 : v;
            col[b] = Math.round(v * 255);
        }
        columns.push(col);
    }

    return { columns, msPerColumn: (HOP / sampleRate) * 1000 };
}

function spectroColor(v: number): [number, number, number] {
    if (v < 0.12) {
        const t = v / 0.12;
        return [0, Math.round(t * 28), Math.round(t * 44)];
    }
    if (v < 0.3) {
        const t = (v - 0.12) / 0.18;
        return [0, 28 + Math.round(t * 90), 44 + Math.round(t * 110)];
    }
    if (v < 0.55) {
        const t = (v - 0.3) / 0.25;
        return [Math.round(t * 40), 118 + Math.round(t * 120), 154 - Math.round(t * 34)];
    }
    if (v < 0.78) {
        const t = (v - 0.55) / 0.23;
        return [40 + Math.round(t * 170), 238 + Math.round(t * 17), 120 - Math.round(t * 70)];
    }
    const t = (v - 0.78) / 0.22;
    return [210 + Math.round(t * 45), 255, 50 + Math.round(t * 205)];
}

// Render the whole spectrogram to an offscreen bitmap ONCE. Playback then just
// blits the revealed slice each frame (cheap) instead of re-running a full
// per-frame pixel loop — which stutters/freezes the UI under software rendering.
export function buildSpectrogramBitmap(columns: Uint8Array[], H: number): HTMLCanvasElement {
    const W = Math.max(1, columns.length);
    const canvas = document.createElement('canvas');
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext('2d')!;
    const img = ctx.createImageData(W, H);
    const d = img.data;

    for (let i = 0; i < d.length; i += 4) {
        d[i] = 6;
        d[i + 1] = 11;
        d[i + 2] = 6;
        d[i + 3] = 255;
    }

    for (let xi = 0; xi < columns.length; xi++) {
        const col = columns[xi];
        for (let yi = 0; yi < H; yi++) {
            const bin = Math.min(Math.floor((1 - yi / H) * MAX_FREQ_BIN), col.length - 1);
            const [r, g, b] = spectroColor(col[bin] / 255);
            const idx = (yi * W + xi) * 4;
            d[idx] = r;
            d[idx + 1] = g;
            d[idx + 2] = b;
            d[idx + 3] = 255;
        }
    }

    ctx.putImageData(img, 0, 0);
    return canvas;
}

function drawGrid(ctx: CanvasRenderingContext2D, W: number, H: number): void {
    ctx.strokeStyle = 'rgba(82,232,118,0.16)';
    ctx.fillStyle = 'rgba(82,232,118,0.5)';
    ctx.font = "9px 'JetBrains Mono', monospace";
    ctx.lineWidth = 0.5;
    [8000, 6000, 4000, 2000, 1000].forEach((freq) => {
        const y = H - (freq / 8000) * H;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(W, y);
        ctx.stroke();
        ctx.fillText(freq >= 1000 ? `${freq / 1000}kHz` : `${freq}Hz`, 5, y - 2);
    });
}

// Draw one playback frame: background, the revealed portion of the precomputed
// bitmap (scrolling once it exceeds the viewport), grid, and a scan cursor.
export function drawSpectrogramFrame(
    ctx: CanvasRenderingContext2D,
    bitmap: HTMLCanvasElement | null,
    colsToShow: number,
    W: number,
    H: number
): void {
    ctx.fillStyle = '#060b06';
    ctx.fillRect(0, 0, W, H);

    const total = bitmap ? bitmap.width : 0;
    const end = Math.max(0, Math.min(colsToShow, total));
    if (bitmap && end > 0) {
        const srcX = Math.max(0, end - W);
        const srcW = end - srcX;
        ctx.imageSmoothingEnabled = false;
        ctx.drawImage(bitmap, srcX, 0, srcW, H, 0, 0, srcW, H);
        if (end < W) {
            ctx.fillStyle = 'rgba(82,232,118,0.25)';
            ctx.fillRect(end - 1, 0, 1, H);
        }
    }

    drawGrid(ctx, W, H);
}
