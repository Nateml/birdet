import { invoke } from '@tauri-apps/api/core';

// Persisted app settings live in the DB `settings` table (key/value).
export async function getSetting(key: string): Promise<string> {
    return await invoke<string>('get_setting', { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
    await invoke('set_setting', { key, value });
}

export const SETTING_EBIRD_KEY = 'ebird_api_key';
export const SETTING_XC_KEY = 'xc_api_key';
