<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { getPacks, type PackDto } from '$lib/api/packs';
	import { getStats, type Stats } from '$lib/api/stats';
	import { startSession, type StudyMode } from '$lib/stores/session';
	import { setup } from '$lib/stores/setup';
	import { get } from 'svelte/store';

	let packs = $state<PackDto[]>([]);
	let stats = $state<Stats | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let showOptions = $state(false);

	onMount(async () => {
		try {
			[packs, stats] = await Promise.all([getPacks(), getStats().catch(() => null as never)]);
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to load packs.';
		} finally {
			loading = false;
		}
	});

	// Real packs carry no emoji yet — assign a stable one per card.
	const EMOJI = ['🏡', '🌿', '🌊', '🦅', '🐦', '🦉', '🕊️', '🐤'];

	// pack '' = whole library. study scopes the SRS queue for the session.
	function start(pack: string, study: StudyMode) {
		const s = get(setup);
		const id = crypto.randomUUID();
		startSession(id, { pack, length: s.length, mode: s.mode, study });
		goto(`/train/${id}/question/0`);
	}

	function setLength(v: number) {
		const length = Math.max(5, Math.min(50, Math.round(v) || 10));
		setup.update((s) => ({ ...s, length }));
	}

	const due = $derived(stats?.due_count ?? 0);
	const newAvail = $derived(stats?.new_count ?? 0);
	const caughtUp = $derived(!!stats && due === 0 && newAvail === 0 && stats.total_birds > 0);

	// Soonest upcoming review, for the "caught up — back in Xh" hint.
	function relFuture(ms: number): string {
		const diff = ms - Date.now();
		if (diff <= 0) return 'now';
		const m = diff / 60000;
		if (m < 60) return `${Math.round(m)}m`;
		const h = m / 60;
		if (h < 24) return `${Math.round(h)}h`;
		return `${Math.round(h / 24)}d`;
	}
	const nextDue = $derived.by(() => {
		const times = (stats?.birds ?? [])
			.filter((b) => b.state !== 'new' && b.due_at)
			.map((b) => new Date(b.due_at!.replace(' ', 'T') + 'Z').getTime())
			.filter((t) => t > Date.now());
		return times.length ? relFuture(Math.min(...times)) : null;
	});
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
	{#if showOptions}
		<div class="mb-8 rounded-xl border border-be-border bg-be-card p-5">
			<p class="font-be-mono mb-3 text-xs uppercase tracking-widest text-be-muted-fg">
				Session options
			</p>
			<label class="flex items-center justify-between gap-4">
				<span class="text-sm">New birds per session</span>
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

	{#if error}
		<div
			class="mb-8 rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive"
		>
			{error}
		</div>
	{/if}

	<!-- Global study driver — the whole library, SRS-scheduled -->
	<div class="mb-4 rounded-2xl border border-be-primary/25 bg-be-primary/[0.06] p-6">
		{#if caughtUp}
			<div class="flex items-start justify-between gap-4">
				<div>
					<h2 class="font-be-serif mb-1 text-2xl font-bold leading-tight">All caught up 🎉</h2>
					<p class="text-sm text-be-muted-fg">
						Nothing due right now{#if nextDue}, next review in <span class="text-be-fg">{nextDue}</span>{/if}. Practice anyway to
						keep sharp.
					</p>
				</div>
			</div>
			<div class="mt-5 flex flex-wrap gap-2.5">
				<button
					onclick={() => start('', 'cram')}
					class="flex items-center gap-2 rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
				>
					Practice anyway
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
				</button>
			</div>
		{:else}
			<div class="mb-5 flex items-start justify-between gap-4">
				<div>
					<h2 class="font-be-serif mb-1 text-2xl font-bold leading-tight">Learn &amp; Review</h2>
					<p class="text-sm text-be-muted-fg">
						Your whole collection, scheduled. Due cards first, then up to {$setup.length} new.
					</p>
				</div>
				<div class="flex shrink-0 gap-4 text-right">
					<div>
						<p class="font-be-serif text-2xl font-bold {due > 0 ? 'text-be-accent' : ''}">{due}</p>
						<p class="font-be-mono text-[11px] text-be-muted-fg">due</p>
					</div>
					<div>
						<p class="font-be-serif text-2xl font-bold">{newAvail}</p>
						<p class="font-be-mono text-[11px] text-be-muted-fg">new</p>
					</div>
				</div>
			</div>
			<div class="flex flex-wrap gap-2.5">
				<button
					onclick={() => start('', 'mixed')}
					class="flex items-center gap-2 rounded-lg bg-be-primary px-5 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
				>
					Start session
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
				</button>
				{#if due > 0}
					<button
						onclick={() => start('', 'review')}
						class="rounded-lg border border-be-accent/40 px-4 py-2.5 text-sm font-semibold text-be-accent transition-colors hover:bg-be-accent/10"
					>
						Review due · {due}
					</button>
				{/if}
				{#if newAvail > 0}
					<button
						onclick={() => start('', 'new')}
						class="rounded-lg border border-be-border px-4 py-2.5 text-sm text-be-fg transition-colors hover:bg-be-secondary"
					>
						New only
					</button>
				{/if}
				<button
					onclick={() => start('', 'cram')}
					class="rounded-lg px-3 py-2.5 text-sm text-be-muted-fg transition-colors hover:text-be-fg"
				>
					Practice anyway
				</button>
			</div>
		{/if}
	</div>

	<!-- Packs = optional filters over the same global schedule -->
	<div class="mb-4 mt-10">
		<h3 class="font-be-serif text-lg font-semibold">Focus a pack</h3>
		<p class="text-sm text-be-muted-fg">Same schedule, narrowed to a curated set of species.</p>
	</div>

	{#if loading}
		<p class="font-be-mono text-sm text-be-muted-fg">loading packs…</p>
	{:else if packs.length === 0}
		<p class="text-sm text-be-muted-fg">No packs yet.</p>
	{:else}
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			{#each packs as p, i (p.id)}
				<button
					onclick={() => start(p.id, 'mixed')}
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
						Train this pack
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
					</div>
				</button>
			{/each}
		</div>
	{/if}
</main>
