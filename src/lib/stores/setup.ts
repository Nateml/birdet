import { writable } from 'svelte/store';
import type { Mode } from './session';

// Persisted quiz defaults so a session can start in one click without a setup
// step each time. Edited from the Options panel on the home screen.
export type QuizSetup = {
    length: number;
    mode: Mode;
    autoplay: boolean;
    showBackground: boolean; // list background species under the player
};

const KEY = 'birdet:setup';
const DEFAULT: QuizSetup = { length: 10, mode: 'multiple', autoplay: true, showBackground: false };

function load(): QuizSetup {
    if (typeof localStorage === 'undefined') return DEFAULT;
    try {
        const raw = localStorage.getItem(KEY);
        if (raw) return { ...DEFAULT, ...JSON.parse(raw) };
    } catch {
        /* ignore malformed storage */
    }
    return DEFAULT;
}

export const setup = writable<QuizSetup>(load());

setup.subscribe((v) => {
    if (typeof localStorage === 'undefined') return;
    try {
        localStorage.setItem(KEY, JSON.stringify(v));
    } catch {
        /* ignore quota/private-mode errors */
    }
});
