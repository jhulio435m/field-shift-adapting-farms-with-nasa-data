import {
  Collapsible,
  CollapsibleTrigger,
  CollapsibleContent,
} from "@/components/ui/collapsible";
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
import { api } from "../api";
import { getForm, text } from "../ui";
import type { Context } from "../types";
import { useList } from "../hooks";
export default function Workspaces(ctx: Context) {
  const workspaces = useList<{
    id: string;
    name: string;
    slug: string;
    users: number;
  }>("/v1/local/workspaces", ctx);
  return (
    <>
      <p className="lead">
        Cada espacio mantiene sus propios usuarios, archivos y operaciones.
      </p>
      <Collapsible className="panel">
        <CollapsibleTrigger asChild>
          <Button variant="ghost" className="justify-start p-0">
            Crear organización
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="pt-4">
          <form
            className="form-grid"
            onSubmit={(e) => {
              const form = e.currentTarget,
                f = getForm(e);
              void ctx.run(async () => {
                await api("/v1/local/workspaces", "POST", {
                  name: text(f, "name"),
                  slug: text(f, "slug"),
                  username: text(f, "username"),
                  display_name: text(f, "display_name"),
                  password: text(f, "password"),
                });
                form.reset();
                ctx.updated();
                ctx.setNotice(
                  "Organización creada. Su propietario puede iniciar sesión con el identificador elegido.",
                );
              });
            }}
          >
            <Label>
              Nombre
              <Input name="name" minLength={2} maxLength={120} required />
            </Label>
            <Label>
              Identificador
              <Input
                name="slug"
                pattern="[a-z][a-z0-9-]*[a-z0-9]"
                required
                placeholder="mi-empresa"
              />
            </Label>
            <Label>
              Nombre del propietario
              <Input name="display_name" required />
            </Label>
            <Label>
              Usuario propietario
              <Input
                name="username"
                required
                pattern="[a-z0-9._-]+"
                minLength={3}
              />
            </Label>
            <Label>
              Contraseña inicial
              <Input
                type="password"
                name="password"
                autoComplete="new-password"
                required
                minLength={12}
                maxLength={128}
              />
            </Label>
            <Button variant="outline" className="primary">
              Crear espacio
            </Button>
          </form>
        </CollapsibleContent>
      </Collapsible>
      <div className="table-wrap panel">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Organización</TableHead>
              <TableHead>Identificador de acceso</TableHead>
              <TableHead>Usuarios</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {workspaces.map((w) => (
              <TableRow key={w.id}>
                <TableCell>{w.name}</TableCell>
                <TableCell>
                  <code>{w.slug}</code>
                </TableCell>
                <TableCell>{w.users}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      <p className="hint">
        Para trabajar en otro espacio, cierra sesión e ingresa con una cuenta de
        esa organización.
      </p>
    </>
  );
}
