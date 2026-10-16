import { get_json, json, request, send_json } from "$lib/api";
import type {
  Action,
  Feedback,
  Order,
  Ship,
  Snapshot,
  Stickers,
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

export const create_wave = () => post<Stickers>("stickers/waves", {});

export const fulfill_wave = (id: number) =>
  post<Stickers>(`stickers/waves/${id}/fulfill`, {});

export const wave_csv = (id: number) => `/api/admin/stickers/waves/${id}/csv`;

export const safe_url = (url: string | null) =>
  url && /^https?:\/\//i.test(url) ? url : undefined;

export const display_name = (name: string, id: number) =>
  name.trim() || `User #${id}`;
