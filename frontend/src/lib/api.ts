const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export async function request(
  input: string,
  init?: RequestInit,
  retries = 3,
): Promise<Response> {
  const response = await fetch(input, { ...init, redirect: "manual" });
  if (response.type === "opaqueredirect" || response.status === 401) {
    location.href = "/login";
    return new Promise(() => {});
  }
  if (response.status === 429 && retries > 0) {
    await wait(Number(response.headers.get("retry-after") ?? 1) * 1000);
    return request(input, init, retries - 1);
  }
  return response;
}

export async function json<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body.error ?? `Request failed (${response.status})`);
  }
  return response.json();
}

export const message = (error: unknown) =>
  error instanceof Error ? error.message : String(error);
