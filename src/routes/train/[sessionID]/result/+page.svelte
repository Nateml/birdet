<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { session, score, endSession } from '$lib/stores/session';

	const id = page.params.sessionID;

	onMount(() => {
		if (!$session || $session.id !== id) goto('/train');
	});

	function again() {
		endSession();
		goto('/train');
	}
</script>

{#if $session && $session.id === id}
	<h1 class="text-2xl font-bold mb-2">Session complete</h1>
	<p class="text-lg mb-6">
		Score: <span class="font-bold">{score($session)}</span> / {$session.answers.length}
	</p>

	<ul class="grid gap-2 mb-6">
		{#each $session.answers as a, i}
			<li class="flex items-center gap-3">
				<span class="opacity-60 w-6">{i + 1}.</span>
				<span class={a.correct ? 'text-success' : 'text-error'}>{a.correct ? '✓' : '✗'}</span>
				<span>{a.correct_name}</span>
				{#if !a.correct}<span class="opacity-60">(you said {a.guess})</span>{/if}
			</li>
		{/each}
	</ul>

	<button class="btn btn-primary" onclick={again}>Train again</button>
{/if}
