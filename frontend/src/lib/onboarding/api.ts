import { request, send_json } from "$lib/api";
import { mutate } from "$lib/dashboard/state.svelte";

export const save_step = (step: string) =>
  request("/api/onboarding/step", send_json("POST", { step })).catch(() => {});

export const grant_mips = () =>
  mutate("/api/onboarding/grant", { method: "POST" });

export const finish = () => mutate("/api/onboarding/finish", { method: "POST" });
