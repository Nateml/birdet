<!-- src/routes/train/[sessionId]/+layout.svelte -->
<script lang="ts">
	import type { Snippet } from 'svelte';
	let { children, params }: { children: Snippet, params: { sessionID: string } } = $props();

	// derive progress from your session store
	let current = $state(1), total = $state(10);
</script>

<div class="mb-4 flex items-center gap-4">
	<div class="badge">Session {params.sessionID.slice(0, 8)}</div>
	<progress class="progress w-64" max={total} value={current}></progress>
	<span>{current} / {total}</span>
</div>

{@render children()}

