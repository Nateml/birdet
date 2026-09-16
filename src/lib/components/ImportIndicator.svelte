<script lang="ts">
	import { importJob } from '$lib/stores/importJob';
	import { infoJob } from '$lib/stores/infoJob';

	const pct = $derived(
		$importJob.total ? Math.min(100, Math.round(($importJob.current / $importJob.total) * 100)) : 0
	);
	// Species notes run in the backend and outlive the screen that started them,
	// so they get the same treatment as an import: visible from anywhere.
	const infoPct = $derived(
		$infoJob.total ? Math.min(100, Math.round(($infoJob.current / $infoJob.total) * 100)) : 0
	);
</script>

<!-- Both can run at once (an import kicks off its own notes fill), so stack. -->
<div class="pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col gap-2">
{#if $importJob.active}
	<div class="pointer-events-auto w-72 max-w-[calc(100vw-2rem)] rounded-xl border border-be-border bg-be-card p-3.5 shadow-lg">
		<div class="flex items-center gap-2.5">
			<svg class="h-4 w-4 shrink-0 animate-spin text-be-primary" viewBox="0 0 24 24" fill="none">
				<circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity="0.25" stroke-width="3" />
				<path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
			</svg>
			<span class="text-sm font-medium">Importing{#if $importJob.label} {$importJob.label}{/if}…</span>
			{#if $importJob.total}
				<span class="font-be-mono ml-auto text-xs text-be-muted-fg">{$importJob.current}/{$importJob.total}</span>
			{/if}
		</div>
		{#if $importJob.total}
			<div class="mt-2.5 h-1 overflow-hidden rounded-full bg-be-muted">
				<div class="h-full rounded-full bg-be-primary transition-all duration-300" style="width: {pct}%"></div>
			</div>
		{/if}
		{#if $importJob.message}
			<p class="mt-2 truncate text-xs text-be-muted-fg">{$importJob.message}</p>
		{/if}
	</div>
{/if}

{#if $infoJob.active}
	<div class="pointer-events-auto w-72 max-w-[calc(100vw-2rem)] rounded-xl border border-be-border bg-be-card p-3.5 shadow-lg">
		<div class="flex items-center gap-2.5">
			<svg class="h-4 w-4 shrink-0 animate-spin text-be-primary" viewBox="0 0 24 24" fill="none">
				<circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity="0.25" stroke-width="3" />
				<path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
			</svg>
			<span class="text-sm font-medium">{$infoJob.force ? 'Re-fetching' : 'Fetching'} species notes…</span>
			{#if $infoJob.total}
				<span class="font-be-mono ml-auto text-xs text-be-muted-fg">{$infoJob.current}/{$infoJob.total}</span>
			{/if}
		</div>
		{#if $infoJob.total}
			<div class="mt-2.5 h-1 overflow-hidden rounded-full bg-be-muted">
				<div class="h-full rounded-full bg-be-primary transition-all duration-300" style="width: {infoPct}%"></div>
			</div>
		{/if}
		{#if $infoJob.message}
			<p class="mt-2 truncate text-xs text-be-muted-fg">{$infoJob.message}</p>
		{/if}
	</div>
{/if}
</div>
