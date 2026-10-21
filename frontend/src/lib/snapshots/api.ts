import { prepare_image } from "$lib/api";
import { mutate } from "$lib/dashboard/state.svelte";

export async function send_snapshot(
  original: File,
  caption: string,
  display_name: string,
) {
  const image = await prepare_image(original);
  const body = new FormData();
  body.append("image", image);
  body.append("caption", caption);
  body.append("display_name", display_name);
  return mutate("/api/snapshots", { method: "POST", body });
}

const key = "snapshot_username";

export function saved_username() {
  try {
    return localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

export function save_username(name: string) {
  try {
    localStorage.setItem(key, name);
  } catch {}
}

const sent_key = "snapshot_sent_at";

export function saved_sent_at() {
  try {
    return Number(localStorage.getItem(sent_key)) || 0;
  } catch {
    return 0;
  }
}

export function save_sent_at(time: number) {
  try {
    localStorage.setItem(sent_key, String(time));
  } catch {}
}
