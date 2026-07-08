<script lang="ts">
	import { page } from '$app/state';
	import BirdDetail from '$lib/components/BirdDetail.svelte';

	// `?rec=<id>` deep-links a specific recording (e.g. from a quiz) so it scrolls
	// into view and flashes — handy for jumping straight to a bad recording.
	// `?return=<path>` (set by the quiz) shows a "back to training" link.
	const rec = $derived(page.url.searchParams.get('rec'));
	// Only honour an in-app path (leading `/`, not `//`) — never an external URL.
	const rawBack = $derived(page.url.searchParams.get('return'));
	const back = $derived(
		rawBack && rawBack.startsWith('/') && !rawBack.startsWith('//') ? rawBack : null
	);
</script>

<BirdDetail
	birdId={Number(page.params.birdId!)}
	standalone
	highlightRecordingId={rec ? Number(rec) : undefined}
	backHref={back ?? undefined}
/>
