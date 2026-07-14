<script lang="ts">
	import { updateState, installUpdate } from '$lib/stores/updater';

	let dismissed = $state(false);
	const s = $derived($updateState);
	const show = $derived(
		!dismissed && (s.status === 'available' || s.status === 'downloading' || s.status === 'ready')
	);
</script>

{#if show}
	<div class="fixed left-1/2 top-3 z-50 flex -translate-x-1/2 items-center gap-3 rounded-full border border-be-border bg-be-card px-4 py-2 text-sm shadow-lg">
		{#if s.status === 'available'}
			<span>
				<span class="font-medium">Update available</span>
				{#if s.version}<span class="text-be-muted-fg"> — v{s.version}</span>{/if}
			</span>
			<button
				onclick={installUpdate}
				class="rounded-full bg-be-primary px-3 py-1 text-xs font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
			>
				Install &amp; restart
			</button>
			<button
				onclick={() => (dismissed = true)}
				class="text-xs text-be-muted-fg hover:text-be-fg"
			>
				Later
			</button>
		{:else if s.status === 'downloading'}
			<span class="font-mono text-xs text-be-muted-fg">
				Downloading update… {Math.round((s.progress ?? 0) * 100)}%
			</span>
			<div class="h-1 w-24 overflow-hidden rounded-full bg-be-muted">
				<div class="h-full rounded-full bg-be-primary transition-all" style="width: {(s.progress ?? 0) * 100}%"></div>
			</div>
		{:else if s.status === 'ready'}
			<span class="font-mono text-xs text-be-muted-fg">Installing… restarting</span>
		{/if}
	</div>
{/if}
