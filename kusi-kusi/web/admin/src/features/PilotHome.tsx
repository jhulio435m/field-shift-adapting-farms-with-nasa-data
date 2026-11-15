import { useEffect, useState } from "react";
import { api } from "@/api";
import type { Context } from "@/types";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { useManagedData } from "./managed-data";

type Kind = "plane" | "frappe" | "telegram";
const names: Record<Kind, string> = {
  plane: "Plane",
  frappe: "ERP CONEIMERA",
  telegram: "Telegram",
};
type Profile = { kind: Kind; configured: boolean; revision: number };
type Resource = {
  resource_ref: string;
  display_label: string;
  granted: boolean;
};
type QueryResult = { source: string; summary: string; evidence: string[] };

export default function PilotHome(ctx: Context) {
  const owner = ctx.user.role === "owner";
  const resources = useManagedData<{ resources: Resource[] }>(
    "/v1/local/resource-access/resources",
    ctx.revision,
    { resources: [] },
  );
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [profilesError, setProfilesError] = useState("");
  const [profilesLoading, setProfilesLoading] = useState(owner);
  const [source, setSource] = useState("demo");
  const [resource, setResource] = useState("demo-resource-001");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState<QueryResult | null>(null);
  useEffect(() => {
    if (!owner) return;
    let active = true;
    setProfilesLoading(true);
    void api<Profile[]>("/v1/local/connection-settings")
      .then((data) => {
        if (active) {
          setProfiles(data);
          setProfilesError("");
        }
      })
      .catch(() => {
        if (active)
          setProfilesError(
            "No se pudo comprobar la configuración. Abre Conexiones para volver a intentarlo.",
          );
      })
      .finally(() => {
        if (active) setProfilesLoading(false);
      });
    return () => {
      active = false;
    };
  }, [owner, ctx.revision]);
  const selectedResource = resources.data.resources.find(
    (item) => item.resource_ref === resource,
  );
  const profile = profiles.find((item) => item.kind === source);
  const available =
    source === "demo" ? selectedResource?.granted : profile?.configured;
  async function query() {
    if (busy || !available) return;
    setBusy(true);
    setError("");
    setResult(null);
    try {
      if (source === "demo") {
        const response = await api<{
          result: { display_label: string; summary: string };
        }>("/v1/local/resource-access/execute", "POST", {
          resource_ref: resource,
        });
        setResult({
          source: response.result.display_label,
          summary: response.result.summary,
          evidence: [resource],
        });
      } else if (profile) {
        const status = await api<{
          status: string;
          mode: string;
          profile_revision: number;
          expires_at: number | null;
        }>(`/v1/local/connection-settings/${profile.kind}/status`);
        if (
          status.status !== "verified" ||
          status.mode !== "fixture" ||
          status.profile_revision !== profile.revision ||
          (status.expires_at ?? 0) <= Date.now() / 1000
        ) {
          setError(
            "Primero simula la verificación de esta configuración en Conexiones. Si cambiaste los datos o venció la prueba, vuelve a verificar.",
          );
          return;
        }
        const response = await api<{ result: { evidence: string[] } }>(
          "/v1/local/managed-queries",
          "POST",
          {
            kind: profile.kind,
            version: "1",
            operation: `${profile.kind}.${profile.kind === "telegram" ? "context" : "resource"}.read`,
            input: { resource_ref: "fixture-a" },
            expected_revision: profile.revision,
          },
        );
        setResult({
          source: names[profile.kind],
          summary:
            "Lectura del recurso ficticio autorizada. El servidor comprobó la identidad vinculada, el permiso y la configuración vigente.",
          evidence: response.result.evidence,
        });
      }
    } catch {
      setError(
        source === "demo"
          ? "No se pudo leer el recurso. Revisa tu acceso y vuelve a intentarlo; los permisos pueden haber cambiado."
          : "No se pudo completar la consulta. Comprueba la verificación en Conexiones. La identidad vinculada y el permiso del conector deben estar preparados por un administrador; guardar credenciales no los concede.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="space-y-8">
      <div className="max-w-2xl space-y-3">
        <h2 className="text-2xl font-semibold">
          Tu primera consulta, paso a paso
        </h2>
        <p>
          Prepara tus sistemas y comprueba qué información puedes consultar. Hoy
          las pruebas usan datos ficticios: no leen Plane ni el ERP real y no
          envían mensajes a Telegram.
        </p>
      </div>
      <ol className="divide-y border-y">
        <li className="flex flex-wrap items-center justify-between gap-3 py-4">
          <div>
            <h3 className="font-semibold">1. Prepara tus sistemas</h3>
            <p className="text-sm text-muted-foreground">
              {owner
                ? profilesLoading
                  ? "Comprobando configuración…"
                  : profilesError ||
                    `${profiles.filter((p) => p.configured).length} de 3 configuraciones con credenciales guardadas. Después debes simular su verificación.`
                : "El propietario configura los sistemas. Puedes comenzar con una demostración autorizada."}
            </p>
          </div>
          {owner && (
            <Button variant="outline" onClick={() => ctx.go("connections")}>
              Configurar sistemas
            </Button>
          )}
        </li>
        <li className="flex flex-wrap items-center justify-between gap-3 py-4">
          <div>
            <h3 className="font-semibold">2. Revisa tu acceso</h3>
            <p className="text-sm text-muted-foreground">
              La demostración usa permisos locales. Los conectores requieren
              además identidad vinculada y permisos específicos del servidor.
            </p>
          </div>
          <Button variant="outline" onClick={() => ctx.go("resourceAccess")}>
            Revisar acceso de demostración
          </Button>
        </li>
        <li className="py-4">
          <h3 className="font-semibold">3. Prueba una consulta</h3>
          <p className="text-sm text-muted-foreground">
            Elige una fuente abajo. Verás el resultado y de dónde procede.
          </p>
        </li>
      </ol>
      <Card className="panel">
        <h2>Consulta de prueba</h2>
        <form
          className="max-w-2xl space-y-5"
          onSubmit={(event) => {
            event.preventDefault();
            void query();
          }}
        >
          <div className="space-y-2">
            <Label htmlFor="query-source">Fuente</Label>
            <NativeSelect
              id="query-source"
              value={source}
              disabled={busy}
              onChange={(event) => {
                setSource(event.target.value);
                setResult(null);
                setError("");
              }}
            >
              <option value="demo">Demostración local</option>
              {owner &&
                (Object.keys(names) as Kind[]).map((kind) => (
                  <option key={kind} value={kind}>
                    {names[kind]} · datos ficticios
                  </option>
                ))}
            </NativeSelect>
          </div>
          {source === "demo" ? (
            <div className="space-y-2">
              <Label htmlFor="query-resource">Recurso de demostración</Label>
              <NativeSelect
                id="query-resource"
                value={resource}
                disabled={busy || resources.loading}
                onChange={(event) => {
                  setResource(event.target.value);
                  setResult(null);
                  setError("");
                }}
              >
                {resources.data.resources.map((item) => (
                  <option key={item.resource_ref} value={item.resource_ref}>
                    {item.display_label}
                    {!item.granted ? " · sin acceso" : ""}
                  </option>
                ))}
              </NativeSelect>
              {resources.loading && <p role="status">Cargando recursos…</p>}
              {resources.error && (
                <p role="alert">
                  No se pudieron cargar los recursos. Actualiza la página para
                  volver a intentarlo.
                </p>
              )}
            </div>
          ) : (
            <p className="text-sm">
              Recurso: registro ficticio del conector. Solo se permite la
              lectura; no se modifican datos.
            </p>
          )}
          {!available && !resources.loading && (
            <p className="text-sm">
              {source === "demo"
                ? "No tienes acceso a este recurso. Selecciona otro o solicita al propietario un permiso de demostración."
                : "Guarda primero las credenciales y simula su verificación en Conexiones."}
            </p>
          )}
          <Button
            type="submit"
            disabled={busy || !available || resources.loading}
          >
            {busy ? "Consultando…" : "Consultar datos ficticios"}
          </Button>
        </form>
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        )}
        {result && (
          <section
            className="space-y-3 border-t pt-5"
            aria-label="Resultado de la consulta"
          >
            <h3 className="font-semibold" role="status">
              Consulta completada · datos ficticios
            </h3>
            <p>{result.summary}</p>
            <p className="text-sm">Fuente: {result.source}</p>
            <details>
              <summary className="cursor-pointer">
                Ver evidencia de la lectura
              </summary>
              <ul className="mt-3 list-disc space-y-2 pl-5 text-sm break-all">
                {result.evidence.map((value) => (
                  <li key={value}>{value}</li>
                ))}
              </ul>
            </details>
            <Button variant="outline" onClick={() => ctx.go("pilot", "memory")}>
              Revisar mi contexto
            </Button>
            <p className="text-sm text-muted-foreground">
              Solo las lecturas de demostración pueden conservar contexto, si lo
              autorizaste previamente. Las consultas no crean envíos ni tareas
              de mensajería.
            </p>
          </section>
        )}
      </Card>
      <p className="max-w-2xl text-sm text-muted-foreground">
        Activación real pendiente. Este entorno no permite verificar proveedores
        remotos ni activar envíos. Guardar o probar una configuración no cambia
        ese estado.
      </p>
    </div>
  );
}
