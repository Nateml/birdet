<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
    import { initTheme } from '$lib/stores/theme';
    import { onMount } from 'svelte';
    import { page } from '$app/state';
    import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
    import ImportIndicator from '$lib/components/ImportIndicator.svelte';
    import Sidebar from '$lib/components/Sidebar.svelte';
    import CommandPalette from '$lib/components/CommandPalette.svelte';
	
	let { children } = $props();

    onMount(initTheme);

    // Dev-only zoom: WSLg renders the webview small, so bump it while developing.
    // `import.meta.env.DEV` is false in production builds, so this never ships.
    onMount(() => {
        if (import.meta.env.DEV) document.documentElement.style.zoom = '1.3';
    });

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

    // Three chrome modes:
    //   training  — in-session screens render chromeless (own focused header)
    //   app shell — sidebar + scrollable content pane (the BirdEar screens)
    //   fallback  — legacy DaisyUI pages (train index, dashboard, species)
    const isTraining = $derived(path.startsWith('/train/') && path !== '/train');
    const appScreens = ['/', '/library', '/stats', '/settings', '/import', '/guide'];
    const isApp = $derived(
        appScreens.includes(path) || path.startsWith('/packs/') || path.startsWith('/birds/')
    );
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<ImportIndicator />
<CommandPalette />

{#if isTraining}
    <div class="min-h-dvh bg-be-bg text-be-fg font-be-sans">
        {@render children()}
    </div>
{:else if isApp}
    <!-- Desktop app shell: fixed sidebar, only the content pane scrolls. -->
    <div class="flex h-dvh overflow-hidden bg-be-bg text-be-fg font-be-sans">
        <Sidebar />
        <div class="min-w-0 flex-1 overflow-y-auto">
            {@render children()}
        </div>
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


