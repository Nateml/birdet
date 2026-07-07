<script lang="ts">
	import { onMount } from 'svelte';
	import { listRegions, type RegionItem } from '$lib/api/regions';

	// Cascading Country → Subregion picker. Binds the selected eBird region code
	// (a subnational1 code if chosen, else the country code = whole country).
	let { value = $bindable(''), disabled = false }: { value?: string; disabled?: boolean } =
		$props();

	let countries = $state<RegionItem[]>([]);
	let subs = $state<RegionItem[]>([]);
	let country = $state('');
	let sub = $state('');
	let loadingSubs = $state(false);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			countries = await listRegions('country', 'world');
		} catch (e) {
			error = (e as string) ?? 'Could not load regions (check eBird key).';
		}
	});

	async function onCountry() {
		sub = '';
		subs = [];
		value = country;
		if (!country) {
			value = '';
			return;
		}
		loadingSubs = true;
		error = null;
		try {
			subs = await listRegions('subnational1', country);
		} catch (e) {
			error = (e as string) ?? 'Could not load subregions.';
		} finally {
			loadingSubs = false;
		}
	}

	function onSub() {
		value = sub || country;
	}
</script>

<div class="grid grid-cols-2 gap-3">
	<select
		bind:value={country}
		onchange={onCountry}
		{disabled}
		class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40 disabled:opacity-50"
	>
		<option value="">Country…</option>
		{#each countries as c (c.code)}
			<option value={c.code}>{c.name}</option>
		{/each}
	</select>
	<select
		bind:value={sub}
		onchange={onSub}
		disabled={disabled || !country || loadingSubs}
		class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40 disabled:opacity-50"
	>
		<option value="">{loadingSubs ? 'loading…' : 'Whole country'}</option>
		{#each subs as s (s.code)}
			<option value={s.code}>{s.name}</option>
		{/each}
	</select>
</div>
{#if error}
	<p class="font-be-mono mt-1.5 text-xs text-be-destructive">{error}</p>
{:else if value}
	<p class="font-be-mono mt-1.5 text-xs text-be-muted-fg">region: {value}</p>
{/if}
