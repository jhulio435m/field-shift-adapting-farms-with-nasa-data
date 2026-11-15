import { useEffect, useState } from "react";
import { api, type User } from "@/api";
import type { Context } from "@/types";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Empty } from "@/ui";

type Resource = {
  resource_ref: string;
  display_label: string;
  granted: boolean;
};
type Grant = {
  user_id: string;
  user_name: string;
  resource_ref: string;
  resource_label: string;
  created_at: number;
};
type Result = {
  capability: string;
  simulated: boolean;
  result: { display_label: string; summary: string };
};
type Audit = {
  actor_id: string;
  capability: string;
  resource_digest: string;
  allowed: boolean;
  adapter_invoked: boolean;
  created_at: number;
};

export default function ResourceAccess(ctx: Context) {
  const [resources, setResources] = useState<Resource[]>([]),
    [grants, setGrants] = useState<Grant[]>([]),
    [users, setUsers] = useState<User[]>([]),
    [audit, setAudit] = useState<Audit[]>([]),
    [selectedUser, setSelectedUser] = useState(""),
    [selectedResource, setSelectedResource] = useState(""),
    [confirmed, setConfirmed] = useState(false),
    [result, setResult] = useState<Result | null>(null),
    [loading, setLoading] = useState(true);
  const owner = ctx.user.role === "owner";
  const viewers = users.filter(
    (user) => user.role === "viewer" && !user.disabled,
  );
  const refresh = async () => {
    const list = await api<{ resources: Resource[] }>(
      "/v1/local/resource-access/resources",
    );
    setResources(list.resources);
    if (owner) {
      const [g, u, a] = await Promise.all([
        api<{ grants: Grant[] }>("/v1/local/resource-access/grants"),
        api<User[]>("/v1/local/users"),
        api<{ events: Audit[] }>("/v1/local/resource-access/audit"),
      ]);
      setGrants(g.grants);
      setUsers(u);
      setAudit(a.events);
    }
  };
  useEffect(() => {
    void ctx.run(async () => {
      setLoading(true);
      try {
        await refresh();
      } finally {
        setLoading(false);
      }
    });
  }, [ctx.revision, ctx.user.role]);
  useEffect(() => {
    if (!selectedUser && viewers[0]) setSelectedUser(viewers[0].id);
  }, [selectedUser, users]);
  useEffect(() => {
    if (!selectedResource && resources[0])
      setSelectedResource(resources[0].resource_ref);
  }, [selectedResource, resources]);
  const grant = () => {
    if (!selectedUser || !selectedResource || !confirmed) return;
    void ctx.run(async () => {
      await api("/v1/local/resource-access/grants", "POST", {
        user_id: selectedUser,
        resource_ref: selectedResource,
        authorization_verified: true,
      });
      setConfirmed(false);
      await refresh();
    });
  };
  const revoke = (item: Grant) =>
    void ctx.run(async () => {
      await api(
        `/v1/local/resource-access/grants/${encodeURIComponent(item.user_id)}/${encodeURIComponent(item.resource_ref)}`,
        "DELETE",
      );
      await refresh();
    });
  const read = (resource: string) =>
    void ctx.run(async () => {
      setResult(null);
      const response = await api<Result>(
        "/v1/local/resource-access/execute",
        "POST",
        { resource_ref: resource },
      );
      setResult(response);
      await refresh();
    });
  return (
    <div className="space-y-6">
      <p className="lead max-w-3xl">
        {owner
          ? "Administra el acceso a recursos ficticios por cuenta. Las consultas se autorizan para el recurso exacto antes de leerlo."
          : "Consulta recursos ficticios que el propietario autorizó para tu cuenta. Cada lectura se valida con tu sesión."}
      </p>
      {loading ? (
        <Card className="p-5" role="status">
          Cargando permisos de demostración…
        </Card>
      ) : (
        <>
          {owner && (
            <Card className="space-y-4 p-5">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h2 className="text-lg font-semibold">
                    Conceder acceso de lectura
                  </h2>
                  <p className="mt-1 text-sm text-muted-foreground">
                    El permiso se limita a una cuenta y un recurso de esta
                    organización.
                  </p>
                </div>
                <Badge variant="secondary">Solo datos ficticios</Badge>
              </div>
              {!viewers.length ? (
                <Empty>
                  Crea primero una cuenta con rol Lector en Usuarios.
                </Empty>
              ) : (
                <div className="grid gap-4 md:grid-cols-2">
                  <Label className="grid gap-2">
                    Cuenta lectora
                    <NativeSelect
                      aria-label="Cuenta lectora"
                      value={selectedUser}
                      onChange={(e) => setSelectedUser(e.target.value)}
                    >
                      {viewers.map((user) => (
                        <option key={user.id} value={user.id}>
                          {user.display_name} · {user.username}
                        </option>
                      ))}
                    </NativeSelect>
                  </Label>
                  <Label className="grid gap-2">
                    Recurso
                    <NativeSelect
                      aria-label="Recurso"
                      value={selectedResource}
                      onChange={(e) => setSelectedResource(e.target.value)}
                    >
                      {resources.map((item) => (
                        <option
                          key={item.resource_ref}
                          value={item.resource_ref}
                        >
                          {item.display_label}
                        </option>
                      ))}
                    </NativeSelect>
                  </Label>
                  <label className="flex items-start gap-3 text-sm md:col-span-2">
                    <Checkbox
                      checked={confirmed}
                      onCheckedChange={(value) => setConfirmed(value === true)}
                      aria-label="Confirmar revisión de autorización"
                    />
                    <span>
                      Confirmo que revisé la autorización fuera del sistema.
                      Esta acción de demostración no concede acceso a datos
                      reales.
                    </span>
                  </label>
                  <Button
                    className="md:col-span-2 md:justify-self-start"
                    disabled={!selectedUser || !selectedResource || !confirmed}
                    onClick={grant}
                  >
                    Conceder permiso
                  </Button>
                </div>
              )}
            </Card>
          )}
          <Card className="space-y-4 p-5">
            <div>
              <h2 className="text-lg font-semibold">
                {owner ? "Catálogo ficticio" : "Recursos disponibles"}
              </h2>
              <p className="mt-1 text-sm text-muted-foreground">
                {owner
                  ? "Inicia sesión con una cuenta Lector autorizada para hacer una consulta."
                  : "El acceso se vuelve a comprobar en el servidor en cada lectura."}
              </p>
            </div>
            {!resources.length ? (
              <Empty>
                No tienes recursos concedidos. Pide al propietario que te
                otorgue acceso.
              </Empty>
            ) : (
              <div className="grid gap-3 md:grid-cols-2">
                {resources.map((item) => (
                  <div
                    key={item.resource_ref}
                    className="flex items-center justify-between gap-3 rounded-lg border p-4"
                  >
                    <div>
                      <p className="font-medium">{item.display_label}</p>
                      <p className="text-xs text-muted-foreground">
                        {item.resource_ref}
                      </p>
                    </div>
                    {!owner && (
                      <Button onClick={() => read(item.resource_ref)}>
                        Leer demostración
                      </Button>
                    )}
                  </div>
                ))}
              </div>
            )}
            {result && (
              <div
                role="status"
                className="rounded-lg border border-primary/30 bg-muted/30 p-4"
              >
                <div className="flex items-center gap-2">
                  <h3 className="font-semibold">
                    {result.result.display_label}
                  </h3>
                  <Badge variant="secondary">Simulado</Badge>
                </div>
                <p className="mt-2 text-sm text-muted-foreground">
                  {result.result.summary}
                </p>
              </div>
            )}
          </Card>
          {owner && (
            <>
              <Card className="space-y-3 p-5">
                <h2 className="text-lg font-semibold">Permisos vigentes</h2>
                {!grants.length ? (
                  <Empty>Aún no hay permisos concedidos.</Empty>
                ) : (
                  grants.map((item) => (
                    <div
                      key={`${item.user_id}:${item.resource_ref}`}
                      className="flex flex-wrap items-center justify-between gap-3 border-b pb-3 last:border-0"
                    >
                      <p className="text-sm">
                        {item.user_name} · {item.resource_label}
                      </p>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => revoke(item)}
                      >
                        Revocar
                      </Button>
                    </div>
                  ))
                )}
              </Card>
              <Card className="space-y-3 p-5">
                <h2 className="text-lg font-semibold">
                  Actividad de autorización
                </h2>
                {!audit.length ? (
                  <Empty>
                    Las decisiones aparecerán aquí sin argumentos ni resultados.
                  </Empty>
                ) : (
                  audit.slice(0, 20).map((event, index) => (
                    <div
                      key={`${event.created_at}:${index}`}
                      className="flex flex-wrap items-center justify-between gap-2 border-b pb-2 text-sm last:border-0"
                    >
                      <span>
                        {event.allowed ? "Permitida" : "Denegada"} ·{" "}
                        {event.capability}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {event.adapter_invoked
                          ? "Lector simulado ejecutado"
                          : "Lector no invocado"}{" "}
                        · {new Date(event.created_at * 1000).toLocaleString()}
                      </span>
                    </div>
                  ))
                )}
              </Card>
            </>
          )}
        </>
      )}
    </div>
  );
}
