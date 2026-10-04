import { mutate } from "$lib/dashboard/state.svelte";

export function send_snapshot(
  image: File,
  caption: string,
  display_name: string,
) {
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
