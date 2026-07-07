<script lang="ts">
	import { goto } from '$app/navigation';
	import { getBirds, type BirdListItem } from '$lib/api/packs';
	import { getRecordingBlobUrl } from '$lib/api/audio';
	import {
		getBirdRecordings,
		deleteRecording,
		deleteBird,
		searchBirdRecordings,
		addBirdRecordings,
		type RecordingInfo,
		type RecordingCandidate
	} from '$lib/api/recordings';
	import { runImportJob, importJob } from '$lib/stores/importJob';

	let {
		birdId,
		standalone = false,
		onChanged,
		onDeleted
	}: {
		birdId: number;
		standalone?: boolean;
		onChanged?: () => void;
		onDeleted?: () => void;
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

	// add panel
	let adding = $state(false);
	let quality = $state('');
	let recType = $state('');
	let searching = $state(false);
	let searchError = $state<string | null>(null);
	let candidates = $state<RecordingCandidate[]>([]);
	let searched = $state(false);
	let toAdd = $state<Set<string>>(new Set());
	let confirmingDeleteBird = $state(false);

	let loadedId: number | null = null;
	$effect(() => {
		if (birdId !== loadedId) {
			loadedId = birdId;
			void reload();
		}
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
			candidates = await searchBirdRecordings(birdId, quality || undefined, recType || undefined);
			searched = true;
		} catch (e) {
			searchError = (e as string) ?? 'Search failed.';
			candidates = [];
		} finally {
			searching = false;
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
	{#if standalone}
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
				<p class="mb-3 text-xs text-be-muted-fg">Search Xeno-Canto for more recordings of this species.</p>
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
					<div class="flex items-center gap-3 px-4 py-3">
						<button onclick={() => play(r.id)} title={playingId === r.id ? 'Pause' : 'Play'}
							class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-be-border text-be-fg transition-colors hover:border-be-primary/40 hover:text-be-primary">
							{#if playingId === r.id}
								<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/></svg>
							{:else}
								<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
							{/if}
						</button>
						<div class="min-w-0 flex-1">
							<p class="flex flex-wrap items-center gap-2 text-sm">
								{#if r.xc_id}
									<a href="https://xeno-canto.org/{r.xc_id}" target="_blank" rel="noopener" class="font-medium hover:text-be-primary hover:underline">XC{r.xc_id}</a>
								{:else}
									<span class="font-medium">{r.filename ?? `#${r.id}`}</span>
								{/if}
								{#if r.rec_type}
									<span class="rounded-full bg-be-secondary px-2 py-0.5 text-[11px] font-medium capitalize leading-none text-be-muted-fg">{r.rec_type}</span>
								{/if}
								{#if r.quality}
									<span class="font-be-mono rounded-full border border-be-border px-2 py-0.5 text-[11px] leading-none text-be-muted-fg">q:{r.quality}</span>
								{/if}
							</p>
							{#if r.recordist || r.location}
								<p class="truncate text-xs text-be-muted-fg">{r.recordist ?? ''}{#if r.recordist && r.location} · {/if}{r.location ?? ''}</p>
							{/if}
						</div>
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
