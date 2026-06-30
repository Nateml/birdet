import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust `commands::BirdStat`
export type BirdStat = {
    common_name: string;
    seen: number;
    correct: number;
};

// Mirrors Rust `commands::Stats`
export type Stats = {
    total_seen: number;
    total_correct: number;
    birds: BirdStat[];
};

export async function getStats(): Promise<Stats> {
    return await invoke<Stats>('get_stats');
}
