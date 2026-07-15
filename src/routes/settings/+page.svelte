<script lang="ts">
	import { onMount } from 'svelte';
	import { setup } from '$lib/stores/setup';
	import { getSetting, setSetting, SETTING_EBIRD_KEY, SETTING_XC_KEY } from '$lib/api/settings';
	import { backfillRecordingMeta, repairRecordings } from '$lib/api/recordings';
	import { updateState, checkForUpdate, installUpdate } from '$lib/stores/updater';
	import logo from '$lib/assets/logo.svg';

	function setLength(v: number) {
		const length = Math.max(5, Math.min(50, Math.round(v) || 10));
		setup.update((s) => ({ ...s, length }));
	}

	// API keys (persisted server-side in the DB settings table).
	let ebirdKey = $state('');
	let xcKey = $state('');
	let savedKey = $state<string | null>(null); // which key just saved (for the ✓)

	onMount(async () => {
		[ebirdKey, xcKey] = await Promise.all([
			getSetting(SETTING_EBIRD_KEY),
			getSetting(SETTING_XC_KEY)
		]);
	});

	async function saveKey(key: string, value: string) {
		await setSetting(key, value.trim());
		savedKey = key;
		setTimeout(() => (savedKey === key ? (savedKey = null) : null), 1500);
	}

	// Updates.
	const u = $derived($updateState);
	const checking = $derived(u.status === 'checking');
	async function manualCheck() {
		if (checking) return;
		await checkForUpdate({ silent: false });
	}

	// Dev tools (collapsed by default).
	let devOpen = $state(false);

	// Back-fill quality/type on recordings imported before those fields existed.
	let backfilling = $state(false);
	let backfillMsg = $state<string | null>(null);
	async function runBackfill() {
		if (backfilling) return;
		backfilling = true;
		backfillMsg = null;
		try {
			const n = await backfillRecordingMeta();
			backfillMsg = n > 0 ? `Updated ${n} recording${n > 1 ? 's' : ''}.` : 'All recordings already up to date.';
		} catch (e) {
			backfillMsg = (e as string) ?? 'Backfill failed.';
		} finally {
			backfilling = false;
		}
	}

	// Re-download recordings whose local audio file is missing or corrupt.
	let repairing = $state(false);
	let repairMsg = $state<string | null>(null);
	async function runRepair() {
		if (repairing) return;
		repairing = true;
		repairMsg = null;
		try {
			const r = await repairRecordings();
			repairMsg =
				r.repaired === 0 && r.failed === 0
					? `Checked ${r.checked} — all recordings healthy.`
					: `Checked ${r.checked}, repaired ${r.repaired}` +
						(r.failed > 0 ? `, ${r.failed} still broken.` : '.');
		} catch (e) {
			repairMsg = (e as string) ?? 'Repair failed.';
		} finally {
			repairing = false;
		}
	}
</script>


<main class="mx-auto max-w-2xl px-8 py-10">
	<div class="mb-8">
		<h2 class="font-be-serif mb-2 text-3xl font-bold leading-tight">Settings</h2>
		<p class="text-sm text-be-muted-fg">Defaults applied to every new session.</p>
	</div>

	<!-- Session defaults -->
	<div class="mb-6 rounded-xl border border-be-border bg-be-card p-6">
		<p class="font-be-mono mb-4 text-xs uppercase tracking-widest text-be-muted-fg">
			Session defaults
		</p>

		<label class="flex items-center justify-between gap-4 py-2">
			<span>
				<span class="block text-sm font-medium">New birds per session</span>
				<span class="block text-xs text-be-muted-fg">
					New species introduced before the session switches to reviews. 5–50.
				</span>
			</span>
			<input
				type="number"
				min="5"
				max="50"
				value={$setup.length}
				oninput={(e) => setLength(+e.currentTarget.value)}
				class="w-20 rounded-lg border border-be-border bg-be-bg px-3 py-1.5 text-right text-sm text-be-fg outline-none focus:border-be-primary/40"
			/>
		</label>

		<label class="mt-4 flex items-center justify-between gap-4 border-t border-be-border pt-4">
			<span>
				<span class="block text-sm font-medium">Auto-play call</span>
				<span class="block text-xs text-be-muted-fg">Play each call automatically when it loads.</span>
			</span>
			<button
				type="button"
				role="switch"
				aria-label="Auto-play call"
				aria-checked={$setup.autoplay}
				onclick={() => setup.update((s) => ({ ...s, autoplay: !s.autoplay }))}
				class="relative h-6 w-11 shrink-0 rounded-full transition-colors {$setup.autoplay
					? 'bg-be-primary'
					: 'bg-be-muted'}"
			>
				<span
					class="absolute top-0.5 h-5 w-5 rounded-full bg-white transition-all {$setup.autoplay
						? 'left-[22px]'
						: 'left-0.5'}"
				></span>
			</button>
		</label>

		<label class="mt-4 flex items-center justify-between gap-4 border-t border-be-border pt-4">
			<span>
				<span class="block text-sm font-medium">Show background birds</span>
				<span class="block text-xs text-be-muted-fg">List other species heard in the clip under the player. They're never offered as wrong options.</span>
			</span>
			<button
				type="button"
				role="switch"
				aria-label="Show background birds"
				aria-checked={$setup.showBackground}
				onclick={() => setup.update((s) => ({ ...s, showBackground: !s.showBackground }))}
				class="relative h-6 w-11 shrink-0 rounded-full transition-colors {$setup.showBackground
					? 'bg-be-primary'
					: 'bg-be-muted'}"
			>
				<span
					class="absolute top-0.5 h-5 w-5 rounded-full bg-white transition-all {$setup.showBackground
						? 'left-[22px]'
						: 'left-0.5'}"
				></span>
			</button>
		</label>

		<div class="mt-4 border-t border-be-border pt-4">
			<span class="mb-2 block text-sm font-medium">Answer mode</span>
			<div class="grid grid-cols-2 gap-2">
				<button
					onclick={() => setup.update((s) => ({ ...s, mode: 'multiple' }))}
					class="rounded-lg border px-4 py-2.5 text-sm font-semibold transition-colors"
					class:border-be-primary={$setup.mode === 'multiple'}
					class:bg-be-secondary={$setup.mode === 'multiple'}
					class:text-be-primary={$setup.mode === 'multiple'}
					class:border-be-border={$setup.mode !== 'multiple'}
				>
					Multiple choice
				</button>
				<button
					disabled
					class="flex items-center justify-center gap-2 rounded-lg border border-be-border px-4 py-2.5 text-sm font-semibold text-be-muted-fg opacity-60"
					title="Coming soon"
				>
					Type the name
					<span class="font-be-mono rounded bg-be-muted px-1.5 py-0.5 text-[10px] uppercase tracking-wider">soon</span>
				</button>
			</div>
		</div>
	</div>

	<!-- API keys (for importing birds) -->
	<div class="mb-6 rounded-xl border border-be-border bg-be-card p-6">
		<p class="font-be-mono mb-1 text-xs uppercase tracking-widest text-be-muted-fg">API keys</p>
		<p class="mb-4 text-xs text-be-muted-fg">
			Needed to import birds. Both are free — grab a token from your account on each site.
		</p>

		<label class="block">
			<span class="mb-1.5 flex items-center gap-2 text-sm font-medium">
				eBird API token
				{#if savedKey === SETTING_EBIRD_KEY}<span class="text-xs text-be-primary">saved ✓</span>{/if}
			</span>
			<input
				type="password"
				bind:value={ebirdKey}
				onblur={() => saveKey(SETTING_EBIRD_KEY, ebirdKey)}
				placeholder="from ebird.org/api/keygen"
				autocomplete="off"
				class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40"
			/>
		</label>

		<label class="mt-4 block">
			<span class="mb-1.5 flex items-center gap-2 text-sm font-medium">
				Xeno-Canto API key
				{#if savedKey === SETTING_XC_KEY}<span class="text-xs text-be-primary">saved ✓</span>{/if}
			</span>
			<input
				type="password"
				bind:value={xcKey}
				onblur={() => saveKey(SETTING_XC_KEY, xcKey)}
				placeholder="from xeno-canto.org account (API v3)"
				autocomplete="off"
				class="w-full rounded-lg border border-be-border bg-be-bg px-3 py-2 text-sm text-be-fg outline-none focus:border-be-primary/40"
			/>
		</label>
	</div>

	<!-- Dev tools (collapsed) -->
	<div class="mb-6 rounded-xl border border-be-border bg-be-card p-6">
		<button
			type="button"
			onclick={() => (devOpen = !devOpen)}
			class="flex w-full items-center justify-between"
			aria-expanded={devOpen}
		>
			<span class="font-be-mono text-xs uppercase tracking-widest text-be-muted-fg">Dev tools</span>
			<svg
				width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
				stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
				class="text-be-muted-fg transition-transform {devOpen ? 'rotate-180' : ''}"
			><path d="m6 9 6 6 6-6"/></svg>
		</button>

		{#if devOpen}
			<div class="mt-4 space-y-4">
				<!-- Repair recordings -->
				<div class="flex items-center justify-between gap-4">
					<span>
						<span class="block text-sm font-medium">Repair recordings</span>
						<span class="block text-xs text-be-muted-fg">
							Re-download any recordings whose audio file is missing or corrupt.
						</span>
					</span>
					<button
						onclick={runRepair}
						disabled={repairing}
						class="shrink-0 rounded-lg border border-be-border px-3.5 py-2 text-sm transition-colors hover:bg-be-secondary disabled:opacity-50"
					>
						{repairing ? 'Repairing…' : 'Repair'}
					</button>
				</div>
				{#if repairMsg}
					<p class="text-xs text-be-muted-fg">{repairMsg}</p>
				{/if}

				<!-- Back-fill metadata -->
				<div class="flex items-center justify-between gap-4 border-t border-be-border pt-4">
					<span>
						<span class="block text-sm font-medium">Back-fill recording metadata</span>
						<span class="block text-xs text-be-muted-fg">
							Fetch quality &amp; type from Xeno-Canto for older recordings missing them.
						</span>
					</span>
					<button
						onclick={runBackfill}
						disabled={backfilling}
						class="shrink-0 rounded-lg border border-be-border px-3.5 py-2 text-sm transition-colors hover:bg-be-secondary disabled:opacity-50"
					>
						{backfilling ? 'Fetching…' : 'Back-fill'}
					</button>
				</div>
				{#if backfillMsg}
					<p class="text-xs text-be-muted-fg">{backfillMsg}</p>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Updates -->
	<div class="mb-6 rounded-xl border border-be-border bg-be-card p-6">
		<p class="font-be-mono mb-4 text-xs uppercase tracking-widest text-be-muted-fg">Updates</p>
		<div class="flex items-center justify-between gap-4">
			<span>
				<span class="block text-sm font-medium">App updates</span>
				<span class="block text-xs text-be-muted-fg">
					{#if u.status === 'checking'}
						Checking for updates…
					{:else if u.status === 'available'}
						Version {u.version} is available.
					{:else if u.status === 'downloading'}
						Downloading… {Math.round((u.progress ?? 0) * 100)}%
					{:else if u.status === 'ready'}
						Installing — the app will restart.
					{:else if u.status === 'up-to-date'}
						You're on the latest version{#if u.currentVersion} (v{u.currentVersion}){/if}.
					{:else if u.status === 'error'}
						Couldn't check for updates. Try again later.
					{:else if u.status === 'unsupported'}
						Updates aren't available in this build.
					{:else}
						{#if u.currentVersion}Current version v{u.currentVersion}.{:else}Check GitHub for a newer release.{/if}
					{/if}
				</span>
			</span>
			<button
				onclick={manualCheck}
				disabled={checking || u.status === 'downloading' || u.status === 'ready'}
				class="shrink-0 rounded-lg border border-be-border px-3.5 py-2 text-sm transition-colors hover:bg-be-secondary disabled:opacity-50"
			>
				{checking ? 'Checking…' : 'Check for updates'}
			</button>
		</div>

		{#if u.status === 'available'}
			{#if u.notes}
				<div class="mt-4 max-h-40 overflow-y-auto whitespace-pre-line rounded-lg border border-be-border bg-be-bg px-3 py-2 text-xs text-be-muted-fg">
					{u.notes}
				</div>
			{/if}
			<button
				onclick={installUpdate}
				class="mt-4 w-full rounded-lg bg-be-primary px-4 py-2.5 text-sm font-semibold text-be-primary-fg transition-opacity hover:opacity-90"
			>
				Install v{u.version} &amp; restart
			</button>
		{/if}

		{#if u.status === 'downloading'}
			<div class="mt-4 h-1.5 w-full overflow-hidden rounded-full bg-be-muted">
				<div class="h-full rounded-full bg-be-primary transition-all" style="width: {(u.progress ?? 0) * 100}%"></div>
			</div>
		{/if}
	</div>

	<!-- About -->
	<div class="rounded-xl border border-be-border bg-be-card p-6">
		<p class="font-be-mono mb-3 text-xs uppercase tracking-widest text-be-muted-fg">About</p>
		<div class="flex items-center gap-3">
			<img src={logo} alt="Birdet" class="h-10 w-10" />
			<div>
				<p class="font-be-serif font-semibold">Birdet</p>
				<p class="text-xs text-be-muted-fg">Learn bird calls by ear.</p>
			</div>
			<span class="font-be-mono ml-auto rounded-full bg-be-accent/15 px-2.5 py-1 text-xs text-be-accent">
				alpha build
			</span>
		</div>

		<div class="mt-4 space-y-2 border-t border-be-border pt-4 text-sm">
			<div class="flex items-center gap-2">
				<span class="w-16 shrink-0 text-xs text-be-muted-fg">Made by</span>
				<span>nateml</span>
			</div>
			<div class="flex items-center gap-2">
				<span class="w-16 shrink-0 text-xs text-be-muted-fg">Logo</span>
				<span>Ockert Kok</span>
			</div>
			<div class="flex items-center gap-2">
				<span class="w-16 shrink-0 text-xs text-be-muted-fg">GitHub</span>
				<a
					href="https://github.com/Nateml/birdet"
					target="_blank"
					rel="noopener noreferrer"
					class="text-be-primary hover:underline"
				>
					github.com/Nateml/birdet
				</a>
			</div>
			<div class="flex items-center gap-2">
				<span class="w-16 shrink-0 text-xs text-be-muted-fg">Contact</span>
				<a href="mailto:nate.mac.dev@gmail.com" class="text-be-primary hover:underline">
					nate.mac.dev@gmail.com
				</a>
			</div>
		</div>

		<p class="mt-4 rounded-lg border border-be-accent/30 bg-be-accent/[0.06] px-3 py-2 text-xs leading-relaxed text-be-muted-fg">
			This is an early <strong class="text-be-fg">alpha build</strong> — expect bugs and incomplete/missing features. Feedback and bug reports are very welcome.
		</p>
	</div>

	<!-- Acknowledgements: Birdet is built on community recordings + open data,
	     each of which requires attribution under its terms. -->
	<div class="mt-6 rounded-xl border border-be-border bg-be-card p-6">
		<p class="font-be-mono mb-3 text-xs uppercase tracking-widest text-be-muted-fg">Acknowledgements</p>
		<p class="text-sm text-be-muted-fg">
			Birdet is built on community recordings and open ornithological data. Huge thanks to
			the recordists and organisations that make it possible.
		</p>

		<div class="mt-4 space-y-4 border-t border-be-border pt-4">
			<div>
				<div class="flex items-baseline justify-between gap-3">
					<span class="text-sm font-semibold text-be-fg">Xeno-Canto</span>
					<a
						href="https://xeno-canto.org"
						target="_blank"
						rel="noopener noreferrer"
						class="text-xs text-be-primary hover:underline"
					>
						xeno-canto.org
					</a>
				</div>
				<p class="mt-1 text-xs leading-relaxed text-be-muted-fg">
					All bird sounds come from Xeno-Canto, a collaborative archive of wildlife
					recordings. Each recording is the work of an individual recordist and is shared
					under a
					<a href="https://creativecommons.org/licenses/" target="_blank" rel="noopener noreferrer" class="text-be-primary hover:underline">Creative Commons</a>
					licence. Birdet credits every recording's recordist and licence — on its bird's
					page in the Library, and after each answer while training.
				</p>
			</div>

			<div>
				<div class="flex items-baseline justify-between gap-3">
					<span class="text-sm font-semibold text-be-fg">eBird &amp; the Cornell Lab of Ornithology</span>
					<a
						href="https://ebird.org"
						target="_blank"
						rel="noopener noreferrer"
						class="text-xs text-be-primary hover:underline"
					>
						ebird.org
					</a>
				</div>
				<p class="mt-1 text-xs leading-relaxed text-be-muted-fg">
					Species names, taxonomy, regional checklists and relative-abundance data are from
					<a href="https://ebird.org" target="_blank" rel="noopener noreferrer" class="text-be-primary hover:underline">eBird</a>,
					a project of the
					<a href="https://www.birds.cornell.edu" target="_blank" rel="noopener noreferrer" class="text-be-primary hover:underline">Cornell Lab of Ornithology</a>.
					Taxonomy follows the eBird/Clements Checklist. Birdet is not affiliated with or
					endorsed by eBird or the Cornell Lab.
				</p>
			</div>
		</div>

		<p class="mt-4 text-[11px] leading-relaxed text-be-muted-fg">
			Birdet is an independent, non-commercial learning tool. Recording copyrights remain with
			their respective recordists; see each recording's licence for its terms.
		</p>
	</div>
</main>
