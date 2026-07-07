import { invoke } from '@tauri-apps/api/core';

export type PackDto = {
    id: string;
    name: string;
    description: string | null;
    bird_count: number;
}

export async function getPacks(): Promise<PackDto[]> {
    return await invoke<PackDto[]>('get_packs');
}

// Mirrors Rust `commands::BirdListItem`
export type BirdListItem = {
    id: number;
    common_name: string;
    scientific_name: string;
    family: string | null;
    region: string | null;
    recording_count: number;
};

export async function getBirds(): Promise<BirdListItem[]> {
    return await invoke<BirdListItem[]>('get_birds');
}

// Mirrors Rust `commands::BirdPackTag` — one row per (bird, pack) pair.
export type BirdPackTag = {
    bird_id: number;
    pack_id: string;
    pack_name: string;
};

export async function getBirdPacks(): Promise<BirdPackTag[]> {
    return await invoke<BirdPackTag[]>('get_bird_packs');
}

export async function createPack(name: string, birdIds: number[]): Promise<string> {
    return await invoke<string>('create_pack', { name, birdIds });
}

export async function createPackFromFilter(
    name: string | null,
    region: string | null,
    family: string | null
): Promise<string> {
    return await invoke<string>('create_pack_from_filter', { name, region, family });
}

export async function getPackBirds(packId: string): Promise<BirdListItem[]> {
    return await invoke<BirdListItem[]>('get_pack_birds', { packId });
}

export async function renamePack(packId: string, name: string): Promise<void> {
    await invoke('rename_pack', { packId, name });
}

export async function deletePack(packId: string): Promise<void> {
    await invoke('delete_pack', { packId });
}

export async function addBirdsToPack(packId: string, birdIds: number[]): Promise<void> {
    await invoke('add_birds_to_pack', { packId, birdIds });
}

export async function removeBirdFromPack(packId: string, birdId: number): Promise<void> {
    await invoke('remove_bird_from_pack', { packId, birdId });
}
