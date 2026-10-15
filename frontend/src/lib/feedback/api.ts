import { request, send_json } from "$lib/api";
import { mutate } from "$lib/dashboard/state.svelte";

export const send_feedback = (feedback: {
  score: number;
  source: string;
  positives: string;
  improvements: string;
}) => mutate("/api/feedback", send_json("POST", feedback));

export const visit_survey = () =>
  request("/api/survey/visit", { method: "POST" })
    .then((response) => (response.ok ? response.json() : { show: false }))
    .then((body: { show: boolean }) => body.show)
    .catch(() => false);

export const dismiss_survey = () =>
  request("/api/survey/dismiss", { method: "POST" }).catch(() => {});
