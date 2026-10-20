import { request, send_json } from "$lib/api";
import { mutate } from "$lib/dashboard/state.svelte";

const unsaved_key = "onboarding_unsaved";

export function unsaved(clear = false) {
  try {
    if (clear) localStorage.removeItem(unsaved_key);
    else return !!localStorage.getItem(unsaved_key);
  } catch {}
  return false;
}

export const save_step = (step: string) =>
  request("/api/onboarding/step", send_json("POST", { step }))
    .then((response) => response.ok)
    .catch(() => false)
    .then((ok) => {
      try {
        if (ok) localStorage.removeItem(unsaved_key);
        else localStorage.setItem(unsaved_key, "1");
      } catch {}
    });

export const grant_mips = () =>
  mutate("/api/onboarding/grant", { method: "POST" });

export const finish = () => mutate("/api/onboarding/finish", { method: "POST" });

export const join_slack = () =>
  request("/api/slack/join", { method: "POST" })
    .then((response) => response.ok)
    .catch(() => false);
