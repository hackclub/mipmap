import { json, request } from "$lib/api";
import { Load } from "$lib/load.svelte";
import type { HackatimeProject, Project } from "$lib/projects/types";
import type { Snapshot, SnapshotStatus } from "$lib/snapshots/types";

export interface Order {
  id: number;
  node_id: number;
  item_name: string;
  ordered_at: string;
  fulfilled: boolean;
  tracking_number: string | null;
}

export interface User {
  first_name: string | null;
  mips: number;
  referral_code: string;
  blocked: string | null;
  hackatime_linked: boolean;
  hackatime_revoked: boolean;
  slack_linked: boolean;
  onboarding_step: string | null;
  onboarding_granted: boolean;
  onboarded: boolean;
  saw_old_onboarding: boolean;
  sticker_skipped: boolean;
}

export interface Dashboard {
  user: User;
  unlocked: number[];
  purchased: number[];
  orders: Order[];
  projects: Project[];
  hackatime_projects: HackatimeProject[];
  snapshots: {
    everyone: Snapshot[];
    mine: Snapshot[];
    status: SnapshotStatus;
  };
}

export const account = $state({ suspended: false });

const suspended = "Your account has been suspended";

async function parse<T = Dashboard>(response: Response) {
  if (response.status === 403) {
    const body = await response.clone().json().catch(() => ({}));
    if (body.error === suspended) account.suspended = true;
  }
  return json<T>(response);
}

export const dashboard = new Load(() =>
  request("/api/dashboard").then((r) => parse(r)),
);

let pending = 0;
let overlapped = false;

export async function mutate(input: string, init: RequestInit) {
  if (pending++ > 0) overlapped = true;
  let data: Dashboard | null | undefined;
  try {
    data = await request(input, init).then((r) =>
      parse<Dashboard | null>(r),
    );
  } finally {
    if (--pending === 0) {
      const stale = overlapped || data === null;
      overlapped = false;
      if (stale) await dashboard.run();
      else if (data) dashboard.set(data);
    }
  }
}

let refreshing: Promise<void> | undefined;

export function refresh() {
  if (pending > 0) overlapped = true;
  else
    refreshing ??= dashboard.run().finally(() => (refreshing = undefined));
}
