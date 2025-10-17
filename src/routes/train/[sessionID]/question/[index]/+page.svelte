<!-- src/routes/train/[sessionId]/question/[index]/+page.svelte -->
<script lang="ts">
	let { params }: { params: { sessionId: string; index: string } } = $props();

	// stub data
	const choices = ['Robin', 'Blackbird', 'Wren', 'Chaffinch'];
	const audioSrc = '/audio/blackbird-01.ogg'; // replace with real path

	function answer(choice: string) {
		// record answer in session store
		// navigate next or to result
		const next = Number(params.index) + 1;
		// if last -> /result, else -> next question
	}
</script>

<h1 class="text-xl font-semibold mb-2">Question {params.index}</h1>

<audio class="w-full mb-4" src={audioSrc} controls preload="auto"></audio>

<div class="grid sm:grid-cols-2 gap-3">
	{#each choices as c}
		<button class="btn btn-outline" onclick={() => answer(c)}>{c}</button>
	{/each}
</div>

<div class="mt-4 flex gap-2">
	<button class="btn btn-ghost">Replay</button>
	<button class="btn btn-ghost">Hint</button>
</div>

