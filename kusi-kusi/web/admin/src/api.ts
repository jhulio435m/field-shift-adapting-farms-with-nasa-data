export type User = {
  id: string;
  organization_id: string;
  organization: string;
  username: string;
  display_name: string;
  role: "owner" | "editor" | "approver" | "viewer";
  platform_admin: boolean;
  disabled: boolean;
};
export type Asset = {
  id: string;
  label: string;
  filename: string;
  media_type: string;
  size_bytes: number;
  sha256: string;
  simulated: boolean;
  creative_job_id: string | null;
  created_at: number;
};
export type Job = {
  id: string;
  state: string;
  approval_digest: string;
  estimated_cost_microusd: number;
  proposal: {
    provider: string;
    model: string;
    estimated_cost_microusd: number;
    input: { prompt: string; asset_id: string; kind: string };
  };
  result: unknown;
  created_at: number;
};
export type Publication = {
  id: string;
  asset_id: string;
  platform: string;
  caption: string;
  privacy: string;
  scheduled_at: number | null;
  revision: number;
  approval_digest: string;
  state: string;
  receipt: string | null;
  idempotency_key: string;
};
export type Conversation = { id: string; title: string; local: boolean };
export type Message = {
  id: number;
  direction: string;
  content: string;
  metadata: {
    creative_job_id?: string;
    publication_id?: string;
    asset_id?: string;
    simulated?: boolean;
    simulation_kind?: string;
  };
  created_at: number;
};
let csrf = "";
export function setCsrf(value: string) {
  csrf = value;
}
export async function api<T>(
  path: string,
  method = "GET",
  body?: unknown,
): Promise<T> {
  const headers: Record<string, string> = {};
  if (method !== "GET") headers["x-csrf-token"] = csrf;
  if (body !== undefined) headers["content-type"] = "application/json";
  const response = await fetch(path, {
    method,
    headers,
    credentials: "same-origin",
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (response.status === 401 && csrf) {
    csrf = "";
    window.dispatchEvent(new Event("kusi:unauthorized"));
  }
  if (!response.ok)
    throw new Error(
      (
        {
          400: "Revisa los datos del formulario.",
          401: "Tu sesión venció. Inicia sesión de nuevo.",
          403: "Tu rol no permite esta acción o falta una confirmación válida.",
          404: "No se encontró el recurso.",
          409: "El borrador cambió, venció o ya se procesó. Actualiza antes de confirmar.",
          429: "Demasiadas solicitudes. Espera un momento.",
          503: "El servicio no está disponible. Revisa el registro del servidor.",
        } as Record<number, string>
      )[response.status] || `La solicitud falló (${response.status}).`,
    );
  const payload = await response.text();
  if (!payload.trim()) return undefined as T;
  try {
    return JSON.parse(payload) as T;
  } catch {
    throw new Error("El servidor devolvió una respuesta inválida.");
  }
}
export async function upload(file: File, label: string): Promise<Asset> {
  const response = await fetch(
    `/v1/local/assets?${new URLSearchParams({ filename: file.name, label })}`,
    { method: "POST", headers: { "x-csrf-token": csrf }, body: file },
  );
  if (response.status === 401 && csrf) {
    csrf = "";
    window.dispatchEvent(new Event("kusi:unauthorized"));
  }
  if (!response.ok)
    throw new Error(
      "No se pudo cargar. Usa PNG, JPG, WebP o un video MP4/WebM válido de hasta 10 MB.",
    );
  return response.json();
}
export const assetUrl = (id: string) => `/v1/local/assets/${id}/content`;
export const labels: Record<string, string> = {
  owner: "Propietario",
  editor: "Editor",
  approver: "Aprobador",
  viewer: "Solo lectura",
  awaiting_approval: "Pendiente de aprobación",
  approved: "Aprobado",
  submitting: "Enviando",
  queued: "En cola",
  processing: "Procesando",
  succeeded: "Completado",
  failed: "Fallido",
  unknown: "Estado incierto",
  canceled: "Cancelado",
  draft: "Borrador",
  scheduled: "Programado",
  published_simulated: "Publicado · simulación",
};
export const label = (value: string) => labels[value] || value;
export const date = (value: number | null) =>
  value
    ? new Date(value * 1000).toLocaleString("es-PE", {
        dateStyle: "medium",
        timeStyle: "short",
      })
    : "Al aprobar";
