import { useEffect, useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/api";
import { Badge, Confirmation } from "@/ui";
import type { Context } from "@/types";

type Kind = "plane" | "frappe" | "telegram";
type Verification = {
  id: string | null;
  profile_revision: number;
  status: string;
  mode: "fixture" | "provider" | null;
  checked_at: number | null;
  expires_at: number | null;
  scope_summary: string;
  error_code: string | null;
  runtime_active: boolean;
};
type Manifest = {
  kind: Kind;
  version: string;
  operations: string[];
  timeout_ms: number;
  auth_fields: {
    name: string;
    label: string;
    secret: boolean;
    required: boolean;
  }[];
};
type Profile = {
  kind: Kind;
  label: string;
  base_url: string | null;
  workspace: string | null;
  bot_username: string | null;
  webhook_url: string | null;
  revision: number;
  credential_fields: string[];
  configured: boolean;
  runtime_active: false;
};
const names: Record<Kind, string> = {
  plane: "Plane",
  frappe: "ERP CONEIMERA",
  telegram: "Telegram",
};
const kinds: Kind[] = ["plane", "frappe", "telegram"];

function Field({
  name,
  title,
  value,
  secret = false,
  required = false,
  hint,
  type = "text",
  maxLength = 2048,
}: {
  name: string;
  title: string;
  value?: string | null;
  secret?: boolean;
  required?: boolean;
  hint?: string;
  type?: string;
  maxLength?: number;
}) {
  const id = "connection-" + name;
  return (
    <div className="space-y-2">
      <Label htmlFor={id} className="items-start text-left">
        {title}
      </Label>
      <Input
        id={id}
        name={name}
        type={secret ? "password" : type}
        defaultValue={secret ? "" : value || ""}
        required={required}
        autoComplete={secret ? "new-password" : "off"}
        spellCheck={false}
        maxLength={secret ? 4096 : maxLength}
        aria-describedby={hint ? id + "-hint" : undefined}
      />
      {hint && (
        <p id={id + "-hint"} className="text-sm text-muted-foreground">
          {hint}
        </p>
      )}
    </div>
  );
}

export default function ConnectionSettings(ctx: Context) {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [verifications, setVerifications] = useState<
    Partial<Record<Kind, Verification>>
  >({});
  const [manifests, setManifests] = useState<Manifest[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [selected, setSelected] = useState<{
    kind: Kind;
    profile?: Profile;
  } | null>(null);
  const [clearing, setClearing] = useState<Profile | null>(null);
  const owner = ctx.user.role === "owner";
  useEffect(() => {
    if (!owner) {
      setLoading(false);
      return;
    }
    let active = true;
    setLoading(true);
    void Promise.all([
      api<Profile[]>("/v1/local/connection-settings"),
      api<Manifest[]>("/v1/local/connector-manifests"),
    ])
      .then(async ([data, definitions]) => {
        const states = await Promise.all(
          data.map(
            async (profile) =>
              [
                profile.kind,
                await api<Verification>(
                  `/v1/local/connection-settings/${profile.kind}/status`,
                ),
              ] as const,
          ),
        );
        if (active) {
          setProfiles(data);
          setManifests(definitions);
          setVerifications(Object.fromEntries(states));
          setError("");
        }
      })
      .catch((cause: unknown) => {
        if (active)
          setError(
            cause instanceof Error
              ? cause.message
              : "No se pudo cargar la configuración.",
          );
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [ctx.revision, owner]);

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selected || busy) return;
    const form = event.currentTarget;
    const data = new FormData(form);
    const value = (name: string) => String(data.get(name) || "");
    const kind = selected.kind;
    const credentials: Record<string, string> = {};
    for (const key of [
      "api_key",
      "api_secret",
      "bot_token",
      "webhook_secret",
    ]) {
      const secret = value(key);
      if (secret) credentials[key] = secret;
      // Clear secrets from the DOM before awaiting the request, also on error.
      const input = form.elements.namedItem(key);
      if (input instanceof HTMLInputElement) input.value = "";
    }
    const body = {
      label: value("label").trim(),
      base_url: kind === "telegram" ? null : value("base_url").trim(),
      workspace: kind === "plane" ? value("workspace").trim() || null : null,
      bot_username:
        kind === "telegram"
          ? value("bot_username").trim().replace(/^@/, "") || null
          : null,
      webhook_url:
        kind === "telegram" ? value("webhook_url").trim() || null : null,
      expected_revision: selected.profile?.revision ?? null,
      credentials,
    };
    setBusy(true);
    setError("");
    try {
      const result = await api<Profile>(
        "/v1/local/connection-settings/" + kind,
        "PUT",
        body,
      );
      setProfiles((previous) => [
        ...previous.filter((profile) => profile.kind !== kind),
        result,
      ]);
      setSelected(null);
      ctx.setNotice(
        "Configuración guardada. La conexión real sigue sin verificar ni activar.",
      );
      ctx.updated();
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : "No se pudo guardar. Vuelve a introducir las credenciales nuevas.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function check(profile: Profile) {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      const result = await api<Verification>(
        `/v1/local/connection-settings/${profile.kind}/verify`,
        "POST",
        { expected_revision: profile.revision, mode: "fixture" },
      );
      setVerifications((previous) => ({ ...previous, [profile.kind]: result }));
      ctx.setNotice(
        result.status === "verified"
          ? "Simulación completada. La identidad y los permisos reales siguen sin comprobarse."
          : "Faltan credenciales. Edita la conexión antes de simular.",
      );
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : "No se pudo simular. Actualiza la página e inténtalo de nuevo.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function clear() {
    if (!clearing || busy) return;
    setBusy(true);
    setError("");
    try {
      const result = await api<Profile>(
        `/v1/local/connection-settings/${clearing.kind}/credentials`,
        "DELETE",
        { expected_revision: clearing.revision },
      );
      setProfiles((previous) => [
        ...previous.filter((profile) => profile.kind !== result.kind),
        result,
      ]);
      ctx.setNotice("Credenciales eliminadas de esta configuración local.");
      ctx.updated();
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : "No se pudieron borrar las credenciales.",
      );
    } finally {
      setBusy(false);
      setClearing(null);
    }
  }

  return (
    <>
      <Card className="panel">
        <div className="space-y-2">
          <h2>Sistemas y canal del piloto</h2>
          <p className="text-sm text-muted-foreground">
            Configura Plane, ERP CONEIMERA y Telegram desde aquí. Guardar no
            prueba la conexión ni activa envíos.
          </p>
        </div>
        {!owner ? (
          <p className="text-sm text-muted-foreground">
            Solo el propietario puede consultar y administrar estas
            credenciales.
          </p>
        ) : (
          <>
            {error && !selected && (
              <p role="alert" className="text-sm text-destructive">
                {error}
              </p>
            )}
            {loading ? (
              <p role="status" className="text-sm text-muted-foreground">
                Cargando configuración…
              </p>
            ) : (
              <div className="divide-y">
                {kinds.map((kind) => {
                  const profile = profiles.find(
                    (profile) => profile.kind === kind,
                  );
                  const state = verifications[kind];
                  const manifest = manifests.find((item) => item.kind === kind);
                  const simulated =
                    state?.mode === "fixture" &&
                    state?.status === "verified" &&
                    state.profile_revision === profile?.revision &&
                    (state.expires_at ?? 0) > Date.now() / 1000;
                  return (
                    <section
                      key={kind}
                      aria-label={names[kind]}
                      className="flex flex-col items-start justify-between gap-4 py-5 sm:flex-row"
                    >
                      <div className="min-w-0 flex-1 space-y-2">
                        <h3 className="font-semibold">{names[kind]}</h3>
                        {profile && profile.label !== names[kind] && (
                          <p className="text-sm">{profile.label}</p>
                        )}
                        <p className="break-all text-sm text-muted-foreground">
                          {profile?.base_url ||
                            (profile?.bot_username
                              ? "@" + profile.bot_username
                              : "Sin configurar")}
                        </p>
                        {profile && (
                          <div className="flex flex-wrap gap-2">
                            <Badge
                              value={
                                profile.configured
                                  ? "Credenciales guardadas"
                                  : "Faltan credenciales"
                              }
                            />
                            <Badge
                              value={
                                simulated
                                  ? "Simulación vigente"
                                  : "Sin verificar"
                              }
                            />
                          </div>
                        )}
                        {manifest && (
                          <p className="text-sm text-muted-foreground">
                            Conector v{manifest.version} · Operaciones de
                            lectura · Límite {manifest.timeout_ms / 1000} s
                          </p>
                        )}
                        {simulated && state?.checked_at && (
                          <p className="text-sm text-muted-foreground">
                            Simulación:{" "}
                            {new Date(state.checked_at * 1000).toLocaleString(
                              "es-PE",
                            )}
                            . Solo comprueba campos guardados; no valida el
                            token ni permisos remotos.
                          </p>
                        )}
                        {profile && (
                          <p className="text-sm text-muted-foreground">
                            Revisión {profile.revision} · Conexión real inactiva
                          </p>
                        )}
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <Button
                          variant="outline"
                          disabled={busy}
                          aria-label={`Configurar ${names[kind]}`}
                          onClick={() => {
                            setError("");
                            setSelected({ kind, profile });
                          }}
                        >
                          {profile ? "Editar" : "Configurar"}
                        </Button>
                        {profile && (
                          <Button
                            variant="outline"
                            disabled={busy || !profile.configured}
                            aria-label={`Simular verificación de ${names[kind]}`}
                            onClick={() => void check(profile)}
                          >
                            Simular verificación
                          </Button>
                        )}
                        {!!profile?.credential_fields.length && (
                          <Button
                            variant="ghost"
                            disabled={busy}
                            aria-label={`Borrar credenciales de ${names[kind]}`}
                            onClick={() => setClearing(profile)}
                          >
                            Borrar credenciales
                          </Button>
                        )}
                      </div>
                    </section>
                  );
                })}
              </div>
            )}
          </>
        )}
        <p className="text-sm text-muted-foreground">
          Las credenciales se cifran en el servidor y no vuelven a mostrarse.
          Este workspace no contacta servicios externos.
        </p>
      </Card>
      <Button variant="outline" className="my-4" onClick={() => ctx.go("home")}>
        Continuar con una consulta de prueba
      </Button>
      <Dialog
        open={!!selected}
        onOpenChange={(open) => {
          if (!open && !busy) {
            setSelected(null);
            setError("");
          }
        }}
        title={
          selected
            ? `Configurar ${names[selected.kind]}`
            : "Configurar conexión"
        }
        description="Guarda los datos de esta organización. Deja una credencial vacía para conservar la existente."
      >
        {selected && (
          <form className="space-y-5" onSubmit={(event) => void save(event)}>
            <Field
              name="label"
              title="Nombre de la conexión"
              maxLength={120}
              value={selected.profile?.label || names[selected.kind]}
              required
            />
            {selected.kind !== "telegram" && (
              <Field
                name="base_url"
                title="URL de la instancia"
                type="url"
                value={selected.profile?.base_url}
                required
                hint="Usa el origen HTTPS, sin /api, parámetros ni credenciales en la URL."
              />
            )}
            {selected.kind === "plane" && (
              <Field
                name="workspace"
                title="Espacio de trabajo de Plane (opcional)"
                maxLength={120}
                value={selected.profile?.workspace}
                hint="Identificador del espacio, por ejemplo equipo-piloto."
              />
            )}
            {manifests
              .find((item) => item.kind === selected.kind)
              ?.auth_fields.filter((field) => field.name !== "webhook_secret")
              .map((field) => (
                <Field
                  key={field.name}
                  name={field.name}
                  title={field.label}
                  secret={field.secret}
                  hint={
                    selected.profile?.credential_fields.includes(field.name)
                      ? "Guardada. Vacío conserva el valor; uno nuevo lo reemplaza."
                      : "Solo se envía al guardar; nunca vuelve a mostrarse."
                  }
                />
              ))}
            {selected.kind === "telegram" && (
              <>
                <Field
                  name="bot_username"
                  title="Usuario del bot (opcional)"
                  maxLength={32}
                  value={selected.profile?.bot_username}
                  hint="Nombre del bot, sin @."
                />
                <Field
                  name="webhook_url"
                  title="URL pública del webhook (opcional)"
                  type="url"
                  value={selected.profile?.webhook_url}
                  hint="Se guarda como configuración pendiente; este modo no registra el webhook en Telegram."
                />
                <Field
                  name="webhook_secret"
                  title="Secreto de webhook (opcional)"
                  secret
                  hint="Se genera en el servidor al guardar el primer token. Vacío conserva el existente; un valor nuevo lo reemplaza."
                />
              </>
            )}
            {error && (
              <p role="alert" className="text-sm text-destructive">
                {error} Si hubo un conflicto, cierra y actualiza antes de editar
                otra vez.
              </p>
            )}
            <Button type="submit" className="w-full" disabled={busy}>
              {busy ? "Guardando…" : "Guardar configuración"}
            </Button>
          </form>
        )}
      </Dialog>
      <Confirmation
        open={!!clearing}
        onOpenChange={(open) => {
          if (!open && !busy) setClearing(null);
        }}
        title="¿Borrar las credenciales guardadas?"
        description="Se conservarán los datos públicos de la conexión. Esto no revoca tokens en el proveedor ni modifica canales activos del servidor."
        onConfirm={() => void clear()}
      />
    </>
  );
}
