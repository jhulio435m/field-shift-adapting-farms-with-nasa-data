import {
  Collapsible,
  CollapsibleTrigger,
  CollapsibleContent,
} from "@/components/ui/collapsible";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import { api, label, type User } from "../api";
import { Badge, getForm, text } from "../ui";
import type { Context } from "../types";
import { useList } from "../hooks";
export default function UsersPage(ctx: Context) {
  const users = useList<User>("/v1/local/users", ctx),
    owner = ctx.user.role === "owner";
  return (
    <>
      <p className="lead">
        Los permisos se verifican en el servidor. Deshabilitar una cuenta revoca
        sus sesiones.
      </p>
      {owner && (
        <Collapsible className="panel">
          <CollapsibleTrigger asChild>
            <Button variant="ghost" className="justify-start p-0">
              Agregar usuario
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className="pt-4">
            <form
              className="form-grid"
              onSubmit={(e) => {
                const form = e.currentTarget,
                  f = getForm(e);
                void ctx.run(async () => {
                  await api("/v1/local/users", "POST", {
                    username: text(f, "username"),
                    display_name: text(f, "display_name"),
                    password: text(f, "password"),
                    role: text(f, "role"),
                  });
                  form.reset();
                  ctx.updated();
                });
              }}
            >
              <Label>
                Nombre
                <Input name="display_name" required maxLength={80} />
              </Label>
              <Label>
                Usuario
                <Input
                  name="username"
                  required
                  minLength={3}
                  pattern="[a-z0-9._-]+"
                />
              </Label>
              <Label>
                Contraseña inicial
                <Input
                  name="password"
                  type="password"
                  required
                  minLength={12}
                  maxLength={128}
                  autoComplete="new-password"
                />
              </Label>
              <Label>
                Rol
                <NativeSelect name="role">
                  {["editor", "approver", "viewer", "owner"].map((r) => (
                    <option key={r} value={r}>
                      {label(r)}
                    </option>
                  ))}
                </NativeSelect>
              </Label>
              <Button variant="outline" className="primary">
                Crear usuario
              </Button>
            </form>
          </CollapsibleContent>
        </Collapsible>
      )}
      <div className="table-wrap panel">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Nombre</TableHead>
              <TableHead>Usuario</TableHead>
              <TableHead>Permisos</TableHead>
              <TableHead>Estado</TableHead>
              {owner && <TableHead>Acciones</TableHead>}
            </TableRow>
          </TableHeader>
          <TableBody>
            {users.map((u) => (
              <TableRow key={u.id}>
                <TableCell>
                  <strong>{u.display_name}</strong>
                  {u.platform_admin && (
                    <small>Administrador de plataforma</small>
                  )}
                </TableCell>
                <TableCell>{u.username}</TableCell>
                <TableCell>
                  {owner ? (
                    <NativeSelect
                      aria-label={`Rol de ${u.username}`}
                      value={u.role}
                      onChange={(e) => {
                        const role = e.target.value;
                        void ctx.run(async () => {
                          await api(`/v1/local/users/${u.id}`, "PATCH", {
                            role,
                            disabled: u.disabled,
                          });
                          ctx.updated();
                        });
                      }}
                    >
                      {["owner", "editor", "approver", "viewer"].map((r) => (
                        <option key={r} value={r}>
                          {label(r)}
                        </option>
                      ))}
                    </NativeSelect>
                  ) : (
                    label(u.role)
                  )}
                </TableCell>
                <TableCell>
                  <Badge value={u.disabled ? "Deshabilitado" : "Activo"} />
                </TableCell>
                {owner && (
                  <TableCell>
                    <div className="actions">
                      <Button
                        variant="outline"
                        onClick={() =>
                          void ctx.run(async () => {
                            await api(`/v1/local/users/${u.id}`, "PATCH", {
                              role: u.role,
                              disabled: !u.disabled,
                            });
                            ctx.updated();
                          })
                        }
                      >
                        {u.disabled ? "Habilitar" : "Deshabilitar"}
                      </Button>
                      <Collapsible>
                        <CollapsibleTrigger asChild>
                          <Button variant="ghost" className="justify-start p-0">
                            Restablecer contraseña
                          </Button>
                        </CollapsibleTrigger>
                        <CollapsibleContent className="pt-4">
                          <form
                            className="stack"
                            onSubmit={(e) => {
                              const form = e.currentTarget,
                                f = getForm(e);
                              void ctx.run(async () => {
                                await api(
                                  `/v1/local/users/${u.id}/password`,
                                  "POST",
                                  { password: text(f, "password") },
                                );
                                form.reset();
                                ctx.setNotice(
                                  "Contraseña actualizada. Se revocaron las sesiones anteriores.",
                                );
                              });
                            }}
                          >
                            <Input
                              type="password"
                              name="password"
                              minLength={12}
                              maxLength={128}
                              aria-label={`Nueva contraseña de ${u.username}`}
                              required
                              autoComplete="new-password"
                            />
                            <Button variant="outline">Guardar</Button>
                          </form>
                        </CollapsibleContent>
                      </Collapsible>
                    </div>
                  </TableCell>
                )}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      <div className="permissions panel">
        <h2>Responsabilidades</h2>
        <p>
          <strong>Editor:</strong> carga archivos y prepara borradores.{" "}
          <strong>Aprobador:</strong> confirma o cancela versiones.{" "}
          <strong>Solo lectura:</strong> consulta. <strong>Propietario:</strong>{" "}
          administra el espacio y sus usuarios.
        </p>
      </div>
    </>
  );
}
