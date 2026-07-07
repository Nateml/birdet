<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import { page } from '$app/state';
	import AppHeader from '$lib/components/AppHeader.svelte';
	import { startSession } from '$lib/stores/session';
	import { setup } from '$lib/stores/setup';
	import {
		getPacks,
		getPackBirds,
		getBirds,
		renamePack,
		deletePack,
		addBirdsToPack,
		removeBirdFromPack,
		exportPack,
		type BirdListItem
	} from '$lib/api/packs';

	const packId = page.params.packId!;

	let name = $state('');
	let originalName = $state('');
	let packBirds = $state<BirdListItem[]>([]);
	let allBirds = $state<BirdListItem[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let busy = $state(false);

	// Add-birds panel
	let adding = $state(false);
	let query = $state('');
	let toAdd = $state<Set<number>>(new Set());

	async function load() {
		const [packs, pb, all] = await Promise.all([getPacks(), getPackBirds(packId), getBirds()]);
		const pack = packs.find((p) => p.id === packId);
		if (!pack) throw new Error('Pack not found.');
		name = pack.name;
		originalName = pack.name;
		packBirds = pb;
		allBirds = all;
	}

	onMount(async () => {
		try {
			await load();
		} catch (e) {
			error = (e as Error)?.message ?? (e as string) ?? 'Failed to load pack.';
		} finally {
			loading = false;
		}
	});

	const inPack = $derived(new Set(packBirds.map((b) => b.id)));
	const candidates = $derived(
		allBirds
			.filter((b) => !inPack.has(b.id))
			.filter((b) =>
				query.trim()
					? `${b.common_name} ${b.scientific_name} ${b.family ?? ''}`
							.toLowerCase()
							.includes(query.trim().toLowerCase())
					: true
			)
	);

	async function saveName() {
		if (busy || name.trim() === originalName || !name.trim()) return;
		busy = true;
		error = null;
		try {
			await renamePack(packId, name.trim());
			originalName = name.trim();
		} catch (e) {
			error = (e as string) ?? 'Rename failed.';
			name = originalName;
		} finally {
			busy = false;
		}
	}

	async function remove(birdId: number) {
		if (busy) return;
		busy = true;
		error = null;
		try {
			await removeBirdFromPack(packId, birdId);
			packBirds = packBirds.filter((b) => b.id !== birdId);
		} catch (e) {
			error = (e as string) ?? 'Remove failed.';
		} finally {
			busy = false;
		}
	}

	function toggleAdd(id: number) {
		const next = new Set(toAdd);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		toAdd = next;
	}

	async function saveAdditions() {
		if (busy || toAdd.size === 0) return;
		busy = true;
		error = null;
		try {
			await addBirdsToPack(packId, [...toAdd]);
			packBirds = await getPackBirds(packId);
			toAdd = new Set();
			query = '';
			adding = false;
		} catch (e) {
			error = (e as string) ?? 'Add failed.';
		} finally {
			busy = false;
		}
	}

	let exportMsg = $state<string | null>(null);
	async function doExport() {
		if (busy) return;
		busy = true;
		error = null;
		exportMsg = null;
		try {
			const path = await exportPack(packId);
			exportMsg = `Saved to ${path}`;
		} catch (e) {
			error = (e as string) ?? 'Export failed.';
		} finally {
			busy = false;
		}
	}

	let confirmingDelete = $state(false);
	async function doDelete() {
		if (busy) return;
		busy = true;
		try {
			await deletePack(packId);
			goto('/library');
		} catch (e) {
			error = (e as string) ?? 'Delete failed.';
			busy = false;
		}
	}

	function train() {
		const s = get(setup);
		const id = crypto.randomUUID();
		startSession(id, { pack: packId, length: s.length, mode: s.mode, study: 'mixed' });
		goto(`/train/${id}/question/0`);
	}
</script>

<AppHeader />

<main class="mx-auto max-w-2xl px-8 py-10">
	<a href="/library" class="mb-4 inline-flex items-center gap-1 text-sm text-be-muted-fg transition-colors hover:text-be-fg">
		<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
		Library
	</a>

	{#if loading}
		<p class="font-be-mono text-sm text-be-muted-fg">loading…</p>
	{:else if error && packBirds.length === 0 && !originalName}
		<div class="rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">{error}</div>
	{:else}
		<div class="mb-6">
			<label class="block">
				<span class="mb-1.5 block text-xs font-medium text-be-muted-fg">Pack name</span>
				<input bind:value={name} onblur={saveName} disabled={busy}
					class="font-be-serif w-full rounded-lg border border-transparent bg-transparent text-2xl font-bold leading-tight text-be-fg outline-none hover:border-be-border focus:border-be-primary/40 focus:px-3 focus:py-1" />
			</label>
		</div>

		{#if error}
			<div class="mb-4 rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">{error}</div>
		{/if}

		<div class="mb-3 flex items-center justify-between">
			<span class="text-sm font-medium">Birds <span class="text-be-muted-fg">({packBirds.length})</span></span>
			<button onclick={() => (adding = !adding)}
				class="rounded-lg border border-be-border px-3 py-1.5 text-sm transition-colors hover:bg-be-secondary">
				{adding ? 'Done adding' : '+ Add birds'}
			</button>
		</div>

		{#if adding}
			<div class="mb-4 rounded-xl border border-be-border bg-be-card p-4">
				<input bind:value={query} placeholder="search your library…"
					class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40" />
				<div class="mt-3 max-h-60 space-y-1 overflow-y-auto">
					{#if candidates.length === 0}
						<p class="px-2 py-1.5 text-sm text-be-muted-fg">No other library birds match.</p>
					{:else}
						{#each candidates as b (b.id)}
							<button onclick={() => toggleAdd(b.id)}
								class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors {toAdd.has(b.id) ? 'bg-be-primary/10' : 'hover:bg-be-secondary'}">
								<span class="flex h-4 w-4 shrink-0 items-center justify-center rounded border {toAdd.has(b.id) ? 'border-be-primary bg-be-primary text-be-primary-fg' : 'border-be-border'}">
									{#if toAdd.has(b.id)}<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>{/if}
								</span>
								<span class="min-w-0 flex-1">
									<span class="block truncate text-sm font-medium">{b.common_name}</span>
									<span class="block truncate text-xs italic text-be-muted-fg">{b.scientific_name}</span>
								</span>
							</button>
						{/each}
					{/if}
				</div>
				<button onclick={saveAdditions} disabled={busy || toAdd.size === 0}
					class="mt-3 w-full rounded-lg bg-be-primary px-4 py-2 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50">
					{toAdd.size ? `Add ${toAdd.size} to pack` : 'Select birds to add'}
				</button>
			</div>
		{/if}

		{#if packBirds.length === 0}
			<p class="rounded-xl border border-be-border bg-be-card px-5 py-6 text-center text-sm text-be-muted-fg">
				This pack is empty. Add birds to train it.
			</p>
		{:else}
			<div class="divide-y divide-be-border overflow-hidden rounded-xl border border-be-border bg-be-card">
				{#each packBirds as b (b.id)}
					<div class="flex items-center gap-4 px-5 py-3">
						<div class="min-w-0 flex-1">
							<h3 class="truncate font-medium leading-snug">{b.common_name}</h3>
							<p class="truncate text-xs italic text-be-muted-fg">{b.scientific_name}{#if b.family} · {b.family}{/if}</p>
						</div>
						<span class="font-be-mono shrink-0 text-xs text-be-muted-fg">{b.recording_count}♪</span>
						<button onclick={() => remove(b.id)} disabled={busy} title="Remove from pack"
							class="shrink-0 rounded-md p-1.5 text-be-muted-fg transition-colors hover:bg-be-destructive/10 hover:text-be-destructive">
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
						</button>
					</div>
				{/each}
			</div>
		{/if}

		<div class="mt-8 flex items-center justify-between gap-4 border-t border-be-border pt-6">
			{#if confirmingDelete}
				<div class="flex items-center gap-2 text-sm">
					<span class="text-be-muted-fg">Delete this pack?</span>
					<button onclick={doDelete} disabled={busy}
						class="rounded-lg bg-be-destructive px-3 py-1.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50">Delete</button>
					<button onclick={() => (confirmingDelete = false)}
						class="rounded-lg border border-be-border px-3 py-1.5 text-sm transition-colors hover:bg-be-secondary">Cancel</button>
				</div>
			{:else}
				<button onclick={() => (confirmingDelete = true)}
					class="text-sm text-be-muted-fg transition-colors hover:text-be-destructive">Delete pack</button>
			{/if}
			<div class="flex items-center gap-2.5">
				<button onclick={doExport} disabled={busy || packBirds.length === 0} title="Save this pack as a shareable file"
					class="rounded-lg border border-be-border px-4 py-2.5 text-sm transition-colors hover:bg-be-secondary disabled:opacity-50">Export</button>
				<button onclick={train} disabled={packBirds.length === 0}
					class="rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 disabled:opacity-50">Train</button>
			</div>
		</div>
		{#if exportMsg}
			<p class="mt-3 break-all text-right text-xs text-be-muted-fg">{exportMsg}</p>
		{/if}
	{/if}
</main>
