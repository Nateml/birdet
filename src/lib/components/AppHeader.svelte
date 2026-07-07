<script lang="ts">
	import { page } from '$app/state';

	// Shared BirdEar header for the secondary screens (Stats / Library / Settings).
	// The home + in-session screens draw their own headers.
	const path = $derived(page.url.pathname);

	const links = [
		{ href: '/stats', label: 'Stats' },
		{ href: '/library', label: 'Library' },
		{ href: '/guide', label: 'Guide' },
		{ href: '/settings', label: 'Settings' }
	];
</script>

<header class="flex items-center justify-between border-b border-be-border px-8 pt-8 pb-5">
	<a href="/" class="flex items-center gap-3">
		<div
			class="flex h-9 w-9 items-center justify-center rounded-lg border border-be-primary/20 bg-be-primary/10 text-lg"
		>
			🐦
		</div>
		<div>
			<h1 class="font-be-serif text-xl font-bold leading-none">Birdet</h1>
			<p class="font-be-mono mt-0.5 text-xs text-be-muted-fg">birdsong identification trainer</p>
		</div>
	</a>
	<nav class="flex items-center gap-4 text-sm text-be-muted-fg">
		{#each links as l}
			<a
				href={l.href}
				class="transition-colors hover:text-be-fg"
				class:text-be-fg={path === l.href}
				class:font-semibold={path === l.href}
			>
				{l.label}
			</a>
		{/each}
		<a
			href="/"
			class="flex items-center gap-1.5 rounded-lg border border-be-border px-3 py-1.5 transition-colors hover:border-be-primary/40 hover:text-be-fg"
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
			Train
		</a>
	</nav>
</header>
