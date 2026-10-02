import { json, request } from "$lib/api";

export interface Profile {
  first_name: string;
  fragments: number;
}

export const fetch_referral = () =>
  request("/api/referral").then((r) => json<{ code: string }>(r));

export const fetch_profile = () =>
  request("/api/profile").then((r) => json<Profile>(r));
