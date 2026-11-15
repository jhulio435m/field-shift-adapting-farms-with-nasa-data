import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import { Card } from "@/components/ui/card";
import React, { useState, useEffect } from "react";
import { api, date } from "../api";
import { Empty, Confirmation } from "../ui";
import type { Context } from "../types";
import { useList } from "../hooks";
export default function Operations(ctx: Context) {
  const [overview, setOverview] = useState<Record<string, number>>({}),
    [retention, setRetention] = useState<unknown>(null),
    [days, setDays] = useState(90),
    [confirmRetention, setConfirmRetention] = useState(false);
  const activity = useList<{
    at: number;
    action: string;
    target: string;
    actor: string;
  }>("/v1/local/activity", ctx, 5000);
  useEffect(() => {
    void ctx.run(async () => setOverview(await api("/v1/local/overview")));
  }, [ctx.revision]);
  return (
    <>
      <p className="lead">
        Estado de las colas, decisiones y accesos de esta organización.
      </p>
      <div className="stats">
        {[
          ["approval_jobs", "Por aprobar"],
          ["pending_messages", "Mensajes pendientes"],
          ["completed_jobs", "Trabajos completados"],
          ["publications_simulated", "Publicaciones simuladas"],
        ].map(([key, title]) => (
          <div key={key}>
            <span>{title}</span>
            <strong>{overview[key] ?? "—"}</strong>
          </div>
        ))}
      </div>
      <div className="connection-grid">
        <Card className="panel">
          <h2>Colas y almacenamiento</h2>
          <dl>
            {[
              ["assets", "Archivos"],
              ["users", "Usuarios activos"],
              ["failed_messages", "Mensajes fallidos"],
              ["pending_delivery", "Envíos externos retenidos"],
            ].map(([key, title]) => (
              <React.Fragment key={key}>
                <dt>{title}</dt>
                <dd>{overview[key] ?? "—"}</dd>
              </React.Fragment>
            ))}
          </dl>
          <p className="hint">
            Los trabajos y mensajes sobreviven a un reinicio gracias a
            PostgreSQL. Los envíos externos permanecen retenidos en este
            entorno.
          </p>
        </Card>
        <Card className="panel">
          <h2>Respaldo y recuperación</h2>
          <p>
            El respaldo incluye la base de datos, archivos privados y un
            manifiesto de integridad.
          </p>
          <code className="command">
            python3 scripts/local_backup.py --verify-restore
          </code>
          <p className="hint">
            Ejecútalo en el servidor. La verificación restaura en una base
            temporal y coteja los archivos. No incluye secretos de proveedores.
          </p>
        </Card>
        {ctx.user.role === "owner" && (
          <Card className="panel">
            <h2>Retención de mensajes</h2>
            <Label>
              Conservar los últimos días
              <Input
                type="number"
                min={1}
                max={3650}
                value={days}
                onChange={(e) => {
                  setDays(Number(e.target.value));
                  setRetention(null);
                }}
              />
            </Label>
            <Button
              variant="outline"
              onClick={() =>
                void ctx.run(async () =>
                  setRetention(
                    await api("/v1/local/retention/preview", "POST", { days }),
                  ),
                )
              }
            >
              Calcular alcance
            </Button>
            {retention !== null && (
              <>
                <pre>{JSON.stringify(retention, null, 2)}</pre>
                <Button
                  variant="outline"
                  className="danger"
                  onClick={() => setConfirmRetention(true)}
                >
                  Aplicar retención
                </Button>
              </>
            )}
          </Card>
        )}
      </div>
      <Card className="panel">
        <h2>Actividad reciente</h2>
        <div className="table-wrap">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Momento</TableHead>
                <TableHead>Acción</TableHead>
                <TableHead>Responsable</TableHead>
                <TableHead>Referencia</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {activity.map((a, i) => (
                <TableRow key={i}>
                  <TableCell>{date(a.at)}</TableCell>
                  <TableCell>{a.action}</TableCell>
                  <TableCell>{a.actor}</TableCell>
                  <TableCell className="mono">{a.target}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {!activity.length && <Empty>Aún no hay actividad registrada.</Empty>}
        </div>
      </Card>
      <Confirmation
        open={confirmRetention}
        onOpenChange={setConfirmRetention}
        title="Aplicar retención de mensajes"
        description={`Se eliminará el contenido de mensajes de hace más de ${days} días. Los identificadores de eventos se conservarán. Esta acción no se puede deshacer.`}
        onConfirm={() =>
          void ctx.run(async () => {
            const result = await api("/v1/local/retention/apply", "POST", {
              days,
              confirm: true,
            });
            setRetention(result);
            setConfirmRetention(false);
            ctx.updated();
          })
        }
      />
    </>
  );
}
