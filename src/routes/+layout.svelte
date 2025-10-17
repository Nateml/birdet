<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
    import { initTheme } from '$lib/stores/theme';
    import { onMount } from 'svelte';
    import { page } from '$app/state';
    import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
	
	let { children } = $props();

    onMount(initTheme);

    const path = $derived(page.url.pathname);
    const onHome = $derived(path === '/');
    const onTrain = $derived(path === '/train');
    const onLibrary = $derived(path === '/library');
    const onStats = $derived(path === '/stats');
    const onSettings = $derived(path === '/settings');
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="min-h-dvh">
    <header class="border-b">
        <div class="navbar container mx-auto">
            <div class="navbar-start">
                <a href="/" class="text-lg font-semibold">Birdet</a>
            </div>

            <div class="navbar-center hidden md:flex">
                <ul class="menu menu-horizontal px-1 gap-2">
                    <li><a href="/" class:active={onHome}>Home</a></li>
                    <li><a href="/train" class:active={onTrain}>Train</a></li>
                    <li><a href="/library" class:active={onLibrary}>Library</a></li>
                    <li><a href="/stats" class:active={onStats}>Stats</a></li>
                    <li><a href="/settings" class:active={onSettings}>Settings</a></li>
                </ul>
            </div>

            <div class="navbar-end">
                <ThemeSwitcher value="night" />
            </div>
        </div>
    </header>

    <main class="container mx-auto py-6 px-4">
        {@render children()}
    </main>

    <footer class="border-t">
        <div class="container mx-auto py-4 px-4 text-center text-sm opacity-70">
            &copy; {new Date().getFullYear()} Birdet. All rights reserved.
        </div>
    </footer>
</div>


