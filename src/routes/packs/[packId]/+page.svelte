<script lang="ts">
	import { page } from '$app/state';
	import PackEditor from '$lib/components/PackEditor.svelte';
</script>

<PackEditor packId={page.params.packId!} standalone />
