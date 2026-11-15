import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import { Card } from "@/components/ui/card";
import { ArrowUpRight } from "lucide-react";
import { Empty } from "../ui";
import type { Context } from "../types";
import { useList } from "../hooks";
export default function Integrations(ctx: Context) {
  const caps = useList<{
    identifier: string;
    version: string;
    description: string;
    enabled: boolean;
    required_permission: string;
  }>("/v1/capabilities", ctx);
  return (
    <>
      <p className="lead">
        La interoperabilidad depende de contratos HTTP y capacidades
        autorizadas, independientemente del lenguaje del sistema conectado.
      </p>
      <Card className="panel">
        <div className="section-line">
          <h2>Capacidades registradas</h2>
          <a
            className="text-link"
            href="/openapi.yaml"
            target="_blank"
            rel="noreferrer"
          >
            Contrato OpenAPI <ArrowUpRight />
          </a>
        </div>
        {caps.length ? (
          <div className="table-wrap">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Identificador</TableHead>
                  <TableHead>Versión</TableHead>
                  <TableHead>Descripción</TableHead>
                  <TableHead>Permiso</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {caps.map((c) => (
                  <TableRow key={c.identifier + ":" + c.version}>
                    <TableCell>
                      <code>{c.identifier}</code>
                    </TableCell>
                    <TableCell>{c.version}</TableCell>
                    <TableCell>{c.description}</TableCell>
                    <TableCell>{c.required_permission}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        ) : (
          <Empty>
            No hay capacidades registradas. Usa la API administrativa para
            definir un contrato, destino permitido y permisos.
          </Empty>
        )}
      </Card>
      <Card className="panel">
        <h2>Incorporar cualquier sistema</h2>
        <ol>
          <li>Expón operaciones concretas mediante una API.</li>
          <li>Registra los esquemas de entrada y salida de cada capacidad.</li>
          <li>
            Configura el destino permitido y sus credenciales en el servidor.
          </li>
          <li>Asigna permisos a identidades verificadas antes de ejecutar.</li>
        </ol>
        <p className="hint">
          El asistente local no selecciona ni ejecuta capacidades empresariales
          automáticamente. No tiene acceso directo a bases de datos.
        </p>
      </Card>
    </>
  );
}
