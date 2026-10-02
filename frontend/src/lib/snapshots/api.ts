import { json, request } from "$lib/api";
import type { Snapshot, SnapshotStatus } from "$lib/snapshots/types";

export const fetch_snapshots = () =>
  request("/api/snapshots").then((r) => json<Snapshot[]>(r));

export const fetch_my_snapshots = () =>
  request("/api/snapshots/mine").then((r) => json<Snapshot[]>(r));

export const fetch_status = () =>
  request("/api/snapshots/status").then((r) => json<SnapshotStatus>(r));

export function send_snapshot(image: File, caption: string) {
  const body = new FormData();
  body.append("image", image);
  body.append("caption", caption);
  return request("/api/snapshots", { method: "POST", body }).then((r) =>
    json<Snapshot>(r),
  );
}
