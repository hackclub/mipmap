import { request } from "$lib/api";

export async function log_out(everywhere = false) {
  const response = await request(
    everywhere ? "/api/logout-all" : "/api/logout",
    { method: "POST" },
  ).catch(() => null);
  if (!response?.ok)
    throw new Error("Couldn't log out, wait a moment and try again");
  location.href = "/";
}
