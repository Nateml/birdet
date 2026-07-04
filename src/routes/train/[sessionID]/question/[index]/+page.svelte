<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { session, loadQuestion, answerCurrent } from '$lib/stores/session';
	import { getRecordingBlobUrl } from '$lib/api/audio';
	import { buildSpectrogramBitmap, drawSpectrogramFrame, type Spectrogram } from '$lib/spectrogram';
	import SpectroWorker from '$lib/spectrogram.worker?worker';
	import type { QuestionDto } from '$lib/api/quiz';

	const id = page.params.sessionID;

	let question = $state<QuestionDto | null>(null);
	let audioUrl = $state<string | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	let selected = $state<string | null>(null);
	let feedback = $state<{ correct: boolean; correct_name: string; guess: string } | null>(null);
	let isPlaying = $state(false);
	let hasPlayed = $state(false);

	// Snapshot of this question's position (session.index advances on answer).
	let qNum = $state(1);
	const length = $derived($session?.config.length ?? 0);
	const answered = $derived($session?.answers ?? []);
	const scoreSoFar = $derived(answered.filter((a) => a.correct).length);
	const progressPct = $derived(length ? ((qNum - 1) / length) * 100 : 0);

	// ── Audio + spectrogram ────────────────────────────────────────
	// The spectrogram is precomputed from the decoded buffer (works with no
	// audio device); the <audio> element only handles playback sound.
	// Playback position is tracked on a wall clock so pause/seek stay in sync
	// with the animation regardless of the audio device.
	let audioEl: HTMLAudioElement;
	let canvasEl: HTMLCanvasElement | undefined = $state();
	let spec: Spectrogram | null = null;
	let bitmap: HTMLCanvasElement | null = null;
	let raf = 0;
	let loadGen = 0; // bumped each question load; guards against stale async
	let playStart = 0; // performance.now() reference for pos 0
	let posMs = $state(0); // current playback position
	let durationMs = $state(0);
	let ready = $state(false); // spectrogram computed, playback allowed
	let audioErr = $state(''); // surfaced playback error, if any

	function draw(colsToShow: number) {
		if (!canvasEl) return;
		const ctx = canvasEl.getContext('2d');
		if (!ctx) return;
		drawSpectrogramFrame(ctx, bitmap, colsToShow, canvasEl.width, canvasEl.height);
	}

	function drawAt(ms: number) {
		if (spec) draw(Math.floor(ms / spec.msPerColumn));
	}

	function tick() {
		if (!spec) return;
		posMs = performance.now() - playStart;
		if (posMs >= durationMs) {
			posMs = durationMs;
			drawAt(posMs);
			isPlaying = false;
			return;
		}
		drawAt(posMs);
		raf = requestAnimationFrame(tick);
	}

	async function play() {
		if (!spec) return;
		if (posMs >= durationMs) posMs = 0; // replay from start when finished
		hasPlayed = true;
		isPlaying = true;
		audioErr = '';
		cancelAnimationFrame(raf);
		try {
			// Only seek for a mid-clip resume. Starting from 0 (fresh or after
			// 'ended') needs no manual seek: play() restarts a finished clip on
			// its own, and skipping the currentTime=0 seek avoids the gstreamer
			// white-noise glitch. (No load() here — that races play() and mutes it.)
			if (posMs > 0) audioEl.currentTime = posMs / 1000;
			await audioEl.play();
		} catch (e) {
			audioErr = `${(e as Error)?.name ?? 'Error'}: ${(e as Error)?.message ?? e}`;
		}
		playStart = performance.now() - posMs;
		raf = requestAnimationFrame(tick);
	}

	function pause() {
		isPlaying = false;
		posMs = Math.min(performance.now() - playStart, durationMs);
		cancelAnimationFrame(raf);
		try {
			audioEl.pause();
		} catch {
			/* noop */
		}
	}

	function togglePlay() {
		if (isPlaying) pause();
		else play();
	}

	async function restart() {
		posMs = 0;
		audioErr = '';
		if (isPlaying) {
			// Pause before rewinding: seeking a *live* element to 0 glitches the
			// decoder, but rewinding while paused then replaying is clean.
			audioEl.pause();
			try {
				audioEl.currentTime = 0;
			} catch {
				/* noop */
			}
			cancelAnimationFrame(raf);
			try {
				await audioEl.play();
			} catch (e) {
				audioErr = `${(e as Error)?.name ?? 'Error'}: ${(e as Error)?.message ?? e}`;
			}
			playStart = performance.now();
			raf = requestAnimationFrame(tick);
		} else {
			try {
				audioEl.currentTime = 0;
			} catch {
				/* noop */
			}
			drawAt(0);
		}
	}

	function seek(ratio: number) {
		if (!durationMs) return;
		posMs = Math.max(0, Math.min(ratio, 1)) * durationMs;
		try {
			audioEl.currentTime = posMs / 1000;
		} catch {
			/* noop */
		}
		if (isPlaying) playStart = performance.now() - posMs;
		else drawAt(posMs);
	}

	function onSeekClick(e: MouseEvent) {
		const el = e.currentTarget as HTMLElement;
		const rect = el.getBoundingClientRect();
		seek((e.clientX - rect.left) / rect.width);
	}

	function onEnded() {
		isPlaying = false;
		posMs = durationMs;
	}

	function fmt(ms: number): string {
		const s = Math.max(0, Math.round(ms / 1000));
		return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
	}

	// Compute the spectrogram in a worker so the (potentially thousands of) FFTs
	// don't block the main thread and freeze the "loading" state.
	function computeInWorker(samples: Float32Array, sampleRate: number): Promise<Spectrogram> {
		return new Promise((resolve, reject) => {
			const w = new SpectroWorker();
			w.onmessage = (e: MessageEvent<Spectrogram>) => {
				resolve(e.data);
				w.terminate();
			};
			w.onerror = () => {
				reject(new Error('spectrogram worker failed'));
				w.terminate();
			};
			w.postMessage({ samples, sampleRate }, [samples.buffer]);
		});
	}

	async function decodeAudio(url: string, gen: number) {
		const ab = await (await fetch(url)).arrayBuffer();
		// Throwaway context just for decoding — close it so it doesn't hold the
		// audio sink (which mutes the <audio> element on WSL/PulseAudio).
		const ctx = new AudioContext();
		let buf: AudioBuffer;
		try {
			buf = await ctx.decodeAudioData(ab);
		} finally {
			ctx.close();
		}
		durationMs = buf.duration * 1000;
		// Copy channel data (transferring detaches it from the AudioBuffer).
		const samples = new Float32Array(buf.getChannelData(0));
		const result = await computeInWorker(samples, buf.sampleRate);
		if (gen !== loadGen) return; // a newer question superseded this load
		spec = result;
		bitmap = buildSpectrogramBitmap(spec.columns, 148);
		ready = true;
		drawAt(0);
	}

	// Load (and reload) the question on every navigation to this route — going
	// question/0 → question/1 reuses this component, so onMount won't re-fire.
	async function init() {
		if (!$session || $session.id !== id) {
			goto('/');
			return;
		}
		// reset per-question state
		cancelAnimationFrame(raf);
		if (audioUrl) URL.revokeObjectURL(audioUrl);
		question = null;
		audioUrl = null;
		spec = null;
		bitmap = null;
		selected = null;
		feedback = null;
		isPlaying = false;
		hasPlayed = false;
		ready = false;
		audioErr = '';
		posMs = 0;
		durationMs = 0;
		loading = true;
		error = null;
		qNum = $session.index + 1;
		const gen = ++loadGen;
		try {
			const q = await loadQuestion();
			if (gen !== loadGen) return;
			if (!q) {
				goto(`/train/${id}/result`);
				return;
			}
			question = q;
			audioUrl = await getRecordingBlobUrl(q.recording_id);
			if (gen !== loadGen) return;
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to load question.';
		} finally {
			if (gen === loadGen) loading = false;
		}
		// Decode + spectrogram run in the background so the question UI appears
		// immediately; the spectrogram fills in when the worker finishes.
		if (audioUrl) decodeAudio(audioUrl, gen).catch(() => {});
	}

	// Reload whenever the route's question index changes — this covers first
	// mount, HMR remounts, and Next (question/0 → question/1 reuses this
	// component). untrack() keeps init()'s session-store reads from turning
	// into effect dependencies (which would reload mid-question on answer).
	$effect(() => {
		page.params.index;
		untrack(() => init());
	});

	// Paint the grid once the canvas mounts, and reflect paused/seek positions.
	$effect(() => {
		if (!loading && canvasEl && !isPlaying) drawAt(posMs);
	});

	onDestroy(() => {
		cancelAnimationFrame(raf);
		try {
			audioEl?.pause();
		} catch {
			/* noop */
		}
		bitmap = null;
		if (audioUrl) URL.revokeObjectURL(audioUrl);
	});

	async function choose(choice: string) {
		if (feedback) return;
		selected = choice;
		try {
			const rec = await answerCurrent(choice);
			if (rec) feedback = { correct: rec.correct, correct_name: rec.correct_name, guess: choice };
		} catch (e) {
			selected = null;
			error = (e as Error)?.message ?? 'Failed to submit answer.';
		}
	}

	function next() {
		try {
			audioEl?.pause();
		} catch {
			/* noop */
		}
		if (!$session || $session.finished) goto(`/train/${id}/result`);
		else goto(`/train/${id}/question/${$session.index}`);
	}

	function exit() {
		try {
			audioEl?.pause();
		} catch {
			/* noop */
		}
		goto('/');
	}
</script>

<audio bind:this={audioEl} src={audioUrl ?? ''} preload="auto" onended={onEnded}></audio>

<header class="flex items-center justify-between border-b border-be-border px-6 pt-6 pb-4">
	<button
		onclick={exit}
		class="flex items-center gap-1.5 text-sm text-be-muted-fg transition-colors hover:text-be-fg"
	>
		<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg>
		<span class="hidden sm:inline">Exit training</span>
	</button>
	<div class="flex items-center gap-3">
		{#if answered.length > 0}
			<span class="font-be-mono text-xs text-be-muted-fg">{scoreSoFar}/{answered.length}</span>
		{/if}
		<span class="font-be-mono rounded-full bg-be-secondary px-2.5 py-1 text-xs text-be-secondary-fg">
			{qNum} / {length}
		</span>
	</div>
</header>

<main class="mx-auto max-w-2xl px-6 pt-6 pb-12">
	{#if loading}
		<p class="font-be-mono text-sm text-be-muted-fg">loading question…</p>
	{:else if error}
		<div class="rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">
			{error}
		</div>
	{:else if question}
		<!-- Progress -->
		<div class="mb-7 h-0.5 overflow-hidden rounded-full bg-be-muted">
			<div
				class="h-full rounded-full bg-be-primary transition-all duration-500"
				style="width: {progressPct}%"
			></div>
		</div>

		<!-- Spectrogram -->
		<div class="mb-5">
			<div class="overflow-hidden rounded-xl border border-be-border" style="line-height:0; background:#060b06">
				<canvas
					bind:this={canvasEl}
					width="600"
					height="148"
					class="block w-full"
					style="image-rendering: pixelated"
				></canvas>
			</div>
			<div class="mt-1.5 flex justify-between px-0.5">
				<span class="font-be-mono text-xs text-be-muted-fg">↑ frequency</span>
				<span class="font-be-mono text-xs text-be-muted-fg">time →</span>
			</div>
		</div>

		<!-- Audio player -->
		<div class="mb-7">
			<div class="flex items-center gap-3">
				<button
					onclick={togglePlay}
					disabled={!ready}
					class="flex w-32 items-center justify-center gap-2.5 rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-all hover:opacity-90 active:scale-[0.98] disabled:opacity-50"
				>
					{#if isPlaying}
						<svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/></svg>
						Pause
					{:else}
						<svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor"><path d="M6 4v16l14-8z"/></svg>
						{posMs > 0 && posMs < durationMs ? 'Resume' : hasPlayed ? 'Replay' : 'Play Call'}
					{/if}
				</button>
				<button
					onclick={restart}
					disabled={!hasPlayed}
					title="Restart"
					aria-label="Restart"
					class="flex items-center justify-center rounded-lg border border-be-border p-2.5 text-be-muted-fg transition-colors hover:border-be-primary/40 hover:text-be-fg disabled:opacity-40"
				>
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 2v6h6"/><path d="M3 13a9 9 0 1 0 3-7.7L3 8"/></svg>
				</button>
				<span class="font-be-mono ml-auto text-xs tabular-nums text-be-muted-fg">
					{fmt(posMs)} / {fmt(durationMs)}
				</span>
			</div>
			<!-- Seekable playback position -->
			<button
				type="button"
				onclick={onSeekClick}
				aria-label="Seek"
				class="mt-3 block h-1.5 w-full overflow-hidden rounded-full bg-be-muted"
			>
				<div
					class="h-full rounded-full bg-be-primary"
					style="width: {durationMs ? (posMs / durationMs) * 100 : 0}%"
				></div>
			</button>
			{#if audioErr}
				<p class="font-be-mono mt-2 text-xs text-be-destructive">{audioErr}</p>
			{:else if !ready}
				<p class="font-be-mono mt-2 text-xs text-be-muted-fg">analyzing audio…</p>
			{:else if !hasPlayed}
				<p class="font-be-mono mt-2 text-xs text-be-muted-fg">press play to hear the bird</p>
			{/if}
		</div>

		<!-- Choices -->
		<p class="font-be-mono mb-3 text-xs uppercase tracking-widest text-be-muted-fg">Identify the bird</p>
		<div class="grid grid-cols-2 gap-3">
			{#each question.choices as choice (choice)}
				{@const isCorrectChoice = feedback ? choice === feedback.correct_name : false}
				{@const isSelected = choice === selected}
				<button
					onclick={() => choose(choice)}
					disabled={!!feedback}
					class="rounded-xl border p-4 text-left transition-all duration-200
						{!feedback
						? 'border-be-border bg-be-card hover:border-be-primary/40 hover:bg-be-secondary active:scale-[0.99] cursor-pointer'
						: isCorrectChoice
							? 'border-emerald-500/50 bg-emerald-950/35 cursor-default'
							: isSelected
								? 'border-red-500/40 bg-red-950/30 cursor-default'
								: 'border-be-border/30 bg-be-card/40 opacity-40 cursor-default'}"
				>
					<div class="flex items-start gap-2">
						{#if feedback && isCorrectChoice}
							<svg class="mt-0.5 shrink-0 text-emerald-400" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.801 10A10 10 0 1 1 17 3.335"/><path d="m9 11 3 3L22 4"/></svg>
						{:else if feedback && isSelected}
							<svg class="mt-0.5 shrink-0 text-red-400" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/></svg>
						{/if}
						<span class="text-sm font-semibold leading-snug">{choice}</span>
					</div>
				</button>
			{/each}
		</div>

		<!-- Feedback + next -->
		{#if feedback}
			<div class="mt-5 flex items-center justify-between gap-4">
				<p class="text-sm font-semibold {feedback.correct ? 'text-emerald-400' : 'text-red-400'}">
					{feedback.correct ? 'Correct!' : `That was ${feedback.correct_name}.`}
				</p>
				<button
					onclick={next}
					class="flex shrink-0 items-center gap-1.5 rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
				>
					{$session?.finished ? 'See Results' : 'Next Bird'}
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
				</button>
			</div>
		{/if}
	{/if}
</main>
