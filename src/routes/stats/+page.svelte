<script lang="ts">
	import { onMount } from 'svelte';
	import { getStats, type Stats, type BirdStat } from '$lib/api/stats';

	let stats = $state<Stats | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	const pct = (correct: number, seen: number) =>
		seen === 0 ? 0 : Math.round((correct / seen) * 100);

	onMount(async () => {
		try {
			stats = await getStats();
		} catch (e) {
			error = (e as Error)?.message ?? 'Failed to load stats.';
		} finally {
			loading = false;
		}
	});

	// SQLite stores 'YYYY-MM-DD HH:MM:SS' in UTC; make it a real instant.
	function relDue(due_at: string | null): string {
		if (!due_at) return '';
		const due = new Date(due_at.replace(' ', 'T') + 'Z').getTime();
		const diff = due - Date.now();
		if (diff <= 0) return 'due now';
		const m = diff / 60000;
		if (m < 60) return `in ${Math.round(m)}m`;
		const h = m / 60;
		if (h < 24) return `in ${Math.round(h)}h`;
		return `in ${Math.round(h / 24)}d`;
	}

	// state → { label, dot color class, text/badge classes }
	const BADGE: Record<string, { label: string; badge: string; dot: string }> = {
		new: { label: 'New', badge: 'bg-be-muted text-be-muted-fg', dot: 'bg-be-muted-fg' },
		learning: { label: 'Learning', badge: 'bg-be-accent/15 text-be-accent', dot: 'bg-be-accent' },
		review: { label: 'Review', badge: 'bg-be-primary/15 text-be-primary', dot: 'bg-be-primary' },
		mastered: {
			label: 'Mastered',
			badge: 'bg-emerald-500/15 text-emerald-400',
			dot: 'bg-emerald-400'
		}
	};
	const badge = (s: string) => BADGE[s] ?? BADGE.new;

	// Deck composition segments (in study-progression order).
	const segments = $derived(
		stats
			? [
					{ key: 'new', n: stats.new_count },
					{ key: 'learning', n: stats.learning_count },
					{ key: 'review', n: stats.review_count },
					{ key: 'mastered', n: stats.mastered_count }
				]
			: []
	);

	const isDue = (b: BirdStat) => b.state !== 'new' && relDue(b.due_at) === 'due now';
</script>


<main class="mx-auto max-w-5xl px-10 py-10">
	<div class="mb-8">
		<h2 class="font-be-serif mb-2 text-3xl font-bold leading-tight">Your Progress</h2>
		<p class="text-sm text-be-muted-fg">How well the flock is settling into long-term memory.</p>
	</div>

	{#if loading}
		<p class="font-be-mono text-sm text-be-muted-fg">loading stats…</p>
	{:else if error}
		<div
			class="rounded-lg border border-be-destructive/40 bg-be-destructive/10 px-4 py-3 text-sm text-be-destructive"
		>
			{error}
		</div>
	{:else if stats && stats.total_seen === 0}
		<div class="rounded-xl border border-be-border bg-be-card p-8 text-center">
			<div class="mb-3 text-4xl">🐣</div>
			<p class="mb-1 font-be-serif text-lg font-semibold">No attempts yet</p>
			<p class="text-sm text-be-muted-fg">
				{stats.total_birds} birds waiting. Train a session to start tracking progress.
			</p>
			<a
				href="/"
				class="mt-4 inline-flex items-center gap-1.5 rounded-lg bg-be-primary px-4 py-2 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
			>
				Start training
			</a>
		</div>
	{:else if stats}
		<!-- Summary tiles -->
		<div class="mb-6 grid grid-cols-3 gap-4">
			<div class="rounded-xl border border-be-border bg-be-card p-5">
				<p class="font-be-mono mb-1 text-xs uppercase tracking-widest text-be-muted-fg">Accuracy</p>
				<p class="font-be-serif text-3xl font-bold text-be-primary">
					{pct(stats.total_correct, stats.total_seen)}%
				</p>
				<p class="font-be-mono mt-1 text-xs text-be-muted-fg">
					{stats.total_correct}/{stats.total_seen} attempts
				</p>
			</div>
			<div class="rounded-xl border border-be-border bg-be-card p-5">
				<p class="font-be-mono mb-1 text-xs uppercase tracking-widest text-be-muted-fg">Due now</p>
				<p class="font-be-serif text-3xl font-bold {stats.due_count > 0 ? 'text-be-accent' : ''}">
					{stats.due_count}
				</p>
				<p class="font-be-mono mt-1 text-xs text-be-muted-fg">cards to review</p>
			</div>
			<div class="rounded-xl border border-be-border bg-be-card p-5">
				<p class="font-be-mono mb-1 text-xs uppercase tracking-widest text-be-muted-fg">Mastered</p>
				<p class="font-be-serif text-3xl font-bold text-emerald-400">
					{stats.mastered_count}<span class="text-lg font-normal text-be-muted-fg"
						>/{stats.total_birds}</span
					>
				</p>
				<p class="font-be-mono mt-1 text-xs text-be-muted-fg">mature (≥21d)</p>
			</div>
		</div>

		<!-- Deck composition -->
		<div class="mb-8 rounded-xl border border-be-border bg-be-card p-5">
			<p class="font-be-mono mb-3 text-xs uppercase tracking-widest text-be-muted-fg">
				Collection · {stats.total_birds} birds
			</p>
			<div class="flex h-2.5 w-full overflow-hidden rounded-full bg-be-muted">
				{#each segments as s (s.key)}
					{#if s.n > 0}
						<div
							class="{badge(s.key).dot} h-full"
							style="width: {(s.n / stats.total_birds) * 100}%"
							title="{badge(s.key).label}: {s.n}"
						></div>
					{/if}
				{/each}
			</div>
			<div class="mt-3 flex flex-wrap gap-x-5 gap-y-1.5">
				{#each segments as s (s.key)}
					<div class="flex items-center gap-1.5">
						<span class="h-2 w-2 rounded-full {badge(s.key).dot}"></span>
						<span class="text-xs text-be-fg">{badge(s.key).label}</span>
						<span class="font-be-mono text-xs text-be-muted-fg">{s.n}</span>
					</div>
				{/each}
			</div>
		</div>

		<!-- Per-species -->
		<p class="font-be-mono mb-3 text-xs uppercase tracking-widest text-be-muted-fg">By species</p>
		<div class="grid grid-cols-1 gap-2 lg:grid-cols-2">
			{#each stats.birds as b (b.common_name)}
				{@const p = pct(b.correct, b.seen)}
				<div
					class="flex items-center gap-3 rounded-xl border bg-be-card px-5 py-3.5 {isDue(b)
						? 'border-be-accent/40'
						: 'border-be-border'}"
				>
					<span
						class="font-be-mono shrink-0 rounded-full px-2 py-0.5 text-[11px] font-medium {badge(
							b.state
						).badge}">{badge(b.state).label}</span
					>
					<div class="min-w-0 flex-1">
						<p class="font-be-serif truncate font-semibold">{b.common_name}</p>
						<div class="mt-1 h-1 w-full overflow-hidden rounded-full bg-be-secondary">
							<div class="h-full rounded-full bg-be-primary" style="width: {p}%"></div>
						</div>
					</div>
					<div class="shrink-0 text-right">
						<p class="font-be-mono text-xs text-be-fg">{p}%</p>
						<p class="font-be-mono mt-0.5 text-[11px] {isDue(b) ? 'text-be-accent' : 'text-be-muted-fg'}">
							{#if b.state === 'new'}
								unseen
							{:else}
								{relDue(b.due_at)}{#if b.lapses > 0} · {b.lapses}✕{/if}
							{/if}
						</p>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</main>
