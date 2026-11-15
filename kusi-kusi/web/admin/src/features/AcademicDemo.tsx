import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api, type User } from "../api";
import { Empty } from "../ui";
import type { Context } from "../types";

type DemoStudent = {
  student_ref: string;
  display_label: string;
  level: string;
};
type GuardianLink = {
  user_id: string;
  user_name: string;
  student_ref: string;
  student_label: string;
  created_at: number;
};
type Channel = { id: string; provider: string };

export default function AcademicDemo(ctx: Context) {
  const [students, setStudents] = useState<DemoStudent[]>([]),
    [users, setUsers] = useState<User[]>([]),
    [links, setLinks] = useState<GuardianLink[]>([]),
    [channels, setChannels] = useState<Channel[]>([]),
    [loading, setLoading] = useState(true),
    [userId, setUserId] = useState(""),
    [studentRef, setStudentRef] = useState(""),
    [channelId, setChannelId] = useState(""),
    [verificationConfirmed, setVerificationConfirmed] = useState(false),
    [oneTimeCode, setOneTimeCode] = useState("");
  const viewers = users.filter(
    (user) => user.role === "viewer" && !user.disabled,
  );
  const selectedViewerHasLink = links.some((link) => link.user_id === userId);

  useEffect(() => {
    void ctx.run(async () => {
      setLoading(true);
      try {
        const [roster, accounts, guardianLinks, settings] = await Promise.all([
          api<DemoStudent[]>("/v1/local/academic/demo-students"),
          api<User[]>("/v1/local/users"),
          api<GuardianLink[]>("/v1/local/academic/guardian-links"),
          api<{ channels: Channel[] }>("/v1/local/settings"),
        ]);
        setStudents(roster);
        setUsers(accounts);
        setLinks(guardianLinks);
        setChannels(settings.channels);
      } finally {
        setLoading(false);
      }
    });
  }, [ctx.revision]);

  useEffect(() => {
    if (!oneTimeCode) return;
    const timer = window.setTimeout(() => setOneTimeCode(""), 600_000);
    return () => window.clearTimeout(timer);
  }, [oneTimeCode]);

  useEffect(() => {
    if (!userId && viewers[0]) setUserId(viewers[0].id);
  }, [userId, users]);
  useEffect(() => {
    if (!studentRef && students[0]) setStudentRef(students[0].student_ref);
  }, [studentRef, students]);
  useEffect(() => {
    if (!channelId && channels[0]) setChannelId(channels[0].id);
  }, [channelId, channels]);

  const createLink = () => {
    if (!userId || !studentRef || !verificationConfirmed) return;
    void ctx.run(async () => {
      await api("/v1/local/academic/guardian-links", "POST", {
        user_id: userId,
        student_ref: studentRef,
        relationship_verified: true,
      });
      setVerificationConfirmed(false);
      ctx.updated();
    });
  };

  const issueCode = () => {
    if (!userId || !channelId) return;
    setOneTimeCode("");
    void ctx.run(async () => {
      const result = await api<{ code: string; expires_in_seconds: number }>(
        "/v1/local/link-codes",
        "POST",
        { user_id: userId, channel_id: channelId },
      );
      setOneTimeCode(result.code);
    });
  };

  const removeLink = (link: GuardianLink) => {
    void ctx.run(async () => {
      await api(
        `/v1/local/academic/guardian-links/${encodeURIComponent(link.user_id)}/${encodeURIComponent(link.student_ref)}`,
        "DELETE",
      );
      setOneTimeCode("");
      ctx.updated();
    });
  };

  return (
    <div className="space-y-6">
      <p className="lead max-w-3xl">
        Asigna registros ficticios a cuentas familiares después de validar el
        vínculo fuera de QORI. Las consultas son de solo lectura y las
        respuestas se simulan localmente; no se envían mensajes.
      </p>

      {loading ? (
        <Card className="p-5" role="status">
          Cargando los datos ficticios…
        </Card>
      ) : (
        <>
          <Card className="space-y-4 p-5">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h2 className="text-lg font-semibold">
                  Vincular una cuenta familiar
                </h2>
                <p className="mt-1 text-sm text-muted-foreground">
                  Usa únicamente usuarios de demostración con rol de lectura.
                </p>
              </div>
              <Badge variant="secondary">Datos ficticios</Badge>
            </div>
            <div
              className="grid gap-2 sm:grid-cols-2"
              aria-label="Estudiantes ficticios disponibles"
            >
              {students.map((student) => (
                <div
                  className="rounded-lg border bg-muted/30 px-4 py-3"
                  key={student.student_ref}
                >
                  <p className="text-sm font-medium">{student.display_label}</p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {student.level}
                  </p>
                </div>
              ))}
            </div>
            {!viewers.length ? (
              <Empty>
                Crea primero una cuenta familiar con rol Lector en Usuarios.
              </Empty>
            ) : (
              <div className="grid gap-4 md:grid-cols-2">
                <Label className="grid gap-2">
                  Cuenta familiar
                  <NativeSelect
                    aria-label="Cuenta familiar"
                    value={userId}
                    onChange={(event) => {
                      setUserId(event.target.value);
                      setOneTimeCode("");
                    }}
                  >
                    {viewers.map((user) => (
                      <option key={user.id} value={user.id}>
                        {user.display_name} · {user.username}
                      </option>
                    ))}
                  </NativeSelect>
                </Label>
                <Label className="grid gap-2">
                  Estudiante de prueba
                  <NativeSelect
                    aria-label="Estudiante de prueba"
                    value={studentRef}
                    onChange={(event) => setStudentRef(event.target.value)}
                  >
                    {students.map((student) => (
                      <option
                        key={student.student_ref}
                        value={student.student_ref}
                      >
                        {student.display_label} · {student.level}
                      </option>
                    ))}
                  </NativeSelect>
                </Label>
                <label className="flex items-start gap-3 text-sm md:col-span-2">
                  <Checkbox
                    checked={verificationConfirmed}
                    onCheckedChange={(checked) =>
                      setVerificationConfirmed(checked === true)
                    }
                    aria-label="Confirmar validación manual del vínculo"
                  />
                  <span>
                    Confirmo que revisé el vínculo familiar fuera del sistema.
                    No usar datos ni documentos reales en esta demostración.
                  </span>
                </label>
                <div className="flex flex-wrap items-center gap-3 md:col-span-2">
                  <Button
                    onClick={createLink}
                    disabled={!userId || !studentRef || !verificationConfirmed}
                  >
                    Guardar vínculo de prueba
                  </Button>
                  <span className="text-xs text-muted-foreground">
                    La acción queda registrada en Operación.
                  </span>
                </div>
              </div>
            )}
          </Card>

          <Card className="space-y-4 p-5">
            <div>
              <h2 className="text-lg font-semibold">
                Conectar el canal de prueba
              </h2>
              <p className="mt-1 text-sm text-muted-foreground">
                El código dura diez minutos, se usa una vez y solo se muestra
                aquí.
              </p>
            </div>
            {!viewers.length ? (
              <Empty>
                Agrega una cuenta familiar de demostración para continuar.
              </Empty>
            ) : !channels.length ? (
              <Empty>
                No hay canales de prueba registrados en el servidor local.
              </Empty>
            ) : (
              <div className="flex flex-wrap items-end gap-3">
                <Label className="grid min-w-52 gap-2">
                  Canal registrado
                  <NativeSelect
                    aria-label="Canal registrado"
                    value={channelId}
                    onChange={(event) => {
                      setChannelId(event.target.value);
                      setOneTimeCode("");
                    }}
                  >
                    {channels.map((channel) => (
                      <option key={channel.id} value={channel.id}>
                        {channel.provider} · {channel.id}
                      </option>
                    ))}
                  </NativeSelect>
                </Label>
                <Button
                  variant="outline"
                  onClick={issueCode}
                  disabled={!selectedViewerHasLink}
                >
                  Generar código para esta cuenta
                </Button>
              </div>
            )}
            {viewers.length > 0 &&
              channels.length > 0 &&
              !selectedViewerHasLink && (
                <p className="text-xs text-muted-foreground">
                  Primero asigna al menos un estudiante ficticio a esta cuenta.
                </p>
              )}
            {oneTimeCode && (
              <div
                className="rounded-lg border bg-muted/40 p-4"
                role="status"
                aria-live="polite"
              >
                <p className="text-sm font-medium">Código de un solo uso</p>
                <code className="mt-2 block break-all text-sm">
                  /vincular {oneTimeCode}
                </code>
                <p className="mt-2 text-xs text-muted-foreground">
                  Entrégalo solo al usuario de prueba por un medio confiable. No
                  se guarda en el navegador y no vuelve a mostrarse al recargar.
                </p>
                <Button
                  className="mt-3"
                  size="sm"
                  variant="ghost"
                  onClick={() => setOneTimeCode("")}
                >
                  Ocultar código
                </Button>
              </div>
            )}
          </Card>
        </>
      )}

      <Card className="space-y-4 p-5">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <h2 className="text-lg font-semibold">Vínculos activos</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Cada vínculo limita qué registros de prueba puede consultar una
              cuenta.
            </p>
          </div>
          <span className="text-sm text-muted-foreground">
            {links.length} {links.length === 1 ? "vínculo" : "vínculos"}
          </span>
        </div>
        {!links.length ? (
          <Empty>Todavía no hay vínculos familiares de prueba.</Empty>
        ) : (
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Cuenta</TableHead>
                  <TableHead>Estudiante ficticio</TableHead>
                  <TableHead>Estado</TableHead>
                  <TableHead>Acción</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {links.map((link) => (
                  <TableRow key={`${link.user_id}:${link.student_ref}`}>
                    <TableCell>{link.user_name}</TableCell>
                    <TableCell>{link.student_label}</TableCell>
                    <TableCell>
                      <Badge variant="secondary">Prueba</Badge>
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => removeLink(link)}
                      >
                        Quitar vínculo
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </Card>
    </div>
  );
}
