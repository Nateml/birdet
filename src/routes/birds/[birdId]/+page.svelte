<script lang="ts">
	import { page } from '$app/state';
	import BirdDetail from '$lib/components/BirdDetail.svelte';
</script>

<BirdDetail birdId={Number(page.params.birdId!)} standalone />
