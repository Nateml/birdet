<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { session } from '$lib/stores/session';

	let { children }: { children: Snippet } = $props();

	const id = page.params.sessionID;
	const total = $derived($session?.config.length ?? 0);
	const current = $derived(Math.min(($session?.index ?? 0) + 1, total));
</script>

<div class="mb-4 flex items-center gap-4">
	<div class="badge">Session {id?.slice(0, 8)}</div>
	<progress class="progress w-64" max={total} value={current}></progress>
	<span>{current} / {total}</span>
</div>

{@render children()}
