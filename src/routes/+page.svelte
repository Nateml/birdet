<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { getPacks, type PackDto } from '$lib/api/packs';
	import { startSession } from '$lib/stores/session';
	import { setup } from '$lib/stores/setup';
	import { get } from 'svelte/store';

	let packs = $state<PackDto[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let showOptions = $state(false);

	onMount(async () => {
		try {
			packs = await getPacks();
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to load packs.';
		} finally {
			loading = false;
		}
	});

	// Real packs carry no emoji yet — assign a stable one per card.
	const EMOJI = ['🏡', '🌿', '🌊', '🦅', '🐦', '🦉', '🕊️', '🐤'];

	function start(packId: string) {
		const s = get(setup);
		const id = crypto.randomUUID();
		startSession(id, { pack: packId, length: s.length, mode: s.mode });
		goto(`/train/${id}/question/0`);
	}

	function setLength(v: number) {
		const length = Math.max(5, Math.min(50, Math.round(v) || 10));
		setup.update((s) => ({ ...s, length }));
	}
</script>

<header class="flex items-center justify-between border-b border-be-border px-8 pt-8 pb-5">
	<div class="flex items-center gap-3">
		<div
			class="flex h-9 w-9 items-center justify-center rounded-lg border border-be-primary/20 bg-be-primary/10 text-lg"
		>
			🐦
		</div>
		<div>
			<h1 class="font-be-serif text-xl font-bold leading-none">Birdet</h1>
			<p class="font-be-mono mt-0.5 text-xs text-be-muted-fg">birdsong identification trainer</p>
		</div>
	</div>
	<nav class="flex items-center gap-4 text-sm text-be-muted-fg">
		<a href="/stats" class="transition-colors hover:text-be-fg">Stats</a>
		<a href="/library" class="hidden transition-colors hover:text-be-fg sm:inline">Library</a>
		<a href="/settings" class="hidden transition-colors hover:text-be-fg sm:inline">Settings</a>
		<button
			onclick={() => (showOptions = !showOptions)}
			class="flex items-center gap-1.5 rounded-lg border border-be-border px-3 py-1.5 transition-colors hover:border-be-primary/40 hover:text-be-fg"
			class:text-be-fg={showOptions}
			aria-expanded={showOptions}
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
			Options
		</button>
	</nav>
</header>

<main class="mx-auto max-w-3xl px-8 py-10">
	<div class="mb-8">
		<h2 class="font-be-serif mb-2 text-3xl font-bold leading-tight">Choose a Pack</h2>
		<p class="text-sm text-be-muted-fg">
			Each session draws {$setup.length} questions from the pack’s species.
		</p>
	</div>

	{#if showOptions}
		<div class="mb-8 rounded-xl border border-be-border bg-be-card p-5">
			<p class="font-be-mono mb-3 text-xs uppercase tracking-widest text-be-muted-fg">
				Session options
			</p>
			<label class="flex items-center justify-between gap-4">
				<span class="text-sm">Questions per session</span>
				<input
					type="number"
					min="5"
					max="50"
					value={$setup.length}
					oninput={(e) => setLength(+e.currentTarget.value)}
					class="w-20 rounded-lg border border-be-border bg-be-bg px-3 py-1.5 text-right text-sm text-be-fg outline-none focus:border-be-primary/40"
				/>
			</label>
		</div>
	{/if}

	{#if loading}
		<p class="font-be-mono text-sm text-be-muted-fg">loading packs…</p>
	{:else if error}
		<div
			class="rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive"
		>
			{error}
		</div>
	{:else if packs.length === 0}
		<p class="text-sm text-be-muted-fg">No packs yet.</p>
	{:else}
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			{#each packs as p, i (p.id)}
				<button
					onclick={() => start(p.id)}
					class="group rounded-xl border border-be-border bg-be-card p-6 text-left transition-all duration-200 hover:border-be-primary/35 hover:bg-be-secondary"
				>
					<div class="mb-3 flex items-start justify-between">
						<span class="text-3xl leading-none">{EMOJI[i % EMOJI.length]}</span>
						<span class="font-be-mono text-xs text-be-muted-fg">{p.bird_count} species</span>
					</div>
					<h3
						class="font-be-serif mb-1.5 text-lg font-semibold leading-snug transition-colors group-hover:text-be-primary"
					>
						{p.name}
					</h3>
					{#if p.description}
						<p class="text-sm leading-relaxed text-be-muted-fg">{p.description}</p>
					{/if}
					<div
						class="mt-4 flex items-center gap-1.5 text-sm font-semibold text-be-primary opacity-0 transition-opacity group-hover:opacity-100"
					>
						Start training
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
					</div>
				</button>
			{/each}
		</div>
	{/if}
</main>
