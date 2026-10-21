import { get_json, send_json } from "$lib/api";
import { mutate } from "$lib/dashboard/state.svelte";
import type { Layout, Tree } from "$lib/tree/types";

export const fetch_tree = () => get_json<Tree>("/dashboard/tree.json");

export const fetch_layout = () => get_json<Layout>("/dashboard/layout.json");

export const unlock_item = (id: number) =>
  mutate("/api/unlock-item", send_json("POST", { id }));

export const purchase_item = (id: number) =>
  mutate("/api/purchase-item", send_json("POST", { id }));

export const skip_sticker = () =>
  mutate("/api/skip-sticker", { method: "POST" });

const messages: Record<string, string> = {
  "already unlocked": "You've already unlocked this",
  "unknown node": "This item doesn't exist anymore, try reloading the page",
  "unknown item": "This item doesn't exist anymore, try reloading the page",
  "item not unlocked": "Unlock this item before redeeming it",
  "Item already redeemed": "You've already redeemed this item",
  "Sticker already skipped": "You've already skipped the sticker",
  "No Tokens available": "You don't have any Tokens to spend",
};

export const friendly = (error: string) => messages[error] ?? error;
