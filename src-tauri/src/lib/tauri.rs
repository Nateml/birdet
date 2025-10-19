import { invoke } from "@tauri-apps/api/tauri";

export async function health(): Promise<string> {
  return invoke("health");
}
