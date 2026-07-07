<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { getPacks, type PackDto } from '$lib/api/packs';
	import { getStats, type Stats } from '$lib/api/stats';
	import { startSession, type StudyMode } from '$lib/stores/session';
	import { setup } from '$lib/stores/setup';
	import { getSetting, SETTING_EBIRD_KEY, SETTING_XC_KEY } from '$lib/api/settings';
	import { get } from 'svelte/store';

	let packs = $state<PackDto[]>([]);
	let stats = $state<Stats | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let hasKeys = $state(true);

	onMount(async () => {
		try {
			const [p, s, eb, xc] = await Promise.all([
				getPacks(),
				getStats().catch(() => null as never),
				getSetting(SETTING_EBIRD_KEY),
				getSetting(SETTING_XC_KEY)
			]);
			packs = p;
			stats = s;
			hasKeys = !!eb.trim() && !!xc.trim();
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to load packs.';
		} finally {
			loading = false;
		}
	});

	// First run: library is empty, so show onboarding instead of the scheduler.
	const firstRun = $derived(!loading && !!stats && stats.total_birds === 0);

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

<main class="mx-auto max-w-3xl px-8 py-10">
	<div class="mb-8">
		<h2 class="font-be-serif text-3xl font-bold leading-tight">Train</h2>
		<p class="text-sm text-be-muted-fg">Your review schedule and pack picker.</p>
	</div>


	{#if error}
		<div
			class="mb-8 rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive"
		>
			{error}
		</div>
	{/if}

	{#if firstRun}
		<!-- First-run onboarding: no birds collected yet -->
		<div class="rounded-2xl border border-be-primary/25 bg-be-primary/[0.06] p-8">
			<div class="mb-1 text-4xl">🐣</div>
			<h2 class="font-be-serif mb-1.5 text-2xl font-bold leading-tight">Welcome to Birdet</h2>
			<p class="mb-6 text-sm text-be-muted-fg">
				Build a collection of local birds, then train your ear. Two quick steps:
			</p>

			<ol class="space-y-4">
				<li class="flex gap-3">
					<span class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full {hasKeys ? 'bg-be-primary text-be-primary-fg' : 'bg-be-secondary text-be-fg'} text-xs font-bold">
						{#if hasKeys}✓{:else}1{/if}
					</span>
					<div class="min-w-0">
						<p class="text-sm font-medium">Add your API keys</p>
						<p class="text-sm text-be-muted-fg">
							Free eBird + Xeno-Canto keys let Birdet find species and fetch recordings.
							{#if hasKeys}<span class="text-be-primary"> Done.</span>{/if}
						</p>
						{#if !hasKeys}
							<a href="/settings" class="mt-2 inline-flex rounded-lg border border-be-border px-3.5 py-1.5 text-sm transition-colors hover:bg-be-secondary">Open Settings</a>
						{/if}
					</div>
				</li>
				<li class="flex gap-3">
					<span class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-be-secondary text-xs font-bold text-be-fg">2</span>
					<div class="min-w-0">
						<p class="text-sm font-medium">Import birds</p>
						<p class="text-sm text-be-muted-fg">Pull the most common species from your region, or search by name.</p>
						<a
							href="/import"
							class="mt-2 inline-flex items-center gap-1.5 rounded-lg bg-be-primary px-4 py-2 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90 {hasKeys ? '' : 'pointer-events-none opacity-50'}"
						>
							Import birds
							<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
						</a>
					</div>
				</li>
			</ol>

			<p class="mt-6 border-t border-be-border pt-4 text-xs text-be-muted-fg">
				New here? The <a href="/guide" class="text-be-primary hover:underline">Guide</a> explains how training works.
			</p>
		</div>
	{:else}
	{#if !hasKeys}
		<a href="/settings" class="mb-4 flex items-center gap-2 rounded-lg border border-be-accent/40 bg-be-accent/10 px-4 py-3 text-sm text-be-accent transition-opacity hover:opacity-90">
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 9v4M12 17h.01M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/></svg>
			Add your eBird + Xeno-Canto API keys in Settings to import more birds.
		</a>
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
	{/if}
</main>
