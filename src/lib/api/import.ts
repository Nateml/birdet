import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// Mirrors Rust `services::import::ImportParams` (snake_case — nested struct).
export type ImportParams = {
    region: string;
    max_species: number;
    max_per_species: number;
    quality: string | null;
    rec_type: string | null;
    family: string | null;
    create_pack: boolean;
};

export type ImportProgress = {
    stage: 'fetching' | 'species' | 'downloading' | 'done' | 'error';
    message: string;
    current: number;
    total: number;
};

export type ImportSummary = {
    species_imported: number;
    species_skipped: number;
    recordings_added: number;
    pack_id: string | null;
};

export async function importBirds(params: ImportParams): Promise<ImportSummary> {
    return await invoke<ImportSummary>('import_birds', { params });
}

// Mirrors Rust `services::taxonomy::SpeciesResult`.
export type SpeciesResult = {
    ebird_code: string;
    common_name: string;
    scientific_name: string;
    family: string | null;
    in_library: boolean;
};

// Search the cached eBird taxonomy by name (manual add-birds flow).
export async function searchSpecies(query: string): Promise<SpeciesResult[]> {
    return await invoke<SpeciesResult[]>('search_species', { query });
}

// Import specific species by eBird code. Emits the same import://progress events.
export async function importSpecies(opts: {
    ebirdCodes: string[];
    quality?: string | null;
    recType?: string | null;
    maxPerSpecies?: number;
    packIds?: string[];
    newPackName?: string | null;
}): Promise<ImportSummary> {
    return await invoke<ImportSummary>('import_species', {
        ebirdCodes: opts.ebirdCodes,
        quality: opts.quality ?? null,
        recType: opts.recType ?? null,
        maxPerSpecies: opts.maxPerSpecies ?? 1,
        packIds: opts.packIds ?? [],
        newPackName: opts.newPackName ?? null
    });
}

export function onImportProgress(cb: (p: ImportProgress) => void): Promise<UnlistenFn> {
    return listen<ImportProgress>('import://progress', (e) => cb(e.payload));
}
