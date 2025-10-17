<!-- src/routes/train/+page.svelte -->
<script lang="ts">
	import { goto } from '$app/navigation';
	let pack = $state('');
	let length = $state(10);
	let mode: 'multiple' | 'type' = $state('multiple');

	function start() {
		// create a session id and stash config in a store/DB
		const id = crypto.randomUUID();
		// …init session data here…
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

	<button class="btn btn-primary" on:click={start} disabled={!pack}>Start</button>
</div>

