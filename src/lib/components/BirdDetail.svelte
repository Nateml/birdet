<script lang="ts">
	import { goto } from '$app/navigation';
	import { tick } from 'svelte';
	import { getBirds, type BirdListItem } from '$lib/api/packs';
	import { getRecordingBlobUrl } from '$lib/api/audio';
	import {
		getBirdRecordings,
		deleteRecording,
		deleteBird,
		searchBirdRecordings,
		addBirdRecordings,
		addRecordingByNumber,
		type RecordingInfo,
		type RecordingCandidate
	} from '$lib/api/recordings';
	import { runImportJob, importJob } from '$lib/stores/importJob';
	import { licenseLabel, licenseHref } from '$lib/license';

	let {
		birdId,
		standalone = false,
		onChanged,
		onDeleted,
		highlightRecordingId,
		backHref
	}: {
		birdId: number;
		standalone?: boolean;
		onChanged?: () => void;
		onDeleted?: () => void;
		highlightRecordingId?: number;
		backHref?: string;
	} = $props();

	let bird = $state<BirdListItem | null>(null);
	let recordings = $state<RecordingInfo[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let busy = $state(false);

	// playback
	let audioEl: HTMLAudioElement;
	let playingId = $state<number | null>(null);
	let curUrl: string | null = null;
	let confirmDelete = $state<number | null>(null);
	let expandedId = $state<number | null>(null);

	// add panel
	let adding = $state(false);
	let quality = $state('');
	let recType = $state('');
	let searching = $state(false);
	let searchError = $state<string | null>(null);
	let candidates = $state<RecordingCandidate[]>([]);
	let nameFallback = $state(false);
	let xcName = $state<string | null>(null);
	let searched = $state(false);
	let toAdd = $state<Set<string>>(new Set());
	let confirmingDeleteBird = $state(false);

	// add by catalogue number
	let catNr = $state('');
	let catBusy = $state(false);
	let catMsg = $state<{ ok: boolean; text: string } | null>(null);

	// Recording durations aren't stored server-side, so read each clip's audio
	// metadata once (headers only) and cache it to show a length chip.
	let durations = $state<Record<number, number>>({});
	const durationCache = new Map<number, number>();

	function fmtDur(sec: number): string {
		const s = Math.round(sec);
		return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
	}

	function audioDuration(url: string): Promise<number> {
		return new Promise((resolve, reject) => {
			const a = new Audio();
			a.preload = 'metadata';
			a.onloadedmetadata = () => resolve(a.duration);
			a.onerror = () => reject(new Error('metadata load failed'));
			a.src = url;
		});
	}

	async function refreshDurations() {
		await Promise.all(
			recordings.map(async (r) => {
				if (durationCache.has(r.id)) {
					durations = { ...durations, [r.id]: durationCache.get(r.id)! };
					return;
				}
				let url: string | null = null;
				try {
					url = await getRecordingBlobUrl(r.id);
					const secs = await audioDuration(url);
					if (Number.isFinite(secs) && secs > 0) {
						durationCache.set(r.id, secs);
						durations = { ...durations, [r.id]: secs };
					}
				} catch {
					/* skip — the chip just won't show for this one */
				} finally {
					if (url) URL.revokeObjectURL(url);
				}
			})
		);
	}

	let loadedId: number | null = null;
	$effect(() => {
		if (birdId !== loadedId) {
			loadedId = birdId;
			void reload();
		}
	});

	// When deep-linked with a recording id (e.g. from the quiz), scroll it into
	// view and flash it once the list has loaded, so the user lands on the exact
	// recording they wanted to replace.
	let flashId = $state<number | null>(null);
	let flashedFor: number | null = null; // guard: auto-scroll each target once
	$effect(() => {
		const target = highlightRecordingId;
		if (!target || loading || flashedFor === target) return;
		if (!recordings.some((r) => r.id === target)) return;
		flashedFor = target;
		flashId = target;
		expandedId = target; // open the details for the recording we jumped to
		void tick().then(() =>
			document
				.getElementById(`rec-${target}`)
				?.scrollIntoView({ block: 'center', behavior: 'smooth' })
		);
		setTimeout(() => {
			if (flashId === target) flashId = null;
		}, 2600);
	});

	function stopPlayback() {
		try {
			audioEl?.pause();
		} catch {
			/* not mounted yet */
		}
		if (curUrl) URL.revokeObjectURL(curUrl);
		curUrl = null;
		playingId = null;
	}

	async function reload() {
		loading = true;
		error = null;
		stopPlayback();
		adding = false;
		searched = false;
		candidates = [];
		toAdd = new Set();
		confirmDelete = null;
		confirmingDeleteBird = false;
		try {
			const birds = await getBirds();
			bird = birds.find((b) => b.id === birdId) ?? null;
			if (!bird) throw new Error('Bird not found.');
			recordings = await getBirdRecordings(birdId);
			void refreshDurations();
		} catch (e) {
			error = (e as Error)?.message ?? (e as string) ?? 'Failed to load bird.';
		} finally {
			loading = false;
		}
	}

	async function play(id: number) {
		try {
			if (playingId === id) {
				audioEl.pause();
				playingId = null;
				return;
			}
			if (curUrl) URL.revokeObjectURL(curUrl);
			curUrl = await getRecordingBlobUrl(id);
			audioEl.src = curUrl;
			await audioEl.play();
			playingId = id;
		} catch (e) {
			error = `Playback failed: ${(e as Error)?.message ?? e}`;
		}
	}

	async function remove(id: number) {
		if (busy) return;
		busy = true;
		error = null;
		try {
			await deleteRecording(id);
			if (playingId === id) {
				audioEl.pause();
				playingId = null;
			}
			confirmDelete = null;
			recordings = await getBirdRecordings(birdId);
			onChanged?.();
		} catch (e) {
			error = (e as string) ?? 'Delete failed.';
		} finally {
			busy = false;
		}
	}

	async function runSearch() {
		if (searching) return;
		searching = true;
		searchError = null;
		toAdd = new Set();
		try {
			const res = await searchBirdRecordings(birdId, quality || undefined, recType || undefined);
			candidates = res.candidates;
			nameFallback = res.name_fallback;
			xcName = res.xc_name;
			searched = true;
		} catch (e) {
			searchError = (e as string) ?? 'Search failed.';
			candidates = [];
			nameFallback = false;
			xcName = null;
		} finally {
			searching = false;
		}
	}

	async function addByNumber() {
		const input = catNr.trim();
		if (!input || catBusy || $importJob.active) return;
		catBusy = true;
		catMsg = null;
		try {
			const res = await runImportJob('recordings', () => addRecordingByNumber(birdId, input));
			if (!res.added) {
				catMsg = { ok: false, text: `XC${res.xc_id} is already in your library.` };
				return;
			}
			recordings = await getBirdRecordings(birdId);
			void refreshDurations();
			// Warn if the recording's species doesn't look like this bird — a wrong
			// catalogue number adds the wrong bird's sound.
			const mismatch =
				res.en && bird?.common_name && res.en.toLowerCase() !== bird.common_name.toLowerCase();
			catMsg = {
				ok: !mismatch,
				text: mismatch
					? `Added XC${res.xc_id}, but Xeno-Canto files it as “${res.en}” — check it's the right species.`
					: `Added XC${res.xc_id}${res.recordist ? ` by ${res.recordist}` : ''}.`
			};
			catNr = '';
			onChanged?.();
		} catch (e) {
			catMsg = { ok: false, text: (e as string) ?? 'Add failed.' };
		} finally {
			catBusy = false;
		}
	}

	function toggle(xcId: string) {
		const next = new Set(toAdd);
		if (next.has(xcId)) next.delete(xcId);
		else next.add(xcId);
		toAdd = next;
	}

	async function saveAdditions() {
		if (busy || toAdd.size === 0 || $importJob.active) return;
		busy = true;
		searchError = null;
		try {
			// Route the download through the global import guard so it shows in the
			// indicator and can't overlap another import.
			await runImportJob('recordings', () => addBirdRecordings(birdId, [...toAdd]));
			recordings = await getBirdRecordings(birdId);
			void refreshDurations();
			candidates = candidates.filter((c) => !toAdd.has(c.xc_id));
			toAdd = new Set();
			onChanged?.();
		} catch (e) {
			searchError = (e as string) ?? 'Add failed.';
		} finally {
			busy = false;
		}
	}

	async function doDeleteBird() {
		if (busy) return;
		busy = true;
		error = null;
		try {
			await deleteBird(birdId);
			if (standalone) goto('/library');
			else onDeleted?.();
		} catch (e) {
			error = (e as string) ?? 'Delete failed.';
			busy = false;
		}
	}
</script>

<audio bind:this={audioEl} onended={() => (playingId = null)} class="hidden"></audio>

<div class={standalone ? 'mx-auto max-w-2xl px-8 py-10' : 'px-8 py-8'}>
	{#if backHref}
		<a href={backHref} class="mb-4 inline-flex items-center gap-1 text-sm text-be-primary transition-colors hover:underline">
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
			Back to training
		</a>
	{:else if standalone}
		<a href="/library" class="mb-4 inline-flex items-center gap-1 text-sm text-be-muted-fg transition-colors hover:text-be-fg">
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
			Library
		</a>
	{/if}

	{#if loading}
		<p class="font-be-mono text-sm text-be-muted-fg">loading…</p>
	{:else if error && !bird}
		<div class="rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">{error}</div>
	{:else if bird}
		<div class="mb-6">
			<h2 class="font-be-serif text-3xl font-bold leading-tight">{bird.common_name}</h2>
			<p class="mt-1 text-sm italic text-be-muted-fg">{bird.scientific_name}{#if bird.family} · {bird.family}{/if}</p>
		</div>

		{#if error}
			<div class="mb-4 rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">{error}</div>
		{/if}

		<div class="mb-3 flex items-center justify-between">
			<span class="text-sm font-medium">Recordings <span class="text-be-muted-fg">({recordings.length})</span></span>
			<button onclick={() => (adding = !adding)}
				class="rounded-lg border border-be-border px-3 py-1.5 text-sm transition-colors hover:bg-be-secondary">
				{adding ? 'Done' : '+ Add recordings'}
			</button>
		</div>

		{#if adding}
			<div class="mb-5 rounded-xl border border-be-border bg-be-card p-4">
				<!-- Add a specific recording straight by its Xeno-Canto catalogue number. -->
				<p class="mb-2 text-xs font-medium text-be-muted-fg">Add by catalogue number</p>
				<form onsubmit={(e) => { e.preventDefault(); addByNumber(); }} class="flex flex-wrap items-center gap-2">
					<input
						bind:value={catNr}
						placeholder="e.g. XC123456"
						class="min-w-0 flex-1 rounded-lg border border-be-border bg-be-bg px-3 py-1.5 text-sm text-be-fg outline-none focus:border-be-primary/40"
					/>
					<button type="submit" disabled={catBusy || !catNr.trim() || $importJob.active}
						class="rounded-lg bg-be-primary px-4 py-1.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50">
						{catBusy ? 'Adding…' : 'Add'}
					</button>
				</form>
				{#if catMsg}
					<p class="mt-2 text-xs {catMsg.ok ? 'text-be-accent' : 'text-be-destructive'}">{catMsg.text}</p>
				{/if}

				<div class="my-4 border-t border-be-border"></div>

				<p class="mb-3 text-xs text-be-muted-fg">Or search Xeno-Canto for more recordings of this species.</p>
				<div class="flex flex-wrap items-end gap-3">
					<label class="block">
						<span class="mb-1 block text-xs font-medium text-be-muted-fg">Min quality</span>
						<select bind:value={quality}
							class="rounded-lg border border-be-border bg-be-bg px-2.5 py-1.5 text-sm text-be-fg outline-none focus:border-be-primary/40">
							<option value="">any</option>
							<option value="A">A</option>
							<option value="B">B</option>
							<option value="C">C</option>
						</select>
					</label>
					<label class="block">
						<span class="mb-1 block text-xs font-medium text-be-muted-fg">Type</span>
						<select bind:value={recType}
							class="rounded-lg border border-be-border bg-be-bg px-2.5 py-1.5 text-sm text-be-fg outline-none focus:border-be-primary/40">
							<option value="">any</option>
							<option value="song">song</option>
							<option value="call">call</option>
						</select>
					</label>
					<button onclick={runSearch} disabled={searching}
						class="rounded-lg bg-be-primary px-4 py-1.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50">
						{searching ? 'Searching…' : 'Search'}
					</button>
				</div>

				{#if searchError}
					<p class="mt-3 text-sm text-be-destructive">{searchError}</p>
				{/if}

				{#if searched && !searching}
					{#if nameFallback}
						<p class="mt-3 rounded-lg border border-be-accent/40 bg-be-accent/10 px-3 py-2 text-xs text-be-accent">
							⚠ No Xeno-Canto recordings under this bird's scientific name — matched by English name
							instead{#if xcName}, filed on Xeno-Canto as <em>{xcName}</em>{/if}. Double-check they're the
							right species before adding.
						</p>
					{/if}
					<div class="mt-4 max-h-72 space-y-1 overflow-y-auto">
						{#if candidates.length === 0}
							<p class="px-2 py-1.5 text-sm text-be-muted-fg">No new recordings found.</p>
						{:else}
							{#each candidates as c (c.xc_id)}
								<button onclick={() => toggle(c.xc_id)}
									class="flex w-full items-start gap-3 rounded-md px-3 py-2 text-left transition-colors {toAdd.has(c.xc_id) ? 'bg-be-primary/10' : 'hover:bg-be-secondary'}">
									<span class="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded border {toAdd.has(c.xc_id) ? 'border-be-primary bg-be-primary text-be-primary-fg' : 'border-be-border'}">
										{#if toAdd.has(c.xc_id)}<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>{/if}
									</span>
									<span class="min-w-0 flex-1">
										<span class="flex flex-wrap items-center gap-x-2 text-sm">
											<span class="font-medium">XC{c.xc_id}</span>
											{#if c.quality}<span class="font-be-mono text-xs text-be-muted-fg">q:{c.quality}</span>{/if}
											{#if c.rec_type}<span class="text-xs text-be-muted-fg">{c.rec_type}</span>{/if}
											{#if c.length}<span class="font-be-mono text-xs text-be-muted-fg">{c.length}</span>{/if}
										</span>
										<span class="block truncate text-xs text-be-muted-fg">{c.recordist}{#if c.location} · {c.location}{/if}</span>
									</span>
								</button>
							{/each}
						{/if}
					</div>
					{#if candidates.length > 0}
						<button onclick={saveAdditions} disabled={busy || toAdd.size === 0 || $importJob.active}
							class="mt-3 w-full rounded-lg bg-be-primary px-4 py-2 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50">
							{$importJob.active
								? 'Import in progress…'
								: toAdd.size
									? `Add ${toAdd.size} recording${toAdd.size > 1 ? 's' : ''}`
									: 'Select recordings to add'}
						</button>
					{/if}
				{/if}
			</div>
		{/if}

		{#if recordings.length === 0}
			<p class="rounded-xl border border-be-border bg-be-card px-5 py-6 text-center text-sm text-be-muted-fg">No recordings.</p>
		{:else}
			<div class="divide-y divide-be-border overflow-hidden rounded-xl border border-be-border bg-be-card">
				{#each recordings as r (r.id)}
					{@const expanded = expandedId === r.id}
					<div
						id="rec-{r.id}"
						class="transition-colors duration-500 {flashId === r.id ? 'bg-be-primary/10' : ''}"
					>
						<div class="flex items-center gap-3 px-4 py-3">
							<button onclick={() => play(r.id)} title={playingId === r.id ? 'Pause' : 'Play'}
								class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-be-border text-be-fg transition-colors hover:border-be-primary/40 hover:text-be-primary">
								{#if playingId === r.id}
									<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/></svg>
								{:else}
									<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
								{/if}
							</button>
							<!-- Click the summary to expand a full, untruncated details card. -->
							<button
								onclick={() => (expandedId = expanded ? null : r.id)}
								aria-expanded={expanded}
								title={expanded ? 'Hide details' : 'Show details'}
								class="flex min-w-0 flex-1 items-center gap-2 text-left"
							>
								<span class="min-w-0 flex-1">
									<span class="flex flex-wrap items-center gap-2 text-sm">
										<span class="font-medium">{r.xc_id ? `XC${r.xc_id}` : (r.filename ?? `#${r.id}`)}</span>
										{#if durations[r.id]}
											<span class="font-be-mono rounded-full border border-be-border px-2 py-0.5 text-[11px] leading-none text-be-muted-fg">{fmtDur(durations[r.id])}</span>
										{/if}
										{#if r.rec_type}
											<span class="rounded-full bg-be-secondary px-2 py-0.5 text-[11px] font-medium capitalize leading-none text-be-muted-fg">{r.rec_type}</span>
										{/if}
										{#if r.quality}
											<span class="font-be-mono rounded-full border border-be-border px-2 py-0.5 text-[11px] leading-none text-be-muted-fg">q:{r.quality}</span>
										{/if}
										{#if licenseLabel(r.license_url)}
											<span class="font-be-mono rounded-full border border-be-border px-2 py-0.5 text-[11px] leading-none text-be-muted-fg">{licenseLabel(r.license_url)}</span>
										{/if}
									</span>
									{#if r.recordist || r.location}
										<span class="mt-0.5 block truncate text-xs text-be-muted-fg">© {r.recordist ?? 'Unknown'}{#if r.location} · {r.location}{/if} · via Xeno-Canto</span>
									{/if}
								</span>
								<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
									class="shrink-0 text-be-muted-fg transition-transform duration-200 {expanded ? 'rotate-180' : ''}"><path d="m6 9 6 6 6-6"/></svg>
							</button>
							{#if confirmDelete === r.id}
								<button onclick={() => remove(r.id)} disabled={busy}
									class="shrink-0 rounded-md bg-be-destructive px-2.5 py-1 text-xs font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50">Delete</button>
								<button onclick={() => (confirmDelete = null)}
									class="shrink-0 rounded-md border border-be-border px-2.5 py-1 text-xs transition-colors hover:bg-be-secondary">Cancel</button>
							{:else}
								<button onclick={() => (confirmDelete = r.id)} disabled={busy || recordings.length <= 1}
									title={recordings.length <= 1 ? "Can't delete the only recording" : 'Delete recording'}
									class="shrink-0 rounded-md p-1.5 text-be-muted-fg transition-colors hover:bg-be-destructive/10 hover:text-be-destructive disabled:cursor-not-allowed disabled:opacity-30">
									<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/></svg>
								</button>
							{/if}
						</div>

						{#if expanded}
							<!-- Full recording details — every field, none truncated. -->
							<dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 border-t border-be-border bg-be-bg/40 px-4 py-3 text-sm">
								<dt class="text-xs text-be-muted-fg">Recordist</dt>
								<dd>{r.recordist ?? 'Unknown'}</dd>
								{#if r.location}
									<dt class="text-xs text-be-muted-fg">Location</dt>
									<dd>{r.location}</dd>
								{/if}
								{#if durations[r.id]}
									<dt class="text-xs text-be-muted-fg">Duration</dt>
									<dd class="font-be-mono">{fmtDur(durations[r.id])}</dd>
								{/if}
								{#if r.rec_type}
									<dt class="text-xs text-be-muted-fg">Type</dt>
									<dd class="capitalize">{r.rec_type}</dd>
								{/if}
								{#if r.quality}
									<dt class="text-xs text-be-muted-fg">Quality</dt>
									<dd class="font-be-mono">{r.quality}</dd>
								{/if}
								{#if r.background.length > 0}
									<dt class="text-xs text-be-muted-fg">Background</dt>
									<dd class="text-be-muted-fg">{r.background.join(', ')}</dd>
								{/if}
								{#if licenseLabel(r.license_url)}
									<dt class="text-xs text-be-muted-fg">Licence</dt>
									<dd>
										<a href={licenseHref(r.license_url)} target="_blank" rel="noopener" class="text-be-primary hover:underline">{licenseLabel(r.license_url)}</a>
									</dd>
								{/if}
								{#if r.xc_id}
									<dt class="text-xs text-be-muted-fg">Source</dt>
									<dd>
										<a href="https://xeno-canto.org/{r.xc_id}" target="_blank" rel="noopener" class="text-be-primary hover:underline">Xeno-Canto XC{r.xc_id}</a>
									</dd>
								{/if}
								{#if r.filename}
									<dt class="text-xs text-be-muted-fg">File</dt>
									<dd class="font-be-mono break-all text-xs text-be-muted-fg">{r.filename}</dd>
								{/if}
							</dl>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

		<div class="mt-8 border-t border-be-border pt-6">
			{#if confirmingDeleteBird}
				<div class="flex flex-wrap items-center gap-2 text-sm">
					<span class="text-be-muted-fg">Delete this bird and all its recordings?</span>
					<button onclick={doDeleteBird} disabled={busy}
						class="rounded-lg bg-be-destructive px-3 py-1.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50">Delete bird</button>
					<button onclick={() => (confirmingDeleteBird = false)}
						class="rounded-lg border border-be-border px-3 py-1.5 text-sm transition-colors hover:bg-be-secondary">Cancel</button>
				</div>
			{:else}
				<button onclick={() => (confirmingDeleteBird = true)} disabled={busy}
					class="text-sm text-be-muted-fg transition-colors hover:text-be-destructive">Delete bird</button>
			{/if}
		</div>
	{/if}
</div>
