import { useState } from "react";
import { api } from "@/api";
import type { Context } from "@/types";
import { Button } from "@/components/ui/button";
import { useManagedData } from "./managed-data";
type Memory = { id: string; source_ref: string; expires_at: number };
export default function AuthorizedMemory(ctx: Context) {
  const preference = useManagedData<{ consent: boolean; ttl_seconds: number }>(
    "/v1/local/authorized-memories/preferences",
    ctx.revision,
    { consent: false, ttl_seconds: 604800 },
  );
  const memories = useManagedData<Memory[]>(
    "/v1/local/authorized-memories",
    ctx.revision,
    [],
  );
  const [busy, setBusy] = useState(false);
  async function toggle() {
    setBusy(true);
    try {
      await ctx.run(async () => {
        await api("/v1/local/authorized-memories/preferences", "PUT", {
          consent: !preference.data.consent,
          ttl_seconds: 604800,
        });
        ctx.updated();
        ctx.setNotice(
          preference.data.consent
            ? "Memoria desactivada y contenido retenido eliminado."
            : "Memoria habilitada para futuras lecturas autorizadas de los recursos de demostración.",
        );
      });
    } finally {
      setBusy(false);
    }
  }
  async function forget(id: string) {
    setBusy(true);
    try {
      await ctx.run(async () => {
        await api(`/v1/local/authorized-memories/${id}`, "DELETE");
        ctx.updated();
        ctx.setNotice("Contexto eliminado de la recuperación.");
      });
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="space-y-5" aria-label="Mi memoria autorizada">
      <h2>Mi contexto autorizado</h2>
      <p className="text-sm text-muted-foreground">
        Desactivado por defecto. Con tu consentimiento, conserva contexto
        cifrado de las lecturas de demostración hasta siete días. Solo tú puedes
        recuperarlo y los permisos se vuelven a comprobar. No hay memoria
        compartida ni aprendizaje automático global.
      </p>
      {preference.loading ? (
        <p role="status">Cargando preferencia…</p>
      ) : preference.error ? (
        <p role="alert">{preference.error}</p>
      ) : (
        <>
          <p>
            Memoria {preference.data.consent ? "habilitada" : "desactivada"}
          </p>
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => void toggle()}
          >
            {preference.data.consent
              ? "Desactivar y borrar contexto"
              : "Permitir memoria durante 7 días"}
          </Button>
        </>
      )}
      {memories.error && <p role="alert">{memories.error}</p>}
      {memories.loading ? (
        <p role="status">Cargando contexto…</p>
      ) : !memories.data.length ? (
        <p>
          No hay contexto vigente. Si habilitas la memoria, una lectura
          autorizada en Accesos a recursos podrá conservarlo.
        </p>
      ) : (
        <ul className="divide-y">
          {memories.data.map((item) => (
            <li
              className="flex flex-wrap items-center justify-between gap-3 py-4"
              key={item.id}
            >
              <div>
                <p>
                  {item.source_ref === "demo-resource-001"
                    ? "Recurso de demostración A"
                    : item.source_ref === "demo-resource-002"
                      ? "Recurso de demostración B"
                      : item.source_ref}
                </p>
                <p className="text-sm text-muted-foreground">
                  Vence{" "}
                  {new Date(item.expires_at * 1000).toLocaleString("es-PE")}
                </p>
              </div>
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void forget(item.id)}
              >
                Olvidar contexto
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
