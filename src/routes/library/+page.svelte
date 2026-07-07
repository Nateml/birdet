<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { page } from '$app/state';
	import { getPacks, getBirds, getBirdPacks, importPack, type PackDto, type BirdListItem } from '$lib/api/packs';
	import { runImportJob, importJob } from '$lib/stores/importJob';
	import PackEditor from '$lib/components/PackEditor.svelte';
	import BirdDetail from '$lib/components/BirdDetail.svelte';

	type Tab = 'packs' | 'birds';
	let tab = $state<Tab>('packs');

	type Sel = { kind: 'pack'; id: string } | { kind: 'bird'; id: number } | null;
	let sel = $state<Sel>(null);

	let masterEl = $state<HTMLElement | null>(null);

	// Deep-link selection via ?bird=<id> / ?pack=<id> (used by the command
	// palette and direct links). Runs whenever those params change.
	$effect(() => {
		const b = page.url.searchParams.get('bird');
		const p = page.url.searchParams.get('pack');
		if (b) {
			tab = 'birds';
			query = '';
			sel = { kind: 'bird', id: Number(b) };
		} else if (p) {
			tab = 'packs';
			sel = { kind: 'pack', id: p };
		}
	});

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

	async function loadData() {
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
	}

	onMount(async () => {
		try {
			await loadData();
		} catch (e) {
			error = (e as Error)?.message ?? (e as string) ?? 'Failed to load library.';
		} finally {
			loading = false;
		}
	});

	function switchTab(t: Tab) {
		if (tab === t) return;
		tab = t;
		sel = null;
	}

	// ↑/↓ move the selection through the active master list.
	async function moveSel(dir: 1 | -1) {
		const packMode = tab === 'packs';
		const list: Array<{ id: string | number }> = packMode ? packs : filteredBirds;
		if (list.length === 0) return;
		let idx = -1;
		if (sel) {
			idx = list.findIndex((x) =>
				packMode ? sel!.kind === 'pack' && sel!.id === x.id : sel!.kind === 'bird' && sel!.id === x.id
			);
		}
		const next = idx < 0 ? (dir > 0 ? 0 : list.length - 1) : Math.min(list.length - 1, Math.max(0, idx + dir));
		const item = list[next];
		sel = packMode ? { kind: 'pack', id: item.id as string } : { kind: 'bird', id: item.id as number };
		await tick();
		masterEl?.querySelector('[data-active]')?.scrollIntoView({ block: 'nearest' });
	}

	function onMasterKey(e: KeyboardEvent) {
		const t = e.target as HTMLElement;
		if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT')) return;
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			void moveSel(1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			void moveSel(-1);
		}
	}

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
			await loadData();
			tab = 'packs';
			sel = { kind: 'pack', id: r.pack_id };
		} catch (err) {
			error = (err as string) ?? 'Pack import failed.';
			importStatus = null;
		} finally {
			importing = false;
		}
	}

	// Detail callbacks — keep the master list counts/names in sync.
	async function onDetailChanged() {
		try {
			await loadData();
		} catch {
			// non-fatal: the detail pane already reflects the change
		}
	}
	async function onPackDeleted() {
		sel = null;
		await onDetailChanged();
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

<div class="flex h-full flex-col">
	<!-- Toolbar -->
	<div class="flex items-center justify-between gap-4 border-b border-be-border px-6 py-4">
		<div class="flex items-center gap-5">
			<h2 class="font-be-serif text-xl font-bold leading-none">Library</h2>
			<div class="inline-flex rounded-lg border border-be-border bg-be-card p-0.5">
				<button
					onclick={() => switchTab('packs')}
					class="rounded-md px-3 py-1 text-sm font-medium transition-colors {tab === 'packs'
						? 'bg-be-secondary text-be-fg'
						: 'text-be-muted-fg'}">Packs <span class="text-be-muted-fg">({packs.length})</span></button
				>
				<button
					onclick={() => switchTab('birds')}
					class="rounded-md px-3 py-1 text-sm font-medium transition-colors {tab === 'birds'
						? 'bg-be-secondary text-be-fg'
						: 'text-be-muted-fg'}">Birds <span class="text-be-muted-fg">({birds.length})</span></button
				>
			</div>
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
		<div class="border-b border-be-border bg-be-card px-6 py-2.5 text-sm text-be-muted-fg">
			{#if importing}<span class="font-be-mono">↓ </span>{/if}{importStatus}
		</div>
	{/if}

	{#if loading}
		<p class="font-be-mono px-6 py-6 text-sm text-be-muted-fg">loading…</p>
	{:else if error}
		<div class="m-6 rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive">
			{error}
		</div>
	{:else}
		<div class="flex min-h-0 flex-1">
			<!-- Master list -->
			<div class="flex w-80 shrink-0 flex-col border-r border-be-border" onkeydown={onMasterKey} role="listbox" tabindex="-1" aria-label="Library items">
				{#if tab === 'birds'}
					<div class="border-b border-be-border p-3">
						<input bind:value={query} placeholder="search birds…"
							class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-1.5 text-sm text-be-fg outline-none focus:border-be-primary/40" />
					</div>
				{/if}
				<div bind:this={masterEl} class="flex-1 overflow-y-auto p-2">
					{#if tab === 'packs'}
						{#if packs.length === 0}
							<p class="px-3 py-4 text-sm text-be-muted-fg">No packs yet.</p>
						{:else}
							{#each packs as p, i (p.id)}
								{@const active = sel?.kind === 'pack' && sel.id === p.id}
								<button
									onclick={() => (sel = { kind: 'pack', id: p.id })}
									data-active={active ? '' : undefined}
									class="mb-1 flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors {active
										? 'bg-be-secondary'
										: 'hover:bg-be-secondary/50'}"
								>
									<span class="text-xl leading-none">{EMOJI[i % EMOJI.length]}</span>
									<span class="min-w-0 flex-1">
										<span class="block truncate font-be-serif text-sm font-semibold">{p.name}</span>
										<span class="font-be-mono block text-[11px] text-be-muted-fg">{p.bird_count} species</span>
									</span>
								</button>
							{/each}
						{/if}
					{:else if birds.length === 0}
						<p class="px-3 py-4 text-sm text-be-muted-fg">No birds yet — <a href="/import" class="text-be-primary underline">import some</a>.</p>
					{:else if filteredBirds.length === 0}
						<p class="px-3 py-4 text-sm text-be-muted-fg">No birds match “{query}”.</p>
					{:else}
						{#each filteredBirds as b (b.id)}
							{@const active = sel?.kind === 'bird' && sel.id === b.id}
							<button
								onclick={() => (sel = { kind: 'bird', id: b.id })}
								data-active={active ? '' : undefined}
								class="mb-1 flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition-colors {active
									? 'bg-be-secondary'
									: 'hover:bg-be-secondary/50'}"
							>
								<span class="min-w-0 flex-1">
									<span class="block truncate text-sm font-medium">{b.common_name}</span>
									<span class="block truncate text-[11px] italic text-be-muted-fg">{b.scientific_name}</span>
								</span>
								<span class="font-be-mono shrink-0 text-[11px] text-be-muted-fg">{b.recording_count}♪</span>
							</button>
						{/each}
					{/if}
				</div>
			</div>

			<!-- Detail pane -->
			<div class="min-w-0 flex-1 overflow-y-auto">
				{#if sel?.kind === 'pack'}
					{#key sel.id}
						<PackEditor
							packId={sel.id}
							onChanged={onDetailChanged}
							onDeleted={onPackDeleted}
							onSelectBird={(id) => {
								tab = 'birds';
								query = '';
								sel = { kind: 'bird', id };
							}}
						/>
					{/key}
				{:else if sel?.kind === 'bird'}
					{#key sel.id}
						<BirdDetail birdId={sel.id} onChanged={onDetailChanged} />
					{/key}
				{:else}
					<div class="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
						<div class="text-4xl opacity-40">{tab === 'packs' ? '📚' : '🐦'}</div>
						<p class="text-sm text-be-muted-fg">
							Select a {tab === 'packs' ? 'pack' : 'bird'} to {tab === 'packs' ? 'edit it' : 'manage its recordings'}.
						</p>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
