<!-- src/routes/train/+page.svelte -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { startSession, type Mode } from '$lib/stores/session';
	let pack = $state('');
	let length = $state(10);
	let mode: Mode = $state('multiple');

	function start() {
		const id = crypto.randomUUID();
		startSession(id, { pack, length, mode });
		goto(`/train/${id}`); // goes to lobby
	}
</script>

<h1 class="text-2xl font-bold mb-4">Training setup</h1>
<div class="grid gap-4 max-w-md">
	<select class="select select-bordered" bind:value={pack} aria-label="Pack">
		<option disabled value=''>Choose a pack</option>
		<option value="eu-garden">EU Garden Birds</option>
	</select>

	<label class="form-control">
		<span class="label-text">Questions</span>
		<input class="input input-bordered" type="number" min="5" max="50" bind:value={length} />
	</label>

	<label class="form-control">
		<span class="label-text">Mode</span>
		<select class="select select-bordered" bind:value={mode}>
			<option value="multiple">Multiple choice</option>
			<option value="type">Type the name</option>
		</select>
	</label>

	<button class="btn btn-primary" onclick={start} disabled={!pack}>Start</button>
</div>

