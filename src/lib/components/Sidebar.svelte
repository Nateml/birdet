<script lang="ts">
	import { page } from '$app/state';

	const path = $derived(page.url.pathname);

	type Item = { href: string; label: string; icon: string; match: (p: string) => boolean };
	// Minimal inline icon paths (stroked, 24x24).
	const items: Item[] = [
		{
			href: '/',
			label: 'Train',
			match: (p) => p === '/',
			icon: 'M8 5v14l11-7z'
		},
		{
			href: '/library',
			label: 'Library',
			match: (p) => p === '/library' || p.startsWith('/packs/') || p.startsWith('/birds/'),
			icon: 'M4 19.5A2.5 2.5 0 0 1 6.5 17H20M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z'
		},
		{
			href: '/stats',
			label: 'Stats',
			match: (p) => p === '/stats',
			icon: 'M3 3v18h18M18 17V9M13 17V5M8 17v-3'
		},
		{
			href: '/guide',
			label: 'Guide',
			match: (p) => p === '/guide',
			icon: 'M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2zM22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z'
		},
		{
			href: '/settings',
			label: 'Settings',
			match: (p) => p === '/settings',
			icon: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z'
		}
	];
</script>

<aside class="flex h-dvh w-56 shrink-0 flex-col border-r border-be-border bg-be-card">
	<!-- brand -->
	<a href="/" class="flex items-center gap-3 px-5 pt-6 pb-5">
		<div class="flex h-9 w-9 items-center justify-center rounded-lg border border-be-primary/20 bg-be-primary/10 text-lg">
			🐦
		</div>
		<div>
			<h1 class="font-be-serif text-lg font-bold leading-none">Birdet</h1>
			<p class="font-be-mono mt-0.5 text-[10px] text-be-muted-fg">by ear</p>
		</div>
	</a>

	<!-- nav -->
	<nav class="flex flex-1 flex-col gap-1 px-3 py-2">
		{#each items as it (it.href)}
			{@const active = it.match(path)}
			<a
				href={it.href}
				class="flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors {active
					? 'bg-be-secondary font-semibold text-be-fg'
					: 'text-be-muted-fg hover:bg-be-secondary/50 hover:text-be-fg'}"
			>
				<svg class="h-[18px] w-[18px] shrink-0" viewBox="0 0 24 24" fill={it.href === '/' ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<path d={it.icon} />
				</svg>
				{it.label}
			</a>
		{/each}
	</nav>

	<p class="font-be-mono px-5 py-4 text-[10px] text-be-muted-fg">alpha</p>
</aside>
