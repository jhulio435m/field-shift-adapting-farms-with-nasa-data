import { useState } from "react";
import { api } from "@/api";
import type { Context } from "@/types";
import { Button } from "@/components/ui/button";
const decisions: Record<string, string> = {
  allowed: "Permitido",
  denied: "Denegado",
  stale: "Revisión obsoleta",
  invalid: "Entrada inválida",
  unavailable: "No disponible",
};
const scenarios: Record<string, string> = {
  allowed: "Lectura permitida",
  forbidden: "Sin concesión",
  revoked: "Acceso revocado",
  expired_connection: "Verificación vencida",
  stale_revision: "Revisión obsoleta",
  unknown_operation: "Operación desconocida",
  invalid_arguments: "Argumentos inválidos",
  provider_unavailable: "Servicio no disponible",
};
function caseLabel(id: string) {
  const split = id.indexOf("-");
  const name = id.slice(0, split);
  return `${name === "frappe" ? "ERP CONEIMERA" : name === "plane" ? "Plane" : "Telegram"}: ${scenarios[id.slice(split + 1)] || id}`;
}
type Result = {
  passed: number;
  total: number;
  cases: {
    id: string;
    question: string;
    kind: string;
    expected: string;
    actual: string;
    passed: boolean;
    evidence: string[];
  }[];
};
export default function IntegrationEvaluations(ctx: Context) {
  const [result, setResult] = useState<Result | null>(null),
    [busy, setBusy] = useState(false);
  async function run() {
    setBusy(true);
    try {
      await ctx.run(async () => {
        const created = await api<{ id: string; result: Result }>(
          "/v1/local/integration-evaluations",
          "POST",
          { suite_version: "1" },
        );
        setResult(created.result);
        ctx.setNotice(
          "Evaluación sintética guardada. No acredita conexiones ni calidad de un modelo de IA.",
        );
      });
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="space-y-5" aria-label="Evaluación de integraciones">
      <h2>Evaluación con datos ficticios</h2>
      <p className="text-sm text-muted-foreground">
        24 casos: ocho por conector, con permisos, revocación, revisión obsoleta
        y fallos. Prueba decisiones deterministas; no consulta proveedores ni
        evalúa respuestas de un modelo.
      </p>
      <Button disabled={busy} onClick={() => void run()}>
        {busy ? "Evaluando…" : "Ejecutar 24 casos"}
      </Button>
      {result && (
        <>
          <p role="status">
            {result.passed} de {result.total} casos correctos
          </p>
          <p className="text-sm text-muted-foreground">
            Los casos que necesitan revisión aparecen primero. Abre un caso para
            consultar su evidencia.
          </p>
          {["plane", "frappe", "telegram"].map((kind) => {
            const cases = result.cases.filter((item) => item.kind === kind);
            return (
              <section key={kind} className="space-y-3">
                <h3 className="font-semibold">
                  {kind === "plane"
                    ? "Plane"
                    : kind === "frappe"
                      ? "ERP CONEIMERA"
                      : "Telegram"}{" "}
                  · {cases.filter((item) => item.passed).length} de{" "}
                  {cases.length} correctos
                </h3>
                {cases
                  .toSorted((a, b) => Number(a.passed) - Number(b.passed))
                  .map((item) => (
                    <details key={item.id} className="border-b py-3">
                      <summary className="cursor-pointer rounded-sm focus-visible:outline-2 focus-visible:outline-ring">
                        {caseLabel(item.id)} ·{" "}
                        {item.passed ? "Correcto" : "Revisar"}
                      </summary>
                      <div className="space-y-2 pt-3 text-sm break-words">
                        <p>{item.question}</p>
                        <p>
                          Esperado: {decisions[item.expected] || item.expected}
                        </p>
                        <p>Obtenido: {decisions[item.actual] || item.actual}</p>
                        <p>
                          Evidencia ficticia:{" "}
                          {item.evidence.join(", ") || "Sin acceso a evidencia"}
                        </p>
                      </div>
                    </details>
                  ))}
              </section>
            );
          })}
        </>
      )}
    </section>
  );
}
