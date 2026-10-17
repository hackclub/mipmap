import { json, request, send_json } from "$lib/api";
import { dashboard, mutate, type Dashboard } from "$lib/dashboard/state.svelte";
import type { CheckEvent, ProjectFields } from "$lib/projects/types";

type ShipLine =
  | CheckEvent
  | { dashboard: Dashboard | null; waiting: boolean }
  | { error: string };

export const create_project = (name: string) =>
  mutate("/api/projects", send_json("POST", { name }));

export const update_project = (id: number, fields: ProjectFields) =>
  mutate(`/api/projects/${id}`, send_json("PUT", fields));

export const delete_project = (id: number) =>
  mutate(`/api/projects/${id}`, { method: "DELETE" });

export const upload_screenshot = (id: number, file: File) =>
  mutate(`/api/projects/${id}/screenshot`, {
    method: "POST",
    headers: { "Content-Type": file.type },
    body: file,
  });

export async function ship_project(
  id: number,
  onevent: (event: CheckEvent) => void,
) {
  const response = await request(`/api/projects/${id}/ship`, {
    method: "POST",
  });
  if (!response.ok) await json(response);
  if (!response.body) throw new Error("Lost connection while shipping");

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  let shipped = false;
  let waiting = false;

  const handle = (line: string) => {
    if (!line.trim()) return;
    const event: ShipLine = JSON.parse(line);
    if ("error" in event) throw new Error(event.error);
    if ("dashboard" in event) {
      if (event.dashboard) dashboard.set(event.dashboard);
      else dashboard.run();
      shipped = true;
      waiting = event.waiting;
    } else {
      onevent(event);
    }
  };

  try {
    for (;;) {
      const { done, value } = await reader.read();
      buffer += decoder.decode(value, { stream: !done });
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";
      lines.forEach(handle);
      if (done) break;
    }
    handle(buffer);
  } finally {
    reader.cancel().catch(() => {});
  }

  if (!shipped) throw new Error("Lost connection while shipping");
  return { waiting };
}
