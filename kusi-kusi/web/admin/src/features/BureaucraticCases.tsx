"use client";

import { useEffect, useState } from "react";
import { api } from "@/api";
import type { Context } from "@/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";

type CaseSummary = { id: string; title: string; status: string };
type Source = {
  key: string;
  title: string;
  official_url: string;
  version: string;
  validity: string;
  page_or_section: string;
  citation_excerpt: string;
};
type Request = {
  id: string;
  version: number;
  recipient_unit: string;
  body: string;
  digest: string;
  state: string;
  consented: boolean;
};
type Detail = {
  id: string;
  tracking_code: string;
  title: string;
  status: string;
  is_requester: boolean;
  is_assigned_staff: boolean;
  payment_amount_cents: number;
  payment_concept: string;
  payment_reference_masked: string;
  health_interruption_summary: string;
  sources: Source[];
  request: Request | null;
  responses: {
    response_type: string;
    body: string;
    actor_label: string;
    citation_keys: string[];
  }[];
};
type Guidance = {
  summary: string;
  next_steps: string[];
  open_questions: string[];
  citations: Source[];
  assistant_mode: string;
};

export default function BureaucraticCases({
  user,
  revision,
  updated,
}: Context) {
  const [cases, setCases] = useState<CaseSummary[]>([]);
  const [detail, setDetail] = useState<Detail | null>(null);
  const [guidance, setGuidance] = useState<Guidance | null>(null);
  const [recipient, setRecipient] = useState("Tesorería - UNCP");
  const [draft, setDraft] = useState("");
  const [response, setResponse] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const selectedId = cases[0]?.id;

  useEffect(() => {
    let active = true;
    api<{ cases: CaseSummary[] }>("/v1/local/cases")
      .then((data) => {
        if (active) setCases(data.cases);
      })
      .catch((e: unknown) => {
        if (active)
          setError(
            e instanceof Error
              ? e.message
              : "No se pudieron cargar los trámites.",
          );
      });
    return () => {
      active = false;
    };
  }, [revision]);

  useEffect(() => {
    if (!selectedId) return;
    let active = true;
    api<{ case: Detail }>(`/v1/local/cases/${selectedId}`)
      .then(({ case: value }) => {
        if (active) {
          setDetail(value);
          setDraft(
            value.request?.body ??
              "Solicito orientación para retomar el trámite de titulación. El pago asociado a la constancia de expedito PA57002335 fue realizado antes de que tuviera que interrumpir el proceso por motivos de salud. Agradeceré confirmar el estado del trámite y si el pago puede aplicarse a los pasos pendientes o corresponde seguir otro procedimiento.",
          );
          setRecipient(value.request?.recipient_unit ?? "Tesorería - UNCP");
        }
      })
      .catch((e: unknown) =>
        setError(
          e instanceof Error ? e.message : "No se pudo abrir el expediente.",
        ),
      );
    return () => {
      active = false;
    };
  }, [selectedId, revision]);

  const action = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await fn();
      updated();
    } catch (e) {
      setError(e instanceof Error ? e.message : "No se completó la acción.");
    } finally {
      setBusy(false);
    }
  };

  if (!detail)
    return (
      <section className="max-w-3xl space-y-4">
        <p className="text-muted-foreground">
          {error || "Preparando el expediente ficticio de la demostración…"}
        </p>
        {error && (
          <Button variant="outline" onClick={updated}>
            Reintentar
          </Button>
        )}
      </section>
    );

  return (
    <div className="grid gap-8 xl:grid-cols-[minmax(0,1.4fr)_minmax(18rem,.8fr)]">
      <section className="min-w-0 space-y-8">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <h2 className="text-xl font-semibold">{detail.title}</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Caso ficticio para la hackatón · sin envío real
            </p>
          </div>
          <p className="rounded-md bg-muted px-4 py-3 text-sm">
            Código de seguimiento de la demo:{" "}
            <strong>{detail.tracking_code}</strong> · en WhatsApp escribe este
            código para consultar el estado ficticio.
          </p>
          <Badge variant="secondary">
            {detail.status.replaceAll("_", " ")}
          </Badge>
        </div>
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        <section className="space-y-3">
          <h3 className="font-semibold">Entender qué ocurrió</h3>
          <p className="max-w-prose text-sm leading-6">
            El expediente de muestra registra un pago de{" "}
            <strong>S/ {(detail.payment_amount_cents / 100).toFixed(2)}</strong>{" "}
            para {detail.payment_concept}. El proceso se interrumpió por motivos
            de salud; no se solicita diagnóstico ni historia clínica.
          </p>
          <p className="text-sm text-muted-foreground">
            Referencia ficticia {detail.payment_reference_masked}. La vigencia,
            reutilización o devolución del pago requiere respuesta de la UNCP.
          </p>
          <Button
            disabled={busy}
            onClick={() =>
              void action(async () => {
                const result = await api<{ guidance: Guidance }>(
                  `/v1/local/cases/${detail.id}/guidance`,
                  "POST",
                  {
                    question:
                      "¿Cómo puedo retomar el trámite y qué debo confirmar sobre el pago?",
                  },
                );
                setGuidance(result.guidance);
              })
            }
          >
            Orientarme con fuentes
          </Button>
        </section>
        {guidance && (
          <section className="space-y-3 border-t pt-6">
            <h3 className="font-semibold">Orientación</h3>
            <p className="max-w-prose text-sm leading-6">{guidance.summary}</p>
            <h4 className="text-sm font-medium">Siguientes pasos</h4>
            <ol className="list-decimal space-y-2 pl-5 text-sm">
              {guidance.next_steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ol>
            <h4 className="text-sm font-medium">Puntos por confirmar</h4>
            <ul className="list-disc space-y-2 pl-5 text-sm">
              {guidance.open_questions.map((q) => (
                <li key={q}>{q}</li>
              ))}
            </ul>
            <p className="text-xs text-muted-foreground">
              Modo:{" "}
              {guidance.assistant_mode === "deterministic_demo"
                ? "demostración local"
                : "modelo local"}
              . La IA orienta; no decide ni confirma derechos.
            </p>
          </section>
        )}
        <section className="space-y-3 border-t pt-6">
          <h3 className="font-semibold">Fuentes oficiales</h3>
          {detail.sources.map((source) => (
            <article key={source.key} className="space-y-1 py-2">
              <div className="flex flex-wrap items-center gap-2">
                <a
                  className="font-medium underline underline-offset-4"
                  href={source.official_url}
                  target="_blank"
                  rel="noreferrer"
                >
                  {source.title} · {source.key}
                </a>
                <Badge variant="outline">
                  {source.validity.replaceAll("_", " ")}
                </Badge>
              </div>
              <p className="text-xs text-muted-foreground">
                {source.version} · {source.page_or_section}
              </p>
              <p className="max-w-prose text-sm">{source.citation_excerpt}</p>
            </article>
          ))}
        </section>
        {detail.is_requester && (
          <section className="space-y-3 border-t pt-6">
            <h3 className="font-semibold">Preparar una consulta</h3>
            <p className="text-sm text-muted-foreground">
              Revisa el texto y el destinatario. Guardar solo crea un borrador
              privado.
            </p>
            <label className="grid gap-2 text-sm">
              Área destinataria
              <input
                className="h-10 rounded-md border bg-background px-3"
                value={recipient}
                onChange={(e) => setRecipient(e.target.value)}
              />
            </label>
            <label className="grid gap-2 text-sm">
              Mensaje
              <textarea
                className="min-h-36 rounded-md border bg-background p-3 leading-6"
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
              />
            </label>
            <Button
              variant="outline"
              disabled={busy}
              onClick={() =>
                void action(async () => {
                  const result = await api<{ request: Request }>(
                    `/v1/local/cases/${detail.id}/requests`,
                    "POST",
                    {
                      recipient_unit: recipient,
                      body: draft,
                      include_receipt_fixture: true,
                    },
                  );
                  setDetail({ ...detail, request: result.request });
                })
              }
            >
              Guardar borrador
            </Button>
            {detail.request?.state === "draft" && (
              <Button
                className="ml-2"
                disabled={busy}
                onClick={() =>
                  void action(async () => {
                    const current = detail.request!;
                    const result = await api<{ request: Request }>(
                      `/v1/local/cases/${detail.id}/requests/${current.id}/approve`,
                      "POST",
                      {
                        version: current.version,
                        digest: current.digest,
                        confirm_share: true,
                      },
                    );
                    setDetail({ ...detail, request: result.request });
                  })
                }
              >
                Revisé y consiento esta versión
              </Button>
            )}
            {detail.request?.consented &&
              detail.request.state === "approved" && (
                <Button
                  className="ml-2"
                  disabled={busy}
                  onClick={() =>
                    void action(async () => {
                      const result = await api<{ request: Request }>(
                        `/v1/local/cases/${detail.id}/requests/${detail.request!.id}/simulate-send`,
                        "POST",
                      );
                      setDetail({ ...detail, request: result.request });
                    })
                  }
                >
                  Registrar envío simulado
                </Button>
              )}
            {detail.request?.state === "simulated_sent" && (
              <p role="status" className="text-sm font-medium">
                Envío simulado localmente. La consulta no llegó a la UNCP.
              </p>
            )}
          </section>
        )}
        {detail.responses.map((r, i) => (
          <section
            key={`${r.response_type}-${i}`}
            className="space-y-2 border-t pt-6"
          >
            <h3 className="font-semibold">
              Respuesta registrada · {r.actor_label}
            </h3>
            <p className="max-w-prose whitespace-pre-wrap text-sm leading-6">
              {r.body}
            </p>
            <p className="text-xs text-muted-foreground">
              Atribuida al personal que la registró; el sistema no la presenta
              como una decisión automática.
            </p>
          </section>
        ))}
      </section>
      <aside className="space-y-5 border-t pt-6 xl:border-l xl:border-t-0 xl:pl-7 xl:pt-0">
        <h3 className="font-semibold">Seguimiento administrativo</h3>
        {detail.is_assigned_staff &&
        (user.role === "owner" || user.role === "approver") ? (
          <>
            <p className="text-sm text-muted-foreground">
              Registra orientación, derivación o respuesta recibida. Identifica
              el área y cita la fuente; no marques como decisión institucional
              algo que no haya sido emitido por la UNCP.
            </p>
            <label className="grid gap-2 text-sm">
              Nota atribuida
              <textarea
                className="min-h-36 rounded-md border bg-background p-3 leading-6"
                value={response}
                onChange={(e) => setResponse(e.target.value)}
                placeholder="Área que respondió, fecha y contenido confirmado…"
              />
            </label>
            <Button
              variant="outline"
              disabled={busy || !response.trim()}
              onClick={() =>
                void action(async () => {
                  await api(`/v1/local/cases/${detail.id}/responses`, "POST", {
                    response_type: "guidance",
                    body: response,
                    citation_keys: ["PA57002335", "PA57002563"],
                  });
                  setResponse("");
                })
              }
            >
              Registrar respuesta del personal
            </Button>
          </>
        ) : (
          <p className="text-sm text-muted-foreground">
            La respuesta administrativa aparecerá aquí cuando una persona
            autorizada la registre.
          </p>
        )}
      </aside>
    </div>
  );
}
