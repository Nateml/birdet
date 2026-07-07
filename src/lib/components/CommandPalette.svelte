<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { getPacks, getBirds, type PackDto, type BirdListItem } from '$lib/api/packs';

	type Cmd = { id: string; label: string; sub?: string; group: string; run: () => void };

	let open = $state(false);
	let query = $state('');
	let highlighted = $state(0);
	let inputEl = $state<HTMLInputElement | null>(null);
	let listEl = $state<HTMLElement | null>(null);
	let packs = $state<PackDto[]>([]);
	let birds = $state<BirdListItem[]>([]);

	function go(path: string) {
		open = false;
		goto(path);
	}

	const screens: Cmd[] = [
		{ id: 'nav-train', label: 'Train', group: 'Go to', run: () => go('/') },
		{ id: 'nav-library', label: 'Library', group: 'Go to', run: () => go('/library') },
		{ id: 'nav-stats', label: 'Stats', group: 'Go to', run: () => go('/stats') },
		{ id: 'nav-guide', label: 'Guide', group: 'Go to', run: () => go('/guide') },
		{ id: 'nav-settings', label: 'Settings', group: 'Go to', run: () => go('/settings') },
		{ id: 'nav-import', label: 'Import birds', group: 'Go to', run: () => go('/import') }
	];

	const items = $derived.by(() => {
		const list: Cmd[] = [...screens];
		for (const p of packs)
			list.push({ id: 'pack-' + p.id, label: p.name, sub: `${p.bird_count} species`, group: 'Packs', run: () => go(`/library?pack=${p.id}`) });
		for (const b of birds)
			list.push({ id: 'bird-' + b.id, label: b.common_name, sub: b.scientific_name, group: 'Birds', run: () => go(`/library?bird=${b.id}`) });
		const q = query.trim().toLowerCase();
		const f = q ? list.filter((c) => `${c.label} ${c.sub ?? ''}`.toLowerCase().includes(q)) : list;
		return f.slice(0, 60);
	});

	// Keep the highlight in range as the list shrinks.
	$effect(() => {
		if (highlighted >= items.length) highlighted = Math.max(0, items.length - 1);
	});

	async function openPalette() {
		open = true;
		query = '';
		highlighted = 0;
		try {
			[packs, birds] = await Promise.all([getPacks(), getBirds()]);
		} catch {
			// palette still works for navigation without data
		}
		await tick();
		inputEl?.focus();
	}

	async function scrollHi() {
		await tick();
		listEl?.querySelector('[data-hi]')?.scrollIntoView({ block: 'nearest' });
	}

	onMount(() => {
		const onKey = (e: KeyboardEvent) => {
			if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
				e.preventDefault();
				if (open) open = false;
				else void openPalette();
				return;
			}
			if (!open) return;
			if (e.key === 'Escape') {
				e.preventDefault();
				open = false;
			} else if (e.key === 'ArrowDown') {
				e.preventDefault();
				highlighted = Math.min(items.length - 1, highlighted + 1);
				void scrollHi();
			} else if (e.key === 'ArrowUp') {
				e.preventDefault();
				highlighted = Math.max(0, highlighted - 1);
				void scrollHi();
			} else if (e.key === 'Enter') {
				e.preventDefault();
				items[highlighted]?.run();
			}
		};
		const onOpen = () => {
			if (!open) void openPalette();
		};
		window.addEventListener('keydown', onKey);
		window.addEventListener('open-command-palette', onOpen);
		return () => {
			window.removeEventListener('keydown', onKey);
			window.removeEventListener('open-command-palette', onOpen);
		};
	});
</script>

{#if open}
	<!-- backdrop -->
	<div
		class="fixed inset-0 z-[60] flex items-start justify-center bg-black/50 px-4 pt-[12vh]"
		onclick={() => (open = false)}
		role="presentation"
	>
		<!-- panel -->
		<div
			class="w-full max-w-lg overflow-hidden rounded-xl border border-be-border bg-be-card shadow-2xl"
			onclick={(e) => e.stopPropagation()}
			role="presentation"
		>
			<div class="flex items-center gap-2 border-b border-be-border px-4 py-3">
				<svg class="h-4 w-4 shrink-0 text-be-muted-fg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
				<input
					bind:this={inputEl}
					bind:value={query}
					oninput={() => (highlighted = 0)}
					placeholder="Search birds, packs, screens…"
					class="w-full bg-transparent text-sm text-be-fg outline-none placeholder:text-be-muted-fg"
				/>
				<kbd class="font-be-mono rounded bg-be-secondary px-1.5 py-0.5 text-[10px] text-be-muted-fg">esc</kbd>
			</div>

			<div bind:this={listEl} class="max-h-80 overflow-y-auto p-1.5">
				{#if items.length === 0}
					<p class="px-3 py-6 text-center text-sm text-be-muted-fg">No matches.</p>
				{:else}
					{#each items as c, i (c.id)}
						{#if i === 0 || items[i - 1].group !== c.group}
							<p class="font-be-mono px-3 pb-1 pt-2.5 text-[10px] uppercase tracking-widest text-be-muted-fg">{c.group}</p>
						{/if}
						<button
							onclick={c.run}
							onmouseenter={() => (highlighted = i)}
							data-hi={highlighted === i ? '' : undefined}
							class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition-colors {highlighted === i ? 'bg-be-secondary' : ''}"
						>
							<span class="min-w-0 flex-1">
								<span class="block truncate text-sm font-medium">{c.label}</span>
								{#if c.sub}<span class="block truncate text-xs italic text-be-muted-fg">{c.sub}</span>{/if}
							</span>
						</button>
					{/each}
				{/if}
			</div>
		</div>
	</div>
{/if}
