<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { UnlistenFn } from '@tauri-apps/api/event';
	import RegionPicker from '$lib/components/RegionPicker.svelte';
	import {
		importBirds,
		importSpecies,
		searchSpecies,
		onImportProgress,
		type ImportProgress,
		type ImportSummary,
		type SpeciesResult
	} from '$lib/api/import';
	import { getSetting, SETTING_EBIRD_KEY, SETTING_XC_KEY } from '$lib/api/settings';
	import { getPacks, type PackDto } from '$lib/api/packs';
	import { runImportJob, importJob } from '$lib/stores/importJob';

	type Mode = 'region' | 'search';
	let mode = $state<Mode>('region');

	// Region form state
	let region = $state('');
	let maxSpecies = $state(20);
	let createPack = $state(true);

	// Shared XC filters (both modes)
	let maxPerSpecies = $state(1);
	let quality = $state(''); // '' = any
	let recType = $state(''); // '' = any
	let family = $state('');

	// Search form state
	let query = $state('');
	let results = $state<SpeciesResult[]>([]);
	let searching = $state(false);
	let selected = $state<SpeciesResult[]>([]);
	let searchTimer: ReturnType<typeof setTimeout> | null = null;

	// Pack targeting (search mode)
	let packs = $state<PackDto[]>([]);
	let packTargets = $state<Set<string>>(new Set());
	let newPackName = $state('');

	// Run state (shared)
	let running = $state(false);
	let lines = $state<string[]>([]);
	let progress = $state<{ current: number; total: number }>({ current: 0, total: 0 });
	let summary = $state<ImportSummary | null>(null);
	let error = $state<string | null>(null);
	let hasKeys = $state(true);

	let unlisten: UnlistenFn | null = null;

	onMount(async () => {
		const [eb, xc] = await Promise.all([
			getSetting(SETTING_EBIRD_KEY),
			getSetting(SETTING_XC_KEY)
		]);
		hasKeys = !!eb.trim() && !!xc.trim();
		try {
			packs = await getPacks();
		} catch {
			// packs are optional targets; ignore load failure
		}
		unlisten = await onImportProgress((p: ImportProgress) => {
			if (p.total) progress = { current: p.current, total: p.total };
			lines = [...lines, p.message].slice(-200);
		});
	});

	onDestroy(() => unlisten?.());

	function onSearchInput() {
		if (searchTimer) clearTimeout(searchTimer);
		const q = query.trim();
		if (q.length < 2) {
			results = [];
			return;
		}
		searchTimer = setTimeout(async () => {
			searching = true;
			try {
				results = await searchSpecies(q);
			} catch (e) {
				error = (e as string) ?? 'Search failed.';
			} finally {
				searching = false;
			}
		}, 250);
	}

	const selectedCodes = $derived(new Set(selected.map((s) => s.ebird_code)));

	function toggle(sp: SpeciesResult) {
		if (sp.in_library) return;
		if (selectedCodes.has(sp.ebird_code)) {
			selected = selected.filter((s) => s.ebird_code !== sp.ebird_code);
		} else {
			selected = [...selected, sp];
		}
	}

	function togglePack(id: string) {
		const next = new Set(packTargets);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		packTargets = next;
	}

	function resetRun() {
		running = true;
		error = null;
		summary = null;
		lines = [];
		progress = { current: 0, total: 0 };
	}

	async function runRegion() {
		if (!region.trim() || running || $importJob.active) return;
		resetRun();
		try {
			summary = await runImportJob(region.trim(), () =>
				importBirds({
					region: region.trim(),
					max_species: Math.max(1, Math.round(maxSpecies) || 20),
					max_per_species: Math.max(1, Math.round(maxPerSpecies) || 1),
					quality: quality || null,
					rec_type: recType || null,
					family: family.trim() || null,
					create_pack: createPack
				})
			);
		} catch (e) {
			error = (e as string) ?? 'Import failed.';
		} finally {
			running = false;
		}
	}

	async function runSearch() {
		if (selected.length === 0 || running || $importJob.active) return;
		resetRun();
		try {
			summary = await runImportJob('birds', () =>
				importSpecies({
					ebirdCodes: selected.map((s) => s.ebird_code),
					quality: quality || null,
					recType: recType || null,
					maxPerSpecies: Math.max(1, Math.round(maxPerSpecies) || 1),
					packIds: [...packTargets],
					newPackName: newPackName.trim() || null
				})
			);
			selected = [];
			newPackName = '';
			packTargets = new Set();
			try {
				packs = await getPacks();
			} catch {
				// ignore
			}
		} catch (e) {
			error = (e as string) ?? 'Import failed.';
		} finally {
			running = false;
		}
	}

	const pctDone = $derived(progress.total ? (progress.current / progress.total) * 100 : 0);
</script>


<main class="mx-auto max-w-2xl px-8 py-10">
	<div class="mb-6">
		<h2 class="font-be-serif mb-2 text-3xl font-bold leading-tight">Import birds</h2>
		<p class="text-sm text-be-muted-fg">
			Pull a whole region from eBird, or search and add individual species. Calls come from
			Xeno-Canto.
		</p>
	</div>

	<div class="mb-6 inline-flex rounded-lg border border-be-border bg-be-card p-1">
		<button
			onclick={() => (mode = 'region')}
			class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors {mode === 'region'
				? 'bg-be-secondary text-be-fg'
				: 'text-be-muted-fg'}">By region</button
		>
		<button
			onclick={() => (mode = 'search')}
			class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors {mode === 'search'
				? 'bg-be-secondary text-be-fg'
				: 'text-be-muted-fg'}">Search species</button
		>
	</div>

	{#if !hasKeys}
		<div class="mb-6 rounded-lg border border-be-accent/40 bg-be-accent/10 px-4 py-3 text-sm text-be-accent">
			Add your eBird + Xeno-Canto API keys in <a href="/settings" class="underline">Settings</a> first.
		</div>
	{/if}

	{#if mode === 'region'}
		<div class="rounded-xl border border-be-border bg-be-card p-6">
			<div class="block">
				<span class="mb-1.5 block text-sm font-medium">Region</span>
				<RegionPicker bind:value={region} disabled={running} />
				<span class="mt-1 block text-xs text-be-muted-fg">
					Pick a country, or narrow to a subregion. Species are ranked by year-round local
					abundance — most common first.
				</span>
			</div>

			<div class="mt-4 grid grid-cols-2 gap-4">
				<label class="block">
					<span class="mb-1.5 block text-sm font-medium">Max species</span>
					<input type="number" min="1" max="200" bind:value={maxSpecies} disabled={running}
						class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
				</label>
				<label class="block">
					<span class="mb-1.5 block text-sm font-medium">Recordings / species</span>
					<input type="number" min="1" max="10" bind:value={maxPerSpecies} disabled={running}
						class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
				</label>
				<label class="block">
					<span class="mb-1.5 block text-sm font-medium">Quality</span>
					<select bind:value={quality} disabled={running}
						class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40">
						<option value="">Any</option>
						<option value="A">A only</option>
						<option value="B">B and above</option>
					</select>
				</label>
				<label class="block">
					<span class="mb-1.5 block text-sm font-medium">Type</span>
					<select bind:value={recType} disabled={running}
						class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40">
						<option value="">Any</option>
						<option value="song">Song</option>
						<option value="call">Call</option>
					</select>
				</label>
			</div>

			<label class="mt-4 block">
				<span class="mb-1.5 block text-sm font-medium">Family filter <span class="text-be-muted-fg">(optional)</span></span>
				<input bind:value={family} placeholder="e.g. Thrushes, Owls" disabled={running}
					class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
			</label>

			<label class="mt-4 flex items-center gap-2.5">
				<input type="checkbox" bind:checked={createPack} disabled={running} class="accent-be-primary" />
				<span class="text-sm">Create a pack from this import</span>
			</label>

			<button
				onclick={runRegion}
				disabled={running || $importJob.active || !region.trim() || !hasKeys}
				class="mt-5 flex w-full items-center justify-center gap-2 rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50"
			>
				{running ? 'Importing…' : 'Import'}
			</button>
		</div>
	{:else}
		<div class="rounded-xl border border-be-border bg-be-card p-6">
			<label class="block">
				<span class="mb-1.5 block text-sm font-medium">Search species</span>
				<input bind:value={query} oninput={onSearchInput} disabled={running}
					placeholder="common or scientific name…"
					class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
			</label>

			{#if selected.length > 0}
				<div class="mt-3 flex flex-wrap gap-1.5">
					{#each selected as s (s.ebird_code)}
						<button onclick={() => toggle(s)}
							class="inline-flex items-center gap-1 rounded-full bg-be-primary/10 px-2.5 py-1 text-xs text-be-fg">
							{s.common_name}
							<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
						</button>
					{/each}
				</div>
			{/if}

			<div class="mt-3 max-h-72 space-y-1 overflow-y-auto rounded-lg border border-be-border p-2">
				{#if searching}
					<p class="font-be-mono px-2 py-1.5 text-sm text-be-muted-fg">searching…</p>
				{:else if query.trim().length < 2}
					<p class="px-2 py-1.5 text-sm text-be-muted-fg">Type at least 2 characters.</p>
				{:else if results.length === 0}
					<p class="px-2 py-1.5 text-sm text-be-muted-fg">No matches.</p>
				{:else}
					{#each results as r (r.ebird_code)}
						<button
							onclick={() => toggle(r)}
							disabled={r.in_library}
							class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors {selectedCodes.has(r.ebird_code)
								? 'bg-be-primary/10'
								: 'hover:bg-be-secondary'} disabled:opacity-40"
						>
							<span class="flex h-4 w-4 shrink-0 items-center justify-center rounded border {selectedCodes.has(r.ebird_code) ? 'border-be-primary bg-be-primary text-be-primary-fg' : 'border-be-border'}">
								{#if selectedCodes.has(r.ebird_code)}<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>{/if}
							</span>
							<span class="min-w-0 flex-1">
								<span class="block truncate text-sm font-medium">{r.common_name}</span>
								<span class="block truncate text-xs italic text-be-muted-fg">{r.scientific_name}{#if r.family} · {r.family}{/if}</span>
							</span>
							{#if r.in_library}
								<span class="font-be-mono shrink-0 text-xs text-be-muted-fg">in library</span>
							{/if}
						</button>
					{/each}
				{/if}
			</div>

			<div class="mt-4 grid grid-cols-3 gap-3">
				<label class="block">
					<span class="mb-1.5 block text-xs font-medium">Recs / species</span>
					<input type="number" min="1" max="10" bind:value={maxPerSpecies} disabled={running}
						class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
				</label>
				<label class="block">
					<span class="mb-1.5 block text-xs font-medium">Quality</span>
					<select bind:value={quality} disabled={running}
						class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40">
						<option value="">Any</option>
						<option value="A">A only</option>
						<option value="B">B and above</option>
					</select>
				</label>
				<label class="block">
					<span class="mb-1.5 block text-xs font-medium">Type</span>
					<select bind:value={recType} disabled={running}
						class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40">
						<option value="">Any</option>
						<option value="song">Song</option>
						<option value="call">Call</option>
					</select>
				</label>
			</div>

			<div class="mt-5 border-t border-be-border pt-4">
				<span class="mb-1.5 block text-sm font-medium">Add to packs <span class="text-be-muted-fg">(optional)</span></span>
				{#if packs.length > 0}
					<div class="flex flex-wrap gap-1.5">
						{#each packs as p (p.id)}
							<button onclick={() => togglePack(p.id)} disabled={running}
								class="inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs transition-colors {packTargets.has(p.id)
									? 'border-be-primary bg-be-primary/10 text-be-fg'
									: 'border-be-border text-be-muted-fg hover:bg-be-secondary'}">
								{#if packTargets.has(p.id)}<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>{/if}
								{p.name}
							</button>
						{/each}
					</div>
				{:else}
					<p class="text-xs text-be-muted-fg">No packs yet — make one below.</p>
				{/if}
				<input bind:value={newPackName} disabled={running} placeholder="…or new pack name"
					class="mt-3 w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
			</div>

			<button
				onclick={runSearch}
				disabled={running || $importJob.active || selected.length === 0 || !hasKeys}
				class="mt-4 flex w-full items-center justify-center gap-2 rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50"
			>
				{running ? 'Adding…' : selected.length ? `Add ${selected.length} ${selected.length === 1 ? 'bird' : 'birds'}` : 'Add birds'}
			</button>
		</div>
	{/if}

	{#if error}
		<div class="mt-6 rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">
			{error}
		</div>
	{/if}

	{#if running || lines.length > 0}
		<div class="mt-6 rounded-xl border border-be-border bg-be-card p-5">
			{#if progress.total > 0}
				<div class="mb-3 h-1.5 w-full overflow-hidden rounded-full bg-be-muted">
					<div class="h-full rounded-full bg-be-primary transition-all" style="width: {pctDone}%"></div>
				</div>
			{/if}
			<div class="font-be-mono max-h-52 space-y-0.5 overflow-y-auto text-xs text-be-muted-fg">
				{#each lines as l, i (i)}
					<div>{l}</div>
				{/each}
			</div>
		</div>
	{/if}

	{#if summary}
		<div class="mt-6 rounded-xl border border-be-primary/30 bg-be-primary/[0.06] p-6">
			<h3 class="font-be-serif mb-2 text-lg font-semibold">Import complete</h3>
			<p class="text-sm text-be-fg">
				{summary.species_imported} species · {summary.recordings_added} recordings
				{#if summary.species_skipped > 0}<span class="text-be-muted-fg"> · {summary.species_skipped} skipped (no audio)</span>{/if}
			</p>
			<div class="mt-4 flex gap-3">
				<a href="/library" class="rounded-lg border border-be-border px-4 py-2 text-sm transition-colors hover:bg-be-secondary">View library</a>
				<a href="/" class="rounded-lg bg-be-primary px-4 py-2 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90">Start training</a>
			</div>
		</div>
	{/if}
</main>
