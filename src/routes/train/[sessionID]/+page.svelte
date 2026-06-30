<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { session } from '$lib/stores/session';

	const id = page.params.sessionID;

	// Session lives in memory; a reload or direct nav loses it -> back to setup.
	onMount(() => {
		if (!$session || $session.id !== id) goto('/train');
	});

	function begin() {
		goto(`/train/${id}/question/0`);
	}
</script>

{#if $session && $session.id === id}
	<h1 class="text-2xl font-bold mb-4">Ready to train</h1>
	<ul class="mb-6 grid gap-1 opacity-80">
		<li>Pack: {$session.config.pack || 'All birds'}</li>
		<li>Questions: {$session.config.length}</li>
		<li>Mode: {$session.config.mode}</li>
	</ul>
	<button class="btn btn-primary" onclick={begin}>Begin</button>
{/if}
