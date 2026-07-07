import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust `commands::BirdStat`
export type BirdStat = {
    common_name: string;
    seen: number;
    correct: number;
    state: string; // new | learning | review | mastered
    interval_days: number;
    ease: number;
    lapses: number;
    due_at: string | null;
};

// Mirrors Rust `commands::Stats`
export type Stats = {
    total_seen: number;
    total_correct: number;
    total_birds: number;
    new_count: number;
    learning_count: number;
    review_count: number;
    mastered_count: number;
    due_count: number;
    birds: BirdStat[];
};

export async function getStats(): Promise<Stats> {
    return await invoke<Stats>('get_stats');
}
