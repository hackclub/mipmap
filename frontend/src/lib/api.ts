const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const offline =
  "Couldn't reach Mipmap, check your connection and try again";
const unreadable = "Got an unexpected response, try again in a moment";

const needs_login = () =>
  /^\/(dashboard|notifications|admin)(\/|$)/.test(location.pathname);

export async function request(
  input: string,
  init?: RequestInit,
  retries = 3,
): Promise<Response> {
  const response = await fetch(input, { ...init, redirect: "manual" }).catch(
    (error) => {
      if (!(error instanceof TypeError)) throw error;
      throw new Error(
        (init?.method ?? "GET") !== "GET"
          ? "Couldn't save, wait a moment and try again"
          : offline,
      );
    },
  );
  if (response.type === "opaqueredirect" || response.status === 401) {
    if (!needs_login())
      throw new Error("You're signed out, log in and try again");
    const next = location.pathname + location.search;
    location.href = `/login?next=${encodeURIComponent(next)}`;
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
  return response.json().catch((error) => {
    if (error instanceof SyntaxError) throw new Error(unreadable);
    if (error instanceof TypeError) throw new Error(offline);
    throw error;
  });
}

export const get_json = <T>(input: string) =>
  request(input).then((r) => json<T>(r));

export const send_json = (method: string, body: unknown): RequestInit => ({
  method,
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(body),
});

export const image_types = "image/png,image/jpeg,image/webp";

const loose_types = ["", "application/octet-stream"];

export const image_error = (file: File) =>
  /^image\/hei[cf]/i.test(file.type) || /\.hei[cf]$/i.test(file.name)
    ? "HEIC isn't supported, please choose a PNG, JPEG or WebP"
    : ![...image_types.split(","), ...loose_types].includes(file.type)
      ? "Please choose a PNG, JPEG or WebP image"
      : "";

const max_side = 2560;

function output_type(file: File) {
  const type = file.type || "";
  if (type === "image/png" || /\.png$/i.test(file.name)) return "image/png";
  if (type === "image/webp" || /\.webp$/i.test(file.name)) return "image/webp";
  return "image/jpeg";
}

export async function shrink_image(file: File): Promise<File> {
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file, { imageOrientation: "from-image" });
  } catch {
    return file;
  }
  try {
    const scale = max_side / Math.max(bitmap.width, bitmap.height);
    if (scale >= 1) return file;
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(bitmap.width * scale));
    canvas.height = Math.max(1, Math.round(bitmap.height * scale));
    const context = canvas.getContext("2d");
    if (!context) return file;
    context.imageSmoothingQuality = "high";
    context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    const type = output_type(file);
    const blob = await new Promise<Blob | null>((resolve) =>
      canvas.toBlob(resolve, type, 0.9),
    );
    if (!blob) return file;
    const extension = blob.type.split("/")[1] ?? "jpeg";
    const name = file.name.replace(/\.[^.]*$/, "") || "image";
    return new File([blob], `${name}.${extension}`, { type: blob.type });
  } catch {
    return file;
  } finally {
    bitmap.close();
  }
}

export async function prepare_image(file: File) {
  const error = image_error(file);
  if (error) throw new Error(error);
  const shrunk = await shrink_image(file);
  if (shrunk.size > 10_000_000)
    throw new Error("Images must be 10 MB or smaller");
  return shrunk;
}

export const message = (error: unknown, fallback?: string) =>
  error instanceof Error ? error.message : (fallback ?? String(error));
