<script lang="ts">
    // Replace later with Rust backend integration
    const official = [
        {
            id: 'na_birds',
            name: 'North American Birds',
            description: 'The 100 most common birds from North America.',
            birds: 100
        },
        {
            id: 'eu_birds',
            name: 'European Birds',
            description: 'The 100 most common birds from Europe.',
            birds: 100
        },
    ];

    const custom = [
        {
            id: 'my_favorite_birds',
            name: 'My Favorite Birds',
            description: 'A personal collection of birds I would like to be able to identify.',
            birds: 24,
        },
    ];
</script>

<h1 class="text-2xl font-bold mb-4">Bird Packs</h1>
<h2 class="text-xl font-semibold mb-2">Official</h2>
<ul class="grid gap-2">{#each official as p}
    <li>
        <a href={`/packs/${p.id}`} class="card border p-6">
            <h3 class="text-lg font-bold">{p.name}</h3>
            <p class="text-sm text-gray-600">{p.description}</p>
            <p class="text-sm text-gray-500">{p.birds} birds</p>
        </a>
    </li>
{/each}</ul>

<div class="flex w-full flex-col lg:flex-row items-start lg:items-center justify-between mt-6 mb-2">
    <h2 class="text-xl font-semibold">Custom</h2>
    <a href="/packs/create" class="btn btn-sm btn-neutral mt-2 lg:mt-0">Create New Pack</a>
</div>

<ul class="grid gap-2">{#each custom as p}
    <li>
        <a href={`/packs/${p.id}`} class="card bg-info text-info-content border p-6">
            <h3 class="text-lg font-bold">{p.name}</h3>
            <p class="text-sm">{p.description}</p>
            <p class="text-xs">{p.birds} birds</p>
        </a>
    </li>
{/each}</ul>

