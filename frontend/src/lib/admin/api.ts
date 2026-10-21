import { json, request, send_json } from "$lib/api";
import { session } from "./session.svelte";
import type {
  Action,
  AdminContact,
  Draft,
  Feedback,
  HeldSnapshot,
  LocatedPerson,
  Locations,
  LogEntry,
  Order,
  Ship,
  Snapshot,
  SnapshotRejection,
  SlackMessage,
  Stickers,
  SurveyResponse,
  SurveyResults,
  Template,
  Thread,
  User,
} from "./types";

export const page_size = 50;

async function admin(path: string, init?: RequestInit) {
  const response = await request(`/api/admin/${path}`, init);
  if (
    response.status === 404 &&
    (path === "overview" ||
      (await request("/api/admin/overview").then(
        (probe) => probe.status === 404,
        () => false,
      )))
  ) {
    session.ended = true;
    throw new Error("Your admin session ended. Log in again");
  }
  return response;
}

const get = <T>(path: string) => admin(path).then((r) => json<T>(r));

const list =
  <T>(path: string) =>
  (params: Record<string, string | number | undefined> = {}) => {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(params))
      if (value !== undefined && value !== "") query.set(key, String(value));
    return get<T[]>(`${path}?${query}`);
  };

const post = <T>(path: string, body: unknown) =>
  admin(path, send_json("POST", body)).then((r) => json<T>(r));

export const fetch_overview = () =>
  get<{ unfulfilled_orders: number }>("overview");

export const fetch_ships = list<Ship>("ships");
export const fetch_snapshots = list<Snapshot>("snapshots");
export const fetch_held_snapshots = list<HeldSnapshot>("snapshots/held");
export const fetch_snapshot_rejections =
  list<SnapshotRejection>("snapshots/rejections");
export const fetch_users = list<User>("users");
export const fetch_orders = list<Order>("orders");
export const fetch_feedback = list<Feedback>("feedback");
export const fetch_audit = list<Action>("audit");

export const set_bonus = (
  id: number,
  body: { percent: number; mips: number; reason: string },
) => post<Ship>(`ships/${id}/bonus`, body);

export const set_shadowban = (
  id: number,
  body: { shadowbanned: boolean; reason: string },
) => post<Snapshot>(`snapshots/${id}/shadowban`, body);

export const review_snapshot = async (id: number, approve: boolean) => {
  const response = await admin(
    `snapshots/${id}/review`,
    send_json("POST", { approve }),
  );
  if (!response.ok) await json(response);
};

export const set_mips = (id: number, body: { mips: number; reason: string }) =>
  post<User>(`users/${id}/mips`, body);
export const adjust_mips = (
  id: number,
  body: { change: number; reason: string },
) => post<User>(`users/${id}/mips/adjust`, body);

export const set_banned = (
  id: number,
  body: { banned: boolean; reason: string },
) => post<User>(`users/${id}/ban`, body);

export const set_fulfilled = (
  id: number,
  body: { fulfilled: boolean; tracking_number: string },
) => post<Order>(`orders/${id}/fulfill`, body);

export const cancel_order = async (id: number, reason: string) => {
  const response = await admin(
    `orders/${id}/cancel`,
    send_json("POST", { reason }),
  );
  if (!response.ok) await json(response);
};

export const fetch_stickers = () =>
  get<Stickers>("stickers");

export const create_wave = (body: { no_time: boolean; cap: number | null }) =>
  post<Stickers>("stickers/waves", body);

export const fulfill_wave = (id: number) =>
  post<Stickers>(`stickers/waves/${id}/fulfill`, {});

export const wave_csv = (id: number) => `/api/admin/stickers/waves/${id}/csv`;

export function safe_url(url: string | null) {
  if (!url || /[\\\u0000-\u001f\u007f]/.test(url)) return undefined;
  try {
    const parsed = new URL(url, location.origin);
    if (url.startsWith("/"))
      return parsed.origin === location.origin ? parsed.href : undefined;
    return /^(?:https?:\/\/|mailto:)/i.test(url) ? parsed.href : undefined;
  } catch {
    return undefined;
  }
}

export const display_name = (name: string, id: number) =>
  name.trim() || `User #${id}`;

export const fetch_slack_log = list<LogEntry>("slack/log");
export const fetch_drafts = () => get<Draft[]>("slack/drafts");
export const send_draft = (id: number) =>
  post<Draft[]>(`slack/drafts/${id}/send`, {});
export const dismiss_draft = (id: number) =>
  post<Draft[]>(`slack/drafts/${id}/dismiss`, {});
export const refresh_draft = (id: number) =>
  post<Draft[]>(`slack/drafts/${id}/refresh`, {});

const post_empty = (path: string, body: unknown) =>
  admin(path, send_json("POST", body)).then(async (r) => {
    if (!r.ok) await json(r);
  });

export const send_key = (..._inputs: unknown[]) => crypto.randomUUID();

export const send_announcement = (text: string, key: string) =>
  post<{ recipients: number }>("slack/announcements", { text, key });
export const send_direct = (slack_id: string, text: string, key: string) =>
  post_empty("slack/direct", { slack_id, text, key });

export const fetch_locations = () => get<Locations>("locations");
export const fetch_located_people = (country: string) =>
  get<LocatedPerson[]>(`locations/people?${new URLSearchParams({ country })}`);

export const fetch_slack_admins = () =>
  get<AdminContact[]>("slack/admins");
export const fetch_slack_templates = () =>
  get<Template[]>("slack/templates");

export const threads_cap = 100;
export const fetch_threads = () => get<Thread[]>("slack/threads");
export const fetch_thread = (slack_id: string) =>
  get<SlackMessage[]>(`slack/threads/${encodeURIComponent(slack_id)}`);
export const mark_read = (slack_id: string) =>
  post_empty(`slack/threads/${encodeURIComponent(slack_id)}/read`, {});
export const reply = (slack_id: string, text: string, key: string) =>
  post<SlackMessage[]>(
    `slack/threads/${encodeURIComponent(slack_id)}/reply`,
    { text, key },
  );

export const fetch_surveys = (before?: number) =>
  get<SurveyResults[]>(`surveys${before ? `?before=${before}` : ""}`);

export const fetch_survey_responses = (
  id: number,
  params: { before?: number } = {},
) => list<SurveyResponse>(`surveys/${id}/responses`)(params);

export const delete_survey_response = async (id: number) => {
  const response = await admin(`surveys/responses/${id}/delete`, {
    method: "POST",
  });
  if (!response.ok) await json(response);
};
