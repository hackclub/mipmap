import { json, request, send_json } from "$lib/api";

export async function send_feedback(feedback: {
  score: number;
  source: string;
  positives: string;
  improvements: string;
}) {
  const response = await request("/api/feedback", send_json("POST", feedback));
  if (!response.ok) await json(response);
}
