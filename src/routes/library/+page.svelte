<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import { getPacks, getBirds, getBirdPacks, importPack, type PackDto, type BirdListItem } from '$lib/api/packs';
	import { startSession } from '$lib/stores/session';
	import { setup } from '$lib/stores/setup';
	import { runImportJob, importJob } from '$lib/stores/importJob';

	type Tab = 'packs' | 'birds';
	let tab = $state<Tab>('packs');

	let packs = $state<PackDto[]>([]);
	let birds = $state<BirdListItem[]>([]);
	let birdPacks = $state<Map<number, { id: string; name: string }[]>>(new Map());
	let error = $state<string | null>(null);
	let loading = $state(true);
	let query = $state('');

	const EMOJI = ['🏡', '🌿', '🌊', '🦅', '🐦', '🦉', '🕊️', '🐤'];

	// Pack import (from an exported .birdet-pack.json file).
	let fileInput = $state<HTMLInputElement | null>(null);
	let importing = $state(false);
	let importStatus = $state<string | null>(null);

	async function onPackFile(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = ''; // allow re-selecting the same file later
		if (!file || importing || $importJob.active) return;

		importing = true;
		error = null;
		importStatus = 'Reading file…';
		try {
			const text = await file.text();
			// Live progress is shown by the global indicator; runImportJob also
			// blocks a second concurrent import.
			const r = await runImportJob('pack', () => importPack(text));
			const parts = [`Linked ${r.linked_existing}`];
			if (r.downloaded_new) parts.push(`downloaded ${r.downloaded_new}`);
			if (r.skipped) parts.push(`skipped ${r.skipped}`);
			importStatus = `Imported “${r.name}” — ${parts.join(', ')}.`;
			goto(`/packs/${r.pack_id}`);
		} catch (err) {
			error = (err as string) ?? 'Pack import failed.';
			importStatus = null;
		} finally {
			importing = false;
		}
	}

	onMount(async () => {
		try {
			const [p, b, tags] = await Promise.all([getPacks(), getBirds(), getBirdPacks()]);
			packs = p;
			birds = b;
			const map = new Map<number, { id: string; name: string }[]>();
			for (const t of tags) {
				const list = map.get(t.bird_id) ?? [];
				list.push({ id: t.pack_id, name: t.pack_name });
				map.set(t.bird_id, list);
			}
			birdPacks = map;
		} catch (e) {
			error = (e as Error)?.message ?? (e as string) ?? 'Failed to load library.';
		} finally {
			loading = false;
		}
	});

	function train(packId: string) {
		const s = get(setup);
		const id = crypto.randomUUID();
		startSession(id, { pack: packId, length: s.length, mode: s.mode, study: 'mixed' });
		goto(`/train/${id}/question/0`);
	}

	const filteredBirds = $derived(
		query.trim()
			? birds.filter((b) =>
					`${b.common_name} ${b.scientific_name} ${b.family ?? ''}`
						.toLowerCase()
						.includes(query.trim().toLowerCase())
				)
			: birds
	);
</script>


<main class="mx-auto max-w-[1400px] px-10 py-10">
	<div class="mb-6 flex items-end justify-between gap-4">
		<div>
			<h2 class="font-be-serif mb-2 text-3xl font-bold leading-tight">Library</h2>
			<p class="text-sm text-be-muted-fg">Your packs and every bird you've collected.</p>
		</div>
		<div class="flex shrink-0 gap-2.5">
			<input bind:this={fileInput} type="file" accept=".json,application/json" class="hidden" onchange={onPackFile} />
			<button
				onclick={() => fileInput?.click()}
				disabled={importing || $importJob.active}
				title="Import a pack from a .birdet-pack.json file"
				class="rounded-lg border border-be-border px-3.5 py-2 text-sm transition-colors hover:bg-be-secondary disabled:opacity-50"
			>
				{importing ? 'Importing…' : 'Import pack'}
			</button>
			<a
				href="/packs/custom"
				class="rounded-lg border border-be-border px-3.5 py-2 text-sm transition-colors hover:bg-be-secondary"
			>
				New pack
			</a>
			<a
				href="/import"
				class="flex items-center gap-1.5 rounded-lg bg-be-primary px-3.5 py-2 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12"/><path d="m8 11 4 4 4-4"/><path d="M8 21h8"/></svg>
				Import birds
			</a>
		</div>
	</div>

	{#if importStatus}
		<div class="mb-6 rounded-lg border border-be-border bg-be-card px-4 py-3 text-sm text-be-muted-fg">
			{#if importing}<span class="font-be-mono">↓ </span>{/if}{importStatus}
		</div>
	{/if}

	<div class="mb-6 inline-flex rounded-lg border border-be-border bg-be-card p-1">
		<button
			onclick={() => (tab = 'packs')}
			class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors {tab === 'packs'
				? 'bg-be-secondary text-be-fg'
				: 'text-be-muted-fg'}">Packs <span class="text-be-muted-fg">({packs.length})</span></button
		>
		<button
			onclick={() => (tab = 'birds')}
			class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors {tab === 'birds'
				? 'bg-be-secondary text-be-fg'
				: 'text-be-muted-fg'}">Birds <span class="text-be-muted-fg">({birds.length})</span></button
		>
	</div>

	{#if loading}
		<p class="font-be-mono text-sm text-be-muted-fg">loading…</p>
	{:else if error}
		<div class="rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">
			{error}
		</div>
	{:else if tab === 'packs'}
		{#if packs.length === 0}
			<p class="text-sm text-be-muted-fg">No packs yet.</p>
		{:else}
			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
				{#each packs as p, i (p.id)}
					<div class="flex flex-col rounded-xl border border-be-border bg-be-card p-5">
						<div class="mb-3 flex items-start justify-between gap-3">
							<span class="text-2xl leading-none">{EMOJI[i % EMOJI.length]}</span>
							<span class="font-be-mono text-xs text-be-muted-fg">{p.bird_count} species</span>
						</div>
						<h3 class="font-be-serif font-semibold leading-snug">{p.name}</h3>
						{#if p.description}
							<p class="mt-0.5 line-clamp-2 text-sm text-be-muted-fg">{p.description}</p>
						{/if}
						<div class="mt-4 flex gap-2">
							<a
								href="/packs/{p.id}"
								class="flex-1 rounded-lg border border-be-border px-3 py-1.5 text-center text-sm transition-colors hover:bg-be-secondary"
							>
								Edit
							</a>
							<button
								onclick={() => train(p.id)}
								class="flex-1 rounded-lg border border-be-border px-3 py-1.5 text-sm font-semibold transition-colors hover:border-be-primary/40 hover:text-be-primary"
							>
								Train
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{:else if birds.length === 0}
		<p class="text-sm text-be-muted-fg">No birds yet — <a href="/import" class="text-be-primary underline">import some</a>.</p>
	{:else}
		<input bind:value={query} placeholder="search birds…"
			class="mb-4 w-full max-w-md rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
		{#if filteredBirds.length === 0}
			<p class="text-sm text-be-muted-fg">No birds match “{query}”.</p>
		{:else}
			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
				{#each filteredBirds as b (b.id)}
					<div class="flex flex-col rounded-xl border border-be-border bg-be-card p-4">
						<a href="/birds/{b.id}" class="group block">
							<div class="flex items-start justify-between gap-2">
								<h3 class="truncate font-medium leading-snug group-hover:text-be-primary">{b.common_name}</h3>
								<span class="font-be-mono shrink-0 text-xs text-be-muted-fg">{b.recording_count}♪</span>
							</div>
							<p class="truncate text-xs italic text-be-muted-fg">{b.scientific_name}{#if b.family} · {b.family}{/if}</p>
						</a>
						{#if birdPacks.get(b.id)?.length}
							<div class="mt-2.5 flex flex-wrap gap-1">
								{#each birdPacks.get(b.id) ?? [] as pk (pk.id)}
									<a href="/packs/{pk.id}"
										class="rounded-full border border-be-border bg-be-secondary/50 px-2 py-0.5 text-[11px] leading-none text-be-muted-fg transition-colors hover:border-be-primary/40 hover:text-be-fg">{pk.name}</a>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</main>
