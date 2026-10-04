import { get_json, send_json } from "$lib/api";
import { mutate } from "$lib/dashboard/state.svelte";
import type { Layout, Tree } from "$lib/tree/types";

export const fetch_tree = () => get_json<Tree>("/dashboard/tree.json");

export const fetch_layout = () => get_json<Layout>("/dashboard/layout.json");

export const unlock_item = (id: number) =>
  mutate("/api/unlock-item", send_json("POST", { id }));

export const purchase_item = (id: number) =>
  mutate("/api/purchase-item", send_json("POST", { id }));
