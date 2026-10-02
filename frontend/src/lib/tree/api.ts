import { json, request } from "$lib/api";
import type { Layout, Tree } from "$lib/tree/types";

export interface Progress {
  unlocked: number[];
  purchased: number[];
}

export const fetch_tree = () =>
  request("/dashboard/tree.json").then((r) => json<Tree>(r));

export const fetch_layout = () =>
  request("/dashboard/layout.json").then((r) => json<Layout>(r));

export const fetch_progress = () =>
  request("/api/progress").then((r) => json<Progress>(r));

export const unlock_item = (id: number) =>
  request("/api/unlock-item", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id }),
  }).then((r) => json<Progress>(r));

export const purchase_item = (id: number) =>
  request("/api/purchase-item", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id }),
  }).then((r) => json<Progress>(r));
