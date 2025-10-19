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
