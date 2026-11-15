import type { Context } from "@/types";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import ChannelOperations from "./ChannelOperations";
import ManagedExecutions from "./ManagedExecutions";
import IntegrationEvaluations from "./IntegrationEvaluations";
import AuthorizedMemory from "./AuthorizedMemory";
import PolicySimulation from "./PolicySimulation";
const sections = {
  executions: "Mis ejecuciones",
  channel: "Vínculos del canal",
  evaluations: "Evaluación",
  memory: "Mi contexto",
  policies: "Permisos",
};
type Section = keyof typeof sections;
export default function PilotOperations(ctx: Context) {
  const selected: Section = Object.hasOwn(sections, ctx.selected)
    ? (ctx.selected as Section)
    : "executions";
  const owner = ctx.user.role === "owner";
  const allowed = owner || selected === "executions" || selected === "memory";
  return (
    <>
      <p className="text-sm text-muted-foreground mb-5">
        Operación y pruebas locales del piloto. Plane, ERP CONEIMERA y Telegram
        siguen sin activación real.
      </p>
      <nav
        aria-label="Secciones del piloto"
        className="mb-5 flex flex-wrap gap-2"
      >
        {(Object.keys(sections) as Section[])
          .filter((key) => owner || key === "executions" || key === "memory")
          .map((key) => (
            <Button
              key={key}
              variant={selected === key ? "default" : "outline"}
              aria-current={selected === key ? "page" : undefined}
              onClick={() => ctx.go("pilot", key)}
            >
              {sections[key]}
            </Button>
          ))}
      </nav>
      <Card className="panel">
        {!allowed ? (
          <p>Tu rol no permite administrar esta sección.</p>
        ) : selected === "channel" ? (
          <ChannelOperations {...ctx} />
        ) : selected === "evaluations" ? (
          <IntegrationEvaluations {...ctx} />
        ) : selected === "memory" ? (
          <AuthorizedMemory {...ctx} />
        ) : selected === "policies" ? (
          <PolicySimulation {...ctx} />
        ) : (
          <ManagedExecutions {...ctx} />
        )}
      </Card>
    </>
  );
}
