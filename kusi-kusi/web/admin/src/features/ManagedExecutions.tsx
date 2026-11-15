import { useState } from "react";
import { api } from "@/api";
import type { Context } from "@/types";
import { Button } from "@/components/ui/button";
import { useManagedData } from "./managed-data";
type Execution = {
  id: string;
  version: number;
  state: string;
  proposal_digest: string;
  updated_at: number;
  timeline: { version: number; state: string; at: number }[];
};
type Proposal = {
  id: string;
  version: number;
  proposal_digest: string;
  operation: string;
  resource_ref: string;
  destination: string;
  content: string;
};
const states: Record<string, string> = {
  pending: "Pendiente",
  running: "En curso",
  uncertain: "Resultado incierto",
  cancelled: "Cancelada",
  completed: "Completada",
  failed: "Fallida",
  awaiting_review: "Pendiente de revisión",
};
export default function ManagedExecutions(ctx: Context) {
  const { data, loading, error } = useManagedData<Execution[]>(
    "/v1/local/managed-executions",
    ctx.revision,
    [],
  );
  const [proposal, setProposal] = useState<Proposal | null>(null);
  const [detail, setDetail] = useState<Execution | null>(null),
    [busy, setBusy] = useState(false);
  async function inspect(item: Execution) {
    setBusy(true);
    setProposal(null);
    try {
      await ctx.run(async () => {
        const result = await api<Execution>(
          `/v1/local/managed-executions/${item.id}`,
        );
        setDetail(result);
        if (
          result.state === "awaiting_review" &&
          (ctx.user.role === "owner" || ctx.user.role === "approver")
        ) {
          setProposal(
            await api<Proposal>(
              `/v1/local/managed-executions/${item.id}/proposal`,
            ),
          );
        }
      });
    } finally {
      setBusy(false);
    }
  }
  async function act(
    item: Execution,
    action: "cancel" | "resume" | "approve" | "reject",
  ) {
    setBusy(true);
    try {
      await ctx.run(async () => {
        setDetail(
          await api<Execution>(
            `/v1/local/managed-executions/${item.id}/${action === "approve" || action === "reject" ? "review" : action}`,
            "POST",
            {
              expected_version: item.version,
              ...(action !== "cancel"
                ? { proposal_digest: item.proposal_digest }
                : {}),
              ...(action === "approve" || action === "reject"
                ? { decision: action }
                : {}),
            },
          ),
        );
        ctx.updated();
        ctx.setNotice(
          action === "cancel"
            ? "Cancelación registrada para la tarea pendiente."
            : action === "resume"
              ? "La tarea sigue en la cola persistente; no se creó otro envío."
              : "Decisión registrada para esta revisión local.",
        );
      });
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="space-y-5" aria-label="Ejecuciones del piloto">
      <h2>Mis ejecuciones</h2>
      <p className="text-sm text-muted-foreground">
        Estado de tu cola de respuestas local, sin contenido de mensajes. Un
        resultado incierto requiere conciliación; no se reenvía automáticamente.
        Las aprobaciones creativas y de publicación siguen en sus secciones.
      </p>
      {loading ? (
        <p role="status">Cargando ejecuciones…</p>
      ) : error ? (
        <p role="alert">{error}</p>
      ) : !data.length ? (
        <p>
          Aún no tienes ejecuciones. Envía una consulta desde Conversaciones
          para crear una tarea local.
        </p>
      ) : (
        <ul className="divide-y">
          {data.map((item) => (
            <li
              key={item.id}
              className="flex flex-wrap items-center justify-between gap-3 py-4"
            >
              <div>
                <p className="font-medium">
                  Respuesta local · {item.id.slice(0, 8)}
                </p>
                <p>{states[item.state] || item.state}</p>
                <p className="text-sm text-muted-foreground">
                  {new Date(item.updated_at * 1000).toLocaleString("es-PE")} ·
                  Revisión {item.version}
                </p>
              </div>
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void inspect(item)}
              >
                Ver historial
              </Button>
            </li>
          ))}
        </ul>
      )}
      {detail && (
        <div className="space-y-3 border-t pt-5">
          <h3>Historial de la tarea</h3>
          <p>{states[detail.state] || detail.state}</p>
          <ol className="space-y-2">
            {detail.timeline.map((event) => (
              <li key={event.version} className="text-sm">
                Revisión {event.version} · {states[event.state] || event.state}{" "}
                · {new Date(event.at * 1000).toLocaleString("es-PE")}
              </li>
            ))}
          </ol>
          {detail.state === "pending" && (
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void act(detail, "cancel")}
              >
                Cancelar tarea
              </Button>
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void act(detail, "resume")}
              >
                Comprobar recuperación
              </Button>
            </div>
          )}
          {detail.state === "awaiting_review" &&
            ctx.user.role !== "viewer" &&
            ctx.user.role !== "editor" && (
              <div className="space-y-3">
                {proposal &&
                  proposal.id === detail.id &&
                  proposal.version === detail.version &&
                  proposal.proposal_digest === detail.proposal_digest && (
                    <div className="space-y-2">
                      <h4>Propuesta de respuesta local</h4>
                      <p className="break-all text-sm">
                        Destino: {proposal.destination}
                      </p>
                      <p className="whitespace-pre-wrap break-words rounded-lg border p-3">
                        {proposal.content}
                      </p>
                    </div>
                  )}
                <p>
                  La decisión se vincula a esta revisión exacta y su destino.
                  Aprobar aquí conserva el modo local sin entrega externa.
                </p>
                <div className="flex flex-wrap gap-2">
                  <Button
                    disabled={
                      busy ||
                      !proposal ||
                      proposal.id !== detail.id ||
                      proposal.version !== detail.version ||
                      proposal.proposal_digest !== detail.proposal_digest
                    }
                    onClick={() => void act(detail, "approve")}
                  >
                    Aprobar revisión local
                  </Button>
                  <Button
                    variant="outline"
                    disabled={busy}
                    onClick={() => void act(detail, "reject")}
                  >
                    Rechazar revisión
                  </Button>
                </div>
              </div>
            )}
          {detail.state === "uncertain" && (
            <p>
              No hay confirmación suficiente del efecto. No es seguro volver a
              enviarlo desde aquí.
            </p>
          )}
        </div>
      )}
    </section>
  );
}
