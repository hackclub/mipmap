const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export async function request(
  input: string,
  init?: RequestInit,
  retries = 3,
): Promise<Response> {
  const response = await fetch(input, { ...init, redirect: "manual" }).catch(
    (error) => {
      if (error instanceof TypeError && (init?.method ?? "GET") !== "GET")
        throw new Error("Couldn't save, wait a moment and try again");
      throw error;
    },
  );
  if (response.type === "opaqueredirect" || response.status === 401) {
    location.href = "/login";
    return new Promise(() => {});
  }
  const retry_after = Number(response.headers.get("retry-after"));
  if (response.status === 429 && retry_after > 0 && retries > 0) {
    await wait(Math.min(retry_after, 30) * 1000);
    return request(input, init, retries - 1);
  }
  return response;
}

const reasons: Record<number, string> = {
  400: "Bad Request",
  403: "Forbidden",
  404: "Not Found",
  405: "Method Not Allowed",
  408: "Request Timeout",
  409: "Conflict",
  413: "Content Too Large",
  429: "Too Many Requests",
  500: "Internal Server Error",
  502: "Bad Gateway",
  503: "Service Unavailable",
  504: "Gateway Timeout",
  520: "Web Server Returned an Unknown Error",
  521: "Web Server Is Down",
  522: "Connection Timed Out",
  523: "Origin Is Unreachable",
  524: "A Timeout Occurred",
};

export async function json<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    const reason = reasons[response.status] ?? response.statusText;
    throw new Error(
      body.error ??
        `Request failed (${response.status})${reason ? `: ${reason}` : ""}`,
    );
  }
  return response.json();
}

export const get_json = <T>(input: string) =>
  request(input).then((r) => json<T>(r));

export const send_json = (method: string, body: unknown): RequestInit => ({
  method,
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(body),
});

export const image_types = "image/png,image/jpeg,image/webp";

export const image_error = (file: File) =>
  !image_types.split(",").includes(file.type)
    ? "Please choose a PNG, JPEG or WebP image"
    : file.size > 10_000_000
      ? "Images must be 10 MB or smaller"
      : "";

export const message = (error: unknown, fallback?: string) =>
  error instanceof Error ? error.message : (fallback ?? String(error));
