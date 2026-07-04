<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { session, score, startSession, endSession } from '$lib/stores/session';

	const id = page.params.sessionID;

	onMount(() => {
		if (!$session || $session.id !== id) goto('/');
	});

	const total = $derived($session?.answers.length ?? 0);
	const correct = $derived($session ? score($session) : 0);
	const pct = $derived(total ? Math.round((correct / total) * 100) : 0);
	const verdict = $derived(
		total === 0
			? ''
			: correct === total
				? 'Perfect score!'
				: correct >= total * 0.75
					? 'Well done!'
					: 'Keep practicing!'
	);

	function playAgain() {
		if (!$session) return goto('/');
		const cfg = $session.config;
		endSession();
		const newId = crypto.randomUUID();
		startSession(newId, cfg);
		goto(`/train/${newId}/question/0`);
	}

	function changePack() {
		endSession();
		goto('/');
	}
</script>

<header class="flex items-center gap-3 border-b border-be-border px-8 pt-8 pb-5">
	<button
		onclick={changePack}
		class="flex items-center gap-1.5 text-sm text-be-muted-fg transition-colors hover:text-be-fg"
	>
		<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg>
		Home
	</button>
</header>

{#if $session && $session.id === id}
	<main class="mx-auto max-w-2xl px-8 py-10">
		<div class="mb-10 flex items-center gap-5 border-b border-be-border pb-8">
			<div
				class="flex h-16 w-16 items-center justify-center rounded-2xl border border-be-accent/25 bg-be-accent/15"
			>
				<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#f0b429" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6"/><path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18"/><path d="M4 22h16"/><path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22"/><path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22"/><path d="M18 2H6v7a6 6 0 0 0 12 0V2Z"/></svg>
			</div>
			<div>
				<p class="font-be-serif text-5xl font-bold leading-none">
					{correct}<span class="text-2xl font-normal text-be-muted-fg">/{total}</span>
				</p>
				<p class="font-be-mono mt-1 text-sm text-be-muted-fg">{pct}% correct · {verdict}</p>
			</div>
		</div>

		<div class="mb-8 space-y-2">
			{#each $session.answers as a, i (i)}
				<div
					class="flex items-center gap-3 rounded-lg border px-4 py-3 {a.correct
						? 'border-emerald-800/30 bg-emerald-950/30'
						: 'border-red-900/25 bg-red-950/25'}"
				>
					{#if a.correct}
						<svg class="shrink-0 text-emerald-400" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.801 10A10 10 0 1 1 17 3.335"/><path d="m9 11 3 3L22 4"/></svg>
					{:else}
						<svg class="shrink-0 text-red-400" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/></svg>
					{/if}
					<div class="min-w-0 flex-1">
						<p class="text-sm font-semibold">{a.correct_name}</p>
						{#if !a.correct}
							<p class="font-be-mono mt-0.5 text-xs text-be-muted-fg">answered: {a.guess}</p>
						{/if}
					</div>
					<span class="font-be-mono shrink-0 text-xs text-be-muted-fg">{i + 1}</span>
				</div>
			{/each}
		</div>

		<div class="flex gap-3">
			<button
				onclick={playAgain}
				class="flex flex-1 items-center justify-center gap-2 rounded-lg bg-be-primary px-5 py-3 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 2v6h6"/><path d="M3 13a9 9 0 1 0 3-7.7L3 8"/></svg>
				Play Again
			</button>
			<button
				onclick={changePack}
				class="flex flex-1 items-center justify-center gap-2 rounded-lg border border-be-border px-5 py-3 text-sm text-be-fg transition-colors hover:bg-be-secondary"
			>
				Change Pack
			</button>
		</div>
	</main>
{/if}
