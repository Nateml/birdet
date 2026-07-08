<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/logo.svg';
    import { initTheme } from '$lib/stores/theme';
    import { onMount } from 'svelte';
    import { page } from '$app/state';
    import ImportIndicator from '$lib/components/ImportIndicator.svelte';
    import Sidebar from '$lib/components/Sidebar.svelte';
    import CommandPalette from '$lib/components/CommandPalette.svelte';
	
	let { children } = $props();

    onMount(initTheme);

    // Dev-only zoom: WSLg renders the webview small, so bump it while developing.
    // `import.meta.env.DEV` is false in production builds, so this never ships.
    // The zoom makes the h-dvh shell overflow the viewport, so also re-enable
    // root scrolling in dev (production keeps the root locked, no scrollbar).
    onMount(() => {
        if (import.meta.env.DEV) {
            document.documentElement.style.zoom = '1.3';
            document.body.style.overflow = 'auto';
        }
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

    // Three chrome modes:
    //   training  — in-session screens render chromeless (own focused header)
    //   app shell — sidebar + scrollable content pane (the BirdEar screens)
    //   fallback  — chromeless themed shell (e.g. the /train redirect stub)
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
    <div class="h-dvh overflow-y-auto bg-be-bg text-be-fg font-be-sans">
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
    <div class="h-dvh overflow-y-auto bg-be-bg text-be-fg font-be-sans">
        {@render children()}
    </div>
{/if}


