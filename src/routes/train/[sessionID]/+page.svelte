<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { session } from '$lib/stores/session';

	const id = page.params.sessionID;

	// The lobby was removed from the flow (home starts a session directly). If a
	// session exists, jump into it; otherwise back to the pack picker.
	onMount(() => {
		if ($session && $session.id === id) goto(`/train/${id}/question/0`, { replaceState: true });
		else goto('/', { replaceState: true });
	});
</script>
