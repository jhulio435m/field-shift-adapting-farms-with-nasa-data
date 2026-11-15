import { useEffect, useState } from "react";
import { api, date } from "@/api";
import type { Context } from "@/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Confirmation, Empty } from "@/ui";

type Session = {
  id: string;
  user_id: string;
  display_name: string;
  username: string;
  created_at: number;
  expires_at: number;
  current: boolean;
};

export default function Sessions(ctx: Context) {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState<Session | null>(null);
  const owner = ctx.user.role === "owner";

  const refresh = async () => {
    const response = await api<{ sessions: Session[] }>("/v1/local/sessions");
    setSessions(response.sessions);
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
  }, [ctx.revision]);

  const revoke = () => {
    const target = pending;
    setPending(null);
    if (!target) return;
    void ctx.run(async () => {
      await api(
        `/v1/local/sessions/${encodeURIComponent(target.id)}`,
        "DELETE",
      );
      await refresh();
      ctx.setNotice("La sesión seleccionada dejó de tener acceso.");
    });
  };

  return (
    <div className="space-y-6">
      <p className="lead max-w-3xl">
        Revisa las sesiones que pueden acceder a este espacio. No recopilamos
        ubicación, dirección IP ni datos del navegador. Para cerrar esta sesión,
        usa el botón de salida.
      </p>
      <Card className="space-y-4 p-5">
        <div>
          <h2 className="text-lg font-semibold">Sesiones activas</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {owner
              ? "Ves las sesiones activas de las cuentas de tu organización."
              : "Solo ves las sesiones de tu cuenta."}
          </p>
        </div>
        {loading ? (
          <div role="status" className="text-sm text-muted-foreground">
            Cargando sesiones…
          </div>
        ) : sessions.length === 0 ? (
          <Empty>No hay sesiones activas para mostrar.</Empty>
        ) : (
          <div className="space-y-3">
            {sessions.map((session) => (
              <div
                key={session.id}
                className="flex flex-col gap-3 rounded-lg border p-4 sm:flex-row sm:items-center sm:justify-between"
              >
                <div className="min-w-0 space-y-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium">
                      {owner
                        ? `${session.display_name} · ${session.username}`
                        : "Tu cuenta"}
                    </p>
                    {session.current && (
                      <Badge variant="secondary">Esta sesión</Badge>
                    )}
                  </div>
                  <p className="text-sm text-muted-foreground">
                    Inició {date(session.created_at)} · vence{" "}
                    {date(session.expires_at)}
                  </p>
                </div>
                {session.current ? (
                  <Button variant="outline" disabled>
                    Sesión actual
                  </Button>
                ) : (
                  <Button variant="outline" onClick={() => setPending(session)}>
                    Revocar sesión
                  </Button>
                )}
              </div>
            ))}
          </div>
        )}
      </Card>
      <Confirmation
        open={pending !== null}
        onOpenChange={(open) => {
          if (!open) setPending(null);
        }}
        title="¿Revocar esta sesión?"
        description="La cuenta tendrá que volver a iniciar sesión en ese navegador. La sesión desde la que estás trabajando se mantiene activa."
        onConfirm={revoke}
      />
    </div>
  );
}
