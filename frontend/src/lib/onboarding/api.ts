import { request, send_json } from "$lib/api";
import { dashboard, mutate } from "$lib/dashboard/state.svelte";

const unsaved_key = "onboarding_unsaved";

const whoami = () => dashboard.data?.user.referral_code;

export function unsaved(clear = false, user = whoami()) {
  try {
    if (clear) localStorage.removeItem(unsaved_key);
    else {
      const stored = localStorage.getItem(unsaved_key);
      return !!stored && (!user || stored === user);
    }
  } catch {}
  return false;
}

let saving = Promise.resolve();

export const save_step = (step: string) =>
  (saving = saving.then(() =>
    request("/api/onboarding/step", send_json("POST", { step }))
      .then((response) => response.ok)
      .catch(() => false)
      .then((ok) => {
        const user = whoami();
        try {
          if (ok) localStorage.removeItem(unsaved_key);
          else if (user) localStorage.setItem(unsaved_key, user);
        } catch {}
      }),
  ));

export const grant_mips = () =>
  mutate("/api/onboarding/grant", { method: "POST" });

export const finish = () => mutate("/api/onboarding/finish", { method: "POST" });

export const join_slack = () =>
  request("/api/slack/join", { method: "POST" })
    .then((response) => response.ok)
    .catch(() => false);
