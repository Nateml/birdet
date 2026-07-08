import { writable } from 'svelte/store';

export type ThemeMode = 'light' | 'dark';

// The main app is themed via the `be-*` design tokens, which are overridden
// under `[data-theme='light']` in app.css. DaisyUI (legacy fallback screens)
// also has `light`/`dark` themes, so the one attribute drives both.
export const theme = writable<ThemeMode>('dark');

function apply(mode: ThemeMode) {
    document.documentElement.setAttribute('data-theme', mode);
    document.documentElement.style.colorScheme = mode;
}

export function initTheme() {
    if (typeof window === 'undefined') return;
    const mode: ThemeMode = localStorage.getItem('theme') === 'light' ? 'light' : 'dark';
    apply(mode);
    theme.set(mode);
}

export function setTheme(mode: ThemeMode) {
    apply(mode);
    localStorage.setItem('theme', mode);
    theme.set(mode);
}

export function toggleTheme() {
    let next: ThemeMode = 'dark';
    theme.update((m) => (next = m === 'dark' ? 'light' : 'dark'));
    setTheme(next);
}
