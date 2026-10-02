import { json, request } from "$lib/api";
import type {
  HackatimeProject,
  Project,
  ProjectFields,
} from "$lib/projects/types";

const send = (method: string, body: unknown) => ({
  method,
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(body),
});

export const fetch_projects = () =>
  request("/api/projects").then((r) => json<Project[]>(r));

export const create_project = (name: string) =>
  request("/api/projects", send("POST", { name })).then((r) =>
    json<Project>(r),
  );

export const update_project = (id: number, fields: ProjectFields) =>
  request(`/api/projects/${id}`, send("PUT", fields)).then((r) =>
    json<Project>(r),
  );

export const upload_screenshot = (id: number, file: File) =>
  request(`/api/projects/${id}/screenshot`, {
    method: "POST",
    headers: { "Content-Type": file.type },
    body: file,
  }).then((r) => json<Project>(r));

export const fetch_hackatime_projects = () =>
  request("/dashboard/hackatime-projects.json").then((r) =>
    json<HackatimeProject[]>(r),
  );

export const hours = (seconds: number) =>
  `${(seconds / 3600).toFixed(1)} hours`;
