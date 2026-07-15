<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import RegionPicker from '$lib/components/RegionPicker.svelte';
	import IconPicker from '$lib/components/IconPicker.svelte';
	import {
		getBirds,
		createPack,
		createPackFromFilter,
		type BirdListItem
	} from '$lib/api/packs';

	type Tab = 'manual' | 'filter';
	let tab = $state<Tab>('manual');

	let birds = $state<BirdListItem[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let creating = $state(false);

	// Manual
	let name = $state('');
	let query = $state('');
	let selected = $state<Set<number>>(new Set());
	let icon = $state<string | null>(null);

	// Filter
	let filterName = $state('');
	let filterRegion = $state('');
	let filterFamily = $state('');
	let filterIcon = $state<string | null>(null);

	onMount(async () => {
		try {
			birds = await getBirds();
		} catch (e) {
			error = (e as string) ?? 'Failed to load birds.';
		} finally {
			loading = false;
		}
	});

	const filtered = $derived(
		query.trim()
			? birds.filter((b) =>
					`${b.common_name} ${b.scientific_name}`.toLowerCase().includes(query.trim().toLowerCase())
				)
			: birds
	);

	// Distinct families present, for the datalist hint.
	const families = $derived([...new Set(birds.map((b) => b.family).filter(Boolean))] as string[]);

	function toggle(id: number) {
		const next = new Set(selected);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		selected = next;
	}

	async function saveManual() {
		if (!name.trim() || selected.size === 0 || creating) return;
		creating = true;
		error = null;
		try {
			await createPack(name.trim(), [...selected], icon);
			goto('/library');
		} catch (e) {
			error = (e as string) ?? 'Failed to create pack.';
			creating = false;
		}
	}

	async function saveFilter() {
		if (creating) return;
		if (!filterRegion.trim() && !filterFamily.trim()) {
			error = 'Set a region or family to filter by.';
			return;
		}
		creating = true;
		error = null;
		try {
			await createPackFromFilter(
				filterName.trim() || null,
				filterRegion.trim() || null,
				filterFamily.trim() || null,
				filterIcon
			);
			goto('/library');
		} catch (e) {
			error = (e as string) ?? 'Failed to create pack.';
			creating = false;
		}
	}
</script>


<main class="mx-auto max-w-2xl px-8 py-10">
	<div class="mb-6">
		<h2 class="font-be-serif mb-2 text-3xl font-bold leading-tight">New pack</h2>
		<p class="text-sm text-be-muted-fg">Group birds into a curated set. Scheduling stays global.</p>
	</div>

	<div class="mb-6 inline-flex rounded-lg border border-be-border bg-be-card p-1">
		<button
			onclick={() => (tab = 'manual')}
			class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors {tab === 'manual'
				? 'bg-be-secondary text-be-fg'
				: 'text-be-muted-fg'}">Pick birds</button
		>
		<button
			onclick={() => (tab = 'filter')}
			class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors {tab === 'filter'
				? 'bg-be-secondary text-be-fg'
				: 'text-be-muted-fg'}">From filter</button
		>
	</div>

	{#if error}
		<div class="mb-6 rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">
			{error}
		</div>
	{/if}

	{#if tab === 'manual'}
		<div class="rounded-xl border border-be-border bg-be-card p-6">
			<label class="block">
				<span class="mb-1.5 block text-sm font-medium">Pack name</span>
				<input bind:value={name} placeholder="Garden regulars"
					class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
			</label>

			<div class="mt-4">
				<span class="mb-1.5 block text-sm font-medium">Icon <span class="text-be-muted-fg">(optional)</span></span>
				<IconPicker bind:value={icon} disabled={creating} />
			</div>

			<div class="mt-4 flex items-center justify-between">
				<span class="text-sm font-medium">Birds <span class="text-be-muted-fg">({selected.size} selected)</span></span>
				<input bind:value={query} placeholder="search…"
					class="w-40 rounded-lg border border-be-border bg-be-bg px-3 py-1.5 text-sm text-be-fg outline-none focus:border-be-primary/40" />
			</div>

			{#if loading}
				<p class="font-be-mono mt-3 text-sm text-be-muted-fg">loading birds…</p>
			{:else if birds.length === 0}
				<p class="mt-3 text-sm text-be-muted-fg">No birds yet — import some first.</p>
			{:else}
				<div class="mt-3 max-h-72 space-y-1 overflow-y-auto rounded-lg border border-be-border p-2">
					{#each filtered as b (b.id)}
						<button
							onclick={() => toggle(b.id)}
							class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors {selected.has(b.id)
								? 'bg-be-primary/10'
								: 'hover:bg-be-secondary'}"
						>
							<span class="flex h-4 w-4 shrink-0 items-center justify-center rounded border {selected.has(b.id) ? 'border-be-primary bg-be-primary text-be-primary-fg' : 'border-be-border'}">
								{#if selected.has(b.id)}<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>{/if}
							</span>
							<span class="min-w-0 flex-1">
								<span class="block truncate text-sm font-medium">{b.common_name}</span>
								<span class="block truncate text-xs italic text-be-muted-fg">{b.scientific_name}</span>
							</span>
							<span class="font-be-mono shrink-0 text-xs text-be-muted-fg">{b.recording_count}♪</span>
						</button>
					{/each}
				</div>
			{/if}

			<button
				onclick={saveManual}
				disabled={creating || !name.trim() || selected.size === 0}
				class="mt-5 w-full rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50"
			>
				{creating ? 'Creating…' : `Create pack · ${selected.size} birds`}
			</button>
		</div>
	{:else}
		<div class="rounded-xl border border-be-border bg-be-card p-6">
			<label class="block">
				<span class="mb-1.5 block text-sm font-medium">Pack name <span class="text-be-muted-fg">(optional)</span></span>
				<input bind:value={filterName} placeholder="auto-named from the filter"
					class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
			</label>
			<div class="mt-4">
				<span class="mb-1.5 block text-sm font-medium">Icon <span class="text-be-muted-fg">(optional)</span></span>
				<IconPicker bind:value={filterIcon} disabled={creating} />
			</div>
			<div class="mt-4 block">
				<span class="mb-1.5 block text-sm font-medium">Region</span>
				<RegionPicker bind:value={filterRegion} />
			</div>
			<label class="mt-4 block">
				<span class="mb-1.5 block text-sm font-medium">Family</span>
				<input bind:value={filterFamily} placeholder="e.g. Thrushes" list="families"
					class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
				<datalist id="families">{#each families as f}<option value={f}></option>{/each}</datalist>
			</label>
			<p class="mt-3 text-xs text-be-muted-fg">
				Pack = library birds whose eBird range covers the region (via eBird), optionally
				narrowed by family. Region packs need your eBird key.
			</p>
			<button
				onclick={saveFilter}
				disabled={creating}
				class="mt-5 w-full rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50"
			>
				{creating ? 'Creating…' : 'Create pack from filter'}
			</button>
		</div>
	{/if}
</main>
