<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { get } from 'svelte/store';
	import { session, loadQuestion, answerCurrent, continueTraining } from '$lib/stores/session';
	import { setup } from '$lib/stores/setup';
	import { getRecordingAudio } from '$lib/api/audio';
	import { buildSpectrogramBitmap, drawSpectrogramFrame, type Spectrogram } from '$lib/spectrogram';
	import SpectroWorker from '$lib/spectrogram.worker?worker';
	import type { QuestionDto } from '$lib/api/quiz';
	import { licenseLabel, licenseHref, xcUrl } from '$lib/license';

	const id = page.params.sessionID;

	let question = $state<QuestionDto | null>(null);
	let audioUrl = $state<string | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Shown when the scheduled queue drains: offer summary-or-continue instead of
	// jumping straight to results.
	let completed = $state(false);

	let selected = $state<string | null>(null);
	let feedback = $state<{
		correct: boolean;
		correct_name: string;
		guess: string;
		skipped: boolean;
	} | null>(null);
	let isPlaying = $state(false);
	let hasPlayed = $state(false);

	// Snapshot of this question's position (session.index advances on answer).
	let qNum = $state(1);
	// Anki-style: `length` is the new-card cap, not a fixed session length. Progress
	// tracks new birds introduced; once the cap is hit the session keeps draining
	// due/learning cards (the "review" phase) until the queue empties.
	const newCap = $derived($session?.config.length ?? 0);
	const newServed = $derived($session?.newServed ?? 0);
	const answered = $derived($session?.answers ?? []);
	const scoreSoFar = $derived(answered.filter((a) => a.correct).length);
	const study = $derived($session?.config.study ?? 'mixed');
	const isCram = $derived(study === 'cram');
	// Whether the card on screen is a brand-new bird vs a scheduled review.
	const currentIsNew = $derived(question?.is_new ?? false);
	// New-bird budget introduced so far, counting the current card if it's new.
	const newShown = $derived(Math.min(newCap, newServed + (currentIsNew ? 1 : 0)));
	// Modes that introduce new birds show the new-bird budget counter.
	const showsNewBudget = $derived(study === 'new' || study === 'mixed');
	// Live "cards left" for the drain phase (new + learning + due). Includes the
	// card on screen, so it counts down to 0 as the queue empties — giving a clear
	// sense of when the session ends. Null for cram (fixed length) / before counted.
	const remaining = $derived($session?.remaining ?? null);
	// Min questions to finish if all correct (drops per correct answer, rises on a miss).
	const toGo = $derived(remaining?.to_go ?? null);
	// Study-ahead mode (user kept training past the scheduled queue).
	const extended = $derived($session?.extended ?? false);
	// The progress bar is meaningful only when there's a known target: the
	// new-card cap (new/mixed) or the fixed cram length. A review-only run
	// drains an unknown number of due cards, so it has no deterministic bar.
	const showBar = $derived((isCram || showsNewBudget) && newCap > 0);
	const progressPct = $derived(
		isCram
			? Math.min(100, (answered.length / newCap) * 100)
			: Math.min(100, (newServed / newCap) * 100)
	);

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
	let ready = $state(false); // spectrogram computed
	let audioReady = $state(false); // <audio> has usable metadata
	let specFailed = $state(false); // spectrogram decode failed (playback still works)
	let audioErr = $state(''); // surfaced playback error, if any
	// Playback is allowed as soon as *either* the spectrogram is ready or the
	// <audio> element has metadata. Decoupling them means a file the Web Audio
	// decoder chokes on can still be played and heard (it just lacks a spectrogram).
	const canPlay = $derived(ready || audioReady);

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
		if (!durationMs) return;
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

	// `silent` suppresses error surfacing — used for autoplay, where the browser
	// may block playback (NotAllowedError) and we just fall back to a manual press.
	async function play(silent = false) {
		if (!durationMs) return;
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
			if (silent) {
				isPlaying = false;
				hasPlayed = false;
				return;
			}
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

	// The <audio> element decodes the real file — trust its duration over the
	// throwaway Web Audio decode (which can fail while playback still works).
	function onLoadedMetadata() {
		const d = audioEl?.duration;
		if (Number.isFinite(d) && d > 0) durationMs = d * 1000;
		audioReady = true;
	}

	// The element couldn't load/decode this source: bad codec, or a corrupt or
	// truncated download. Surface it (with the recording id) so the user can jump
	// over and replace it, instead of a silent 0:00 clip.
	function onAudioError() {
		const err = audioEl?.error;
		if (!err || !audioUrl) return; // ignore the transient error from src=''
		const codes: Record<number, string> = {
			1: 'load aborted',
			2: 'network error',
			3: 'decode failed',
			4: 'format not supported'
		};
		isPlaying = false;
		cancelAnimationFrame(raf);
		audioErr = `Can't play recording #${question?.recording_id}: ${codes[err.code] ?? 'error'}.`;
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

	async function decodeAudio(ab: ArrayBuffer, gen: number) {
		// Decode with an OfflineAudioContext, NOT a real AudioContext: a real
		// context grabs the audio device/sink, which on WSL/PulseAudio disrupts
		// the <audio> element (it fails to load with "operation not supported",
		// worst for longer clips whose decode holds the sink longer). Offline
		// rendering never touches the device, so playback and analysis coexist.
		const OfflineCtx =
			window.OfflineAudioContext ?? (window as unknown as { webkitOfflineAudioContext: typeof OfflineAudioContext }).webkitOfflineAudioContext;
		const ctx = new OfflineCtx(1, 1, 48000);
		const buf: AudioBuffer = await ctx.decodeAudioData(ab);
		durationMs = buf.duration * 1000;
		// Copy channel data (transferring detaches it from the AudioBuffer).
		const samples = new Float32Array(buf.getChannelData(0));
		const result = await computeInWorker(samples, buf.sampleRate);
		if (gen !== loadGen) return; // a newer question superseded this load
		spec = result;
		bitmap = buildSpectrogramBitmap(spec.columns, 148);
		ready = true;
		drawAt(0);
		// Auto-play once analysis finishes (opt-out in Settings). Silent: if the
		// webview blocks autoplay, the user just presses Play — no scary error.
		if (get(setup).autoplay && !feedback) play(true);
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
		completed = false;
		isPlaying = false;
		hasPlayed = false;
		ready = false;
		audioReady = false;
		specFailed = false;
		audioErr = '';
		posMs = 0;
		durationMs = 0;
		loading = true;
		error = null;
		qNum = $session.index + 1;
		const gen = ++loadGen;
		let audioBuf: ArrayBuffer | null = null;
		try {
			const q = await loadQuestion();
			if (gen !== loadGen) return;
			if (!q) {
				// No card. If the queue merely drained (not truly finished), show the
				// "scheduled cards done" screen; otherwise go to the results summary.
				if ($session && !$session.finished) {
					completed = true;
					loading = false;
					return;
				}
				goto(`/train/${id}/result`);
				return;
			}
			question = q;
			// One read: blob URL for the <audio> element + buffer for the
			// spectrogram. Fetching the blob URL a second time to decode it breaks
			// webkit's media element (silent playback), so reuse this buffer.
			const audio = await getRecordingAudio(q.recording_id);
			audioUrl = audio.url;
			audioBuf = audio.buffer;
			if (gen !== loadGen) return;
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to load question.';
		} finally {
			if (gen === loadGen) loading = false;
		}
		// Decode + spectrogram run in the background so the question UI appears
		// immediately; the spectrogram fills in when the worker finishes. If it
		// fails, playback still works via the <audio> element — just flag it so
		// the "analyzing…" state clears instead of hanging with a 0:00 clip.
		if (audioBuf)
			decodeAudio(audioBuf, gen).catch(() => {
				if (gen === loadGen) specFailed = true;
			});
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
			if (rec)
				feedback = {
					correct: rec.correct,
					correct_name: rec.correct_name,
					guess: choice,
					skipped: false
				};
		} catch (e) {
			selected = null;
			error = (e as Error)?.message ?? 'Failed to submit answer.';
		}
	}

	// "I don't know" — reveal the answer without a guess. Counts as not-correct.
	async function skip() {
		if (feedback) return;
		selected = null;
		try {
			const rec = await answerCurrent('', true);
			if (rec)
				feedback = {
					correct: false,
					correct_name: rec.correct_name,
					guess: '',
					skipped: true
				};
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to submit answer.';
		}
	}

	// Keyboard: Space/K play·pause, R replay, 1-9 pick, S skip, Enter/→ next.
	function onKey(e: KeyboardEvent) {
		const t = e.target as HTMLElement | null;
		if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
		if (loading || error || !question) return;
		const k = e.key;
		if (k === ' ' || k === 'k') {
			e.preventDefault();
			togglePlay();
		} else if (k === 'r' || k === 'R') {
			e.preventDefault();
			restart();
		} else if (!feedback) {
			if (k >= '1' && k <= '9') {
				const idx = +k - 1;
				if (idx < question.choices.length) {
					e.preventDefault();
					choose(question.choices[idx]);
				}
			} else if (k === 's' || k === 'S') {
				e.preventDefault();
				skip();
			}
		} else if (k === 'Enter' || k === 'ArrowRight') {
			e.preventDefault();
			next();
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

	// From the "scheduled cards done" screen: go to the results summary.
	function toSummary() {
		try {
			audioEl?.pause();
		} catch {
			/* noop */
		}
		goto(`/train/${id}/result`);
	}

	// From the "scheduled cards done" screen: keep going in study-ahead mode.
	async function keepTraining() {
		continueTraining();
		completed = false;
		await init();
	}

	// Jump to this recording on its bird's page (to replace/delete a bad one).
	// Only reachable after answering, so it doesn't spoil the identity.
	function manageRecording() {
		if (!question) return;
		try {
			audioEl?.pause();
		} catch {
			/* noop */
		}
		const back = encodeURIComponent(page.url.pathname);
		goto(`/birds/${question.bird_id}?rec=${question.recording_id}&return=${back}`);
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

<svelte:window onkeydown={onKey} />

<audio
	bind:this={audioEl}
	src={audioUrl ?? ''}
	preload="auto"
	onloadedmetadata={onLoadedMetadata}
	onerror={onAudioError}
	onended={onEnded}
></audio>

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
		{#if isCram}
			<span class="font-be-mono rounded-full bg-be-secondary px-2.5 py-1 text-xs text-be-secondary-fg" title="Practice run — a fixed number of questions, no effect on your review schedule">
				practice {Math.min(answered.length + 1, newCap)} / {newCap}
			</span>
		{:else}
			<!-- What this card is: a first-ever bird, or a scheduled review. -->
			{#if currentIsNew}
				<span class="font-be-mono rounded-full bg-be-primary/15 px-2.5 py-1 text-xs text-be-primary" title="A bird you haven't seen before">
					new bird
				</span>
			{:else}
				<span class="font-be-mono rounded-full bg-be-accent/15 px-2.5 py-1 text-xs text-be-accent" title="A bird that's due for review">
					review
				</span>
			{/if}
			<!-- New-bird budget for this session (hidden in review-only runs). -->
			{#if showsNewBudget}
				<span class="font-be-mono rounded-full bg-be-secondary px-2.5 py-1 text-xs text-be-secondary-fg" title="New birds introduced this session, out of the cap">
					{newShown} / {newCap} new
				</span>
			{/if}
			<!-- Cards left in the queue (incl. this one), so the end is visible. In
			     study-ahead mode there's no target, so show a mode chip instead. -->
			{#if extended}
				<span class="font-be-mono rounded-full bg-be-secondary px-2.5 py-1 text-xs text-be-secondary-fg" title="Studying ahead — practising cards before they're due. No effect beyond normal scheduling.">
					study ahead
				</span>
			{:else if toGo !== null}
				<span
					class="font-be-mono rounded-full bg-be-muted px-2.5 py-1 text-xs text-be-muted-fg"
					title="≈{toGo} questions to finish if you get them all right ({remaining?.new} new · {remaining?.learning} learning · {remaining?.due} due). Rises when you miss one."
				>
					{toGo} to go
				</span>
			{/if}
		{/if}
	</div>
</header>

<main class="mx-auto max-w-2xl px-6 pt-6 pb-12">
	{#if completed}
		<!-- Scheduled queue drained: all new + due (+ in-window learning) cleared. -->
		<div class="mx-auto mt-10 max-w-md text-center">
			<div class="mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-full bg-be-primary/15 text-be-primary">
				<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.801 10A10 10 0 1 1 17 3.335"/><path d="m9 11 3 3L22 4"/></svg>
			</div>
			<h2 class="font-be-serif text-2xl font-bold leading-tight">All caught up</h2>
			<p class="mt-2 text-sm text-be-muted-fg">
				You've cleared every new and due card for this session.
			</p>
			{#if answered.length > 0}
				<p class="font-be-mono mt-3 text-xs text-be-muted-fg">
					{scoreSoFar}/{answered.length} correct
				</p>
			{/if}
			<div class="mt-8 flex flex-col gap-3 sm:flex-row sm:justify-center">
				<button
					onclick={toSummary}
					class="rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
				>
					See summary
				</button>
				<button
					onclick={keepTraining}
					class="rounded-lg border border-be-border px-5 py-2.5 text-sm font-semibold transition-colors hover:bg-be-secondary"
				>
					Keep training →
				</button>
			</div>
			<p class="font-be-mono mt-4 text-[11px] text-be-muted-fg">
				Keep training studies cards ahead of their due date.
			</p>
		</div>
	{:else if loading}
		<p class="font-be-mono text-sm text-be-muted-fg">loading question…</p>
	{:else if error}
		<div class="rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">
			{error}
		</div>
	{:else if question}
		<!-- Progress (only when there's a known target; review-only runs have none) -->
		{#if showBar}
			<div class="mb-7 h-0.5 overflow-hidden rounded-full bg-be-muted">
				<div
					class="h-full rounded-full bg-be-primary transition-all duration-500"
					style="width: {progressPct}%"
				></div>
			</div>
		{:else}
			<div class="mb-7"></div>
		{/if}

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
					disabled={!canPlay}
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
			{:else if specFailed && canPlay}
				<p class="font-be-mono mt-2 text-xs text-be-muted-fg">no spectrogram for this one — audio still plays</p>
			{:else if !canPlay}
				<p class="font-be-mono mt-2 text-xs text-be-muted-fg">analyzing audio…</p>
			{:else if !hasPlayed}
				<p class="font-be-mono mt-2 text-xs text-be-muted-fg">press play to hear the bird</p>
			{/if}
		</div>

		<!-- Choices -->
		<div class="mb-3 flex items-center justify-between">
			<p class="font-be-mono text-xs uppercase tracking-widest text-be-muted-fg">Identify the bird</p>
			<p class="font-be-mono hidden text-xs text-be-muted-fg sm:block">
				<kbd class="rounded bg-be-muted px-1 py-0.5">1–{question.choices.length}</kbd> pick ·
				<kbd class="rounded bg-be-muted px-1 py-0.5">space</kbd> play ·
				<kbd class="rounded bg-be-muted px-1 py-0.5">S</kbd> skip
			</p>
		</div>
		<div class="grid grid-cols-2 gap-3">
			{#each question.choices as choice, i (choice)}
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
						{:else}
							<span
								class="font-be-mono mt-px flex h-4 w-4 shrink-0 items-center justify-center rounded bg-be-muted text-[10px] text-be-muted-fg"
								>{i + 1}</span
							>
						{/if}
						<span class="text-sm font-semibold leading-snug">{choice}</span>
					</div>
				</button>
			{/each}
		</div>

		<!-- Skip -->
		{#if !feedback}
			<button
				onclick={skip}
				class="mt-3 w-full rounded-lg border border-be-border/60 py-2.5 text-sm text-be-muted-fg transition-colors hover:border-be-border hover:text-be-fg"
			>
				Skip · I don't know
			</button>
		{/if}

		<!-- Feedback + next -->
		{#if feedback}
			<div class="mt-5 flex items-center justify-between gap-4">
				<p class="text-sm font-semibold {feedback.correct ? 'text-emerald-400' : 'text-red-400'}">
					{#if feedback.correct}
						Correct!
					{:else if feedback.skipped}
						Skipped — that was {feedback.correct_name}.
					{:else}
						That was {feedback.correct_name}.
					{/if}
				</p>
				<button
					onclick={next}
					class="flex shrink-0 items-center gap-1.5 rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
				>
					Next Bird
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
				</button>
			</div>
			<!-- Recording credit (Creative Commons attribution). Revealed with the
			     answer so it doesn't hint the bird before you guess. -->
			{#if question.recordist || question.xc_id}
				<p class="font-be-mono mt-4 text-center text-[11px] leading-relaxed text-be-muted-fg">
					Recording
					{#if xcUrl(question.xc_id)}
						<a href={xcUrl(question.xc_id)} target="_blank" rel="noopener" class="hover:text-be-fg hover:underline">XC{question.xc_id}</a>
					{/if}
					{#if question.recordist}by {question.recordist}{/if}
					{#if question.location} · {question.location}{/if}
					{#if licenseLabel(question.license_url)}
						·
						<a href={licenseHref(question.license_url)} target="_blank" rel="noopener" class="hover:text-be-fg hover:underline">{licenseLabel(question.license_url)}</a>
					{/if}
					· via Xeno-Canto
				</p>
			{/if}

			<!-- Escape hatch: the recording was bad — go straight to it to replace it. -->
			<div class="mt-4 text-center">
				<button
					onclick={manageRecording}
					class="font-be-mono inline-flex items-center gap-1.5 text-xs text-be-muted-fg transition-colors hover:text-be-fg"
				>
					<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>
					Bad recording? Go manage it
				</button>
			</div>
		{/if}
	{/if}
</main>
