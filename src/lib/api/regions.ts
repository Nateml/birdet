import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust `commands::RegionItem`
export type RegionItem = { code: string; name: string };

// level ∈ 'country' | 'subnational1' | 'subnational2'; parent 'world' for countries.
export async function listRegions(level: string, parent: string): Promise<RegionItem[]> {
    return await invoke<RegionItem[]>('list_regions', { level, parent });
}
