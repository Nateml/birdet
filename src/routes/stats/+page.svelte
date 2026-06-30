<script lang="ts">
	import { onMount } from 'svelte';
	import { getStats, type Stats } from '$lib/api/stats';

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
</script>

<h1 class="text-2xl font-bold mb-4">Your progress</h1>

{#if loading}
	<p class="opacity-70">Loading…</p>
{:else if error}
	<div class="alert alert-error">{error}</div>
{:else if stats && stats.birds.length === 0}
	<p class="opacity-70">No attempts yet. Train a session to start tracking progress.</p>
{:else if stats}
	<p class="text-lg mb-6">
		Overall: <span class="font-bold">{stats.total_correct}</span> / {stats.total_seen}
		correct ({pct(stats.total_correct, stats.total_seen)}%)
	</p>

	<div class="overflow-x-auto">
		<table class="table">
			<thead>
				<tr><th>Bird</th><th class="text-right">Seen</th><th class="text-right">Correct</th><th class="text-right">Accuracy</th></tr>
			</thead>
			<tbody>
				{#each stats.birds as b}
					<tr>
						<td>{b.common_name}</td>
						<td class="text-right">{b.seen}</td>
						<td class="text-right">{b.correct}</td>
						<td class="text-right">{pct(b.correct, b.seen)}%</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
