import "server-only";
// Server-owned destination. A caller cannot supply another origin or obtain operator keys.
export async function forward(request: Request): Promise<Response> {
  const incoming = new URL(request.url);
  const configured = process.env.KUSI_API_ORIGIN || "http://127.0.0.1:8080";
  const upstream = new URL(configured);
  if (
    !["http:", "https:"].includes(upstream.protocol) ||
    upstream.username ||
    upstream.password ||
    upstream.pathname !== "/" ||
    upstream.search ||
    upstream.hash
  ) {
    return Response.json(
      { error: "invalid_backend_configuration" },
      { status: 503 },
    );
  }
  const headers = new Headers();
  for (const name of [
    "content-type",
    "cookie",
    "x-csrf-token",
    "origin",
    "accept",
    "range",
  ]) {
    const value = request.headers.get(name);
    if (value) headers.set(name, value);
  }
  try {
    const init: RequestInit & { duplex?: "half" } = {
      method: request.method,
      headers,
      redirect: "manual",
      cache: "no-store",
      signal: AbortSignal.timeout(40_000),
    };
    if (!["GET", "HEAD"].includes(request.method)) {
      init.body = request.body;
      init.duplex = "half";
    }
    const response = await fetch(
      upstream.origin + incoming.pathname + incoming.search,
      init,
    );
    const outgoing = new Headers({
      "Cache-Control": "no-store",
      "X-Content-Type-Options": "nosniff",
    });
    for (const name of [
      "content-type",
      "content-length",
      "content-range",
      "accept-ranges",
      "x-request-id",
      "location",
    ]) {
      const value = response.headers.get(name);
      if (value) outgoing.set(name, value);
    }
    for (const value of response.headers.getSetCookie())
      outgoing.append("set-cookie", value);
    return new Response(response.body, {
      status: response.status,
      headers: outgoing,
    });
  } catch {
    return Response.json(
      { error: "backend_unavailable" },
      { status: 503, headers: { "Cache-Control": "no-store" } },
    );
  }
}
