<script lang="ts">
	// The pack-picker home screen replaced this setup form; starting a session
	// is now one click from home. Redirect any stale links there.
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	onMount(() => goto('/', { replaceState: true }));
</script>
