<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
    import { initTheme } from '$lib/stores/theme';
    import { onMount } from 'svelte';
    import { page } from '$app/state';
    import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
	
	let { children } = $props();

    onMount(initTheme);

    // F11 toggles OS fullscreen (hides the window title bar); Esc leaves it.
    // Tauri-only — guarded so `pnpm dev` in a plain browser is a no-op.
    async function toggleFullscreen(force?: boolean) {
        try {
            const { getCurrentWindow } = await import('@tauri-apps/api/window');
            const w = getCurrentWindow();
            const fs = force ?? !(await w.isFullscreen());
            await w.setFullscreen(fs);
        } catch {
            // not running under Tauri
        }
    }

    onMount(() => {
        const onKey = (e: KeyboardEvent) => {
            if (e.key === 'F11') {
                e.preventDefault();
                toggleFullscreen();
            } else if (e.key === 'Escape') {
                toggleFullscreen(false);
            }
        };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    });

    const path = $derived(page.url.pathname);
    const onHome = $derived(path === '/');
    const onTrain = $derived(path === '/train');
    const onLibrary = $derived(path === '/library');
    const onStats = $derived(path === '/stats');
    const onSettings = $derived(path === '/settings');

    // BirdEar design screens render full-bleed (own header, no app navbar):
    // the home pack-picker, the in-session training screens, and the
    // BirdEar-styled secondary screens (Stats / Library / Settings).
    const beScreens = ['/stats', '/library', '/settings', '/import', '/guide'];
    const inSession = $derived(
        path === '/' ||
        (path.startsWith('/train/') && path !== '/train') ||
        path.startsWith('/packs/') ||
        path.startsWith('/birds/') ||
        beScreens.includes(path)
    );
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{#if inSession}
    <div class="min-h-dvh bg-be-bg text-be-fg font-be-sans">
        {@render children()}
    </div>
{:else}
    <div class="flex flex-col min-h-dvh">
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
    </div>
{/if}


