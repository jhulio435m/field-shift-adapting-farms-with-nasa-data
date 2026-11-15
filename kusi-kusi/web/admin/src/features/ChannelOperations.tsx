import { useState } from "react";
import { api } from "@/api";
import type { Context } from "@/types";
import { Button } from "@/components/ui/button";
import { Confirmation } from "@/ui";
import { useManagedData } from "./managed-data";
type Binding = {
  id: string;
  user_id: string;
  version: number;
  expires_at: number;
  revoked: boolean;
};
export default function ChannelOperations(ctx: Context) {
  const { data, loading, error } = useManagedData<{ bindings: Binding[] }>(
    "/v1/local/channel-operations/telegram",
    ctx.revision,
    { bindings: [] },
  );
  const [selected, setSelected] = useState<Binding | null>(null);
  const [busy, setBusy] = useState(false);
  async function revoke() {
    if (!selected || busy) return;
    setBusy(true);
    try {
      await ctx.run(async () => {
        await api(
          `/v1/local/channel-operations/telegram/bindings/${selected.id}/revoke`,
          "POST",
          { expected_version: selected.version },
        );
        ctx.updated();
        ctx.setNotice(
          "Vínculo revocado. Las consultas nuevas requieren otra vinculación y permisos vigentes.",
        );
      });
    } finally {
      setBusy(false);
      setSelected(null);
    }
  }
  return (
    <section className="space-y-5" aria-label="Operación del canal">
      <h2>Vínculos del canal</h2>
      <p className="text-sm text-muted-foreground">
        Contextos registrados en esta organización. No se muestran
        identificadores de chat ni mensajes. El entorno local no entrega
        mensajes ni registra webhooks.
      </p>
      {loading ? (
        <p role="status">Cargando vínculos…</p>
      ) : error ? (
        <p role="alert">{error}</p>
      ) : !data.bindings.length ? (
        <p>
          No hay vínculos registrados. Genera un código desde Conexiones cuando
          dispongas de un canal registrado.
        </p>
      ) : (
        <ul className="divide-y">
          {data.bindings.map((binding) => (
            <li
              key={binding.id}
              className="flex flex-wrap items-center justify-between gap-3 py-4"
            >
              <div>
                <p>Vínculo {binding.id.slice(0, 8)}</p>
                <p className="text-sm text-muted-foreground">
                  {binding.revoked
                    ? "Revocado"
                    : binding.expires_at * 1000 <= Date.now()
                      ? "Vencido"
                      : "Vigente"}{" "}
                  · Vence{" "}
                  {new Date(binding.expires_at * 1000).toLocaleString("es-PE")}
                </p>
              </div>
              {!binding.revoked && (
                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={() => setSelected(binding)}
                >
                  Revocar vínculo
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
      <Confirmation
        open={!!selected}
        onOpenChange={(open) => {
          if (!open && !busy) setSelected(null);
        }}
        title="¿Revocar este vínculo?"
        description="Se bloquearán consultas nuevas desde este contexto y se descartará su contexto conversacional retenido. No revoca tokens del proveedor."
        onConfirm={() => void revoke()}
      />
    </section>
  );
}
