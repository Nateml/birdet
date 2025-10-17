import { writable } from 'svelte/store';

export const theme = writable('lemonade');

export function initTheme() {
    if (typeof window === 'undefined') return;
    const saved = localStorage.getItem('theme');
    if (saved) {
        document.documentElement.setAttribute('data-theme', saved);
        theme.set(saved);
    }
}

export function setTheme(newTheme: string) {
    document.documentElement.setAttribute('data-theme', newTheme);
    localStorage.setItem('theme', newTheme);
    theme.set(newTheme);
}
