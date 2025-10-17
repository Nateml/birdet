<script lang="ts">
	let { params }: { params: { packID: string } } = $props();
	// fetch pack metadata & species list here
</script>

<h1 class="text-2xl font-bold mb-2">Pack: {params.packID}</h1>
<p class="opacity-70">Description: description</p>
<p class="opacity-70">Bird count: 42</p>
<p class="opacity-70">Profficiency: 32%</p>
<a class="btn btn-primary mt-4" href={`/train?pack=${params.packID}`}>Start training</a>

