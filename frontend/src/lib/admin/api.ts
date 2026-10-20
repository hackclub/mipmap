import { get_json, json, request, send_json } from "$lib/api";
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

const list =
  <T>(path: string) =>
  (params: Record<string, string | number | undefined> = {}) => {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(params))
      if (value !== undefined && value !== "") query.set(key, String(value));
    return get_json<T[]>(`/api/admin/${path}?${query}`);
  };

const post = <T>(path: string, body: unknown) =>
  request(`/api/admin/${path}`, send_json("POST", body)).then((r) =>
    json<T>(r),
  );

export const fetch_overview = () =>
  get_json<{ unfulfilled_orders: number }>("/api/admin/overview");

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
  const response = await request(
    `/api/admin/snapshots/${id}/review`,
    send_json("POST", { approve }),
  );
  if (!response.ok) await json(response);
};

export const set_mips = (id: number, body: { mips: number; reason: string }) =>
  post<User>(`users/${id}/mips`, body);

export const set_banned = (
  id: number,
  body: { banned: boolean; reason: string },
) => post<User>(`users/${id}/ban`, body);

export const set_fulfilled = (
  id: number,
  body: { fulfilled: boolean; tracking_number: string },
) => post<Order>(`orders/${id}/fulfill`, body);

export const cancel_order = async (id: number, reason: string) => {
  const response = await request(
    `/api/admin/orders/${id}/cancel`,
    send_json("POST", { reason }),
  );
  if (!response.ok) await json(response);
};

export const fetch_stickers = () =>
  get_json<Stickers>("/api/admin/stickers");

export const create_wave = (body: { no_time: boolean; cap: number | null }) =>
  post<Stickers>("stickers/waves", body);

export const fulfill_wave = (id: number) =>
  post<Stickers>(`stickers/waves/${id}/fulfill`, {});

export const wave_csv = (id: number) => `/api/admin/stickers/waves/${id}/csv`;

export const safe_url = (url: string | null) =>
  url && /^(?:https?:\/\/|\/(?!\/))/i.test(url) ? url : undefined;

export const display_name = (name: string, id: number) =>
  name.trim() || `User #${id}`;

export const fetch_slack_log = list<LogEntry>("slack/log");
export const fetch_drafts = () => get_json<Draft[]>("/api/admin/slack/drafts");
export const send_draft = (id: number) =>
  post<Draft[]>(`slack/drafts/${id}/send`, {});
export const dismiss_draft = (id: number) =>
  post<Draft[]>(`slack/drafts/${id}/dismiss`, {});

const post_empty = (path: string, body: unknown) =>
  request(`/api/admin/${path}`, send_json("POST", body)).then(async (r) => {
    if (!r.ok) await json(r);
  });

export const send_announcement = (text: string) =>
  post_empty("slack/announcements", { text });
export const send_direct = (slack_id: string, text: string) =>
  post_empty("slack/direct", { slack_id, text });

export const fetch_locations = () => get_json<Locations>("/api/admin/locations");
export const fetch_located_people = (country: string) =>
  get_json<LocatedPerson[]>(
    `/api/admin/locations/people?${new URLSearchParams({ country })}`,
  );

export const fetch_slack_admins = () =>
  get_json<AdminContact[]>("/api/admin/slack/admins");
export const fetch_slack_templates = () =>
  get_json<Template[]>("/api/admin/slack/templates");

export const fetch_threads = () =>
  get_json<Thread[]>("/api/admin/slack/threads");
export const fetch_thread = (slack_id: string) =>
  get_json<SlackMessage[]>(
    `/api/admin/slack/threads/${encodeURIComponent(slack_id)}`,
  );
export const reply = (slack_id: string, text: string) =>
  post<SlackMessage[]>(
    `slack/threads/${encodeURIComponent(slack_id)}/reply`,
    { text },
  );

export const fetch_surveys = () =>
  get_json<SurveyResults[]>("/api/admin/surveys");

export const fetch_survey_responses = (
  id: number,
  params: { before?: number } = {},
) => list<SurveyResponse>(`surveys/${id}/responses`)(params);

export const delete_survey_response = async (id: number) => {
  const response = await request(
    `/api/admin/surveys/responses/${id}/delete`,
    { method: "POST" },
  );
  if (!response.ok) await json(response);
};
