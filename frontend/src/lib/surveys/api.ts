import { json, request, send_json } from "$lib/api";

const key = (id: number) => `survey-${id}`;

export const completed = (id: number) => {
  try {
    return !!localStorage.getItem(key(id));
  } catch {
    return false;
  }
};

export const mark_completed = (id: number) => {
  try {
    localStorage.setItem(key(id), "1");
  } catch {}
};

export const submit_survey = async (id: number, answers: unknown) => {
  const response = await request(
    `/api/surveys/${id}`,
    send_json("POST", answers),
  );
  if (!response.ok) await json(response);
};
