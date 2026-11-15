import { useState, type FormEvent } from "react";
import { api, type User } from "@/api";
import type { Context } from "@/types";
import { Button } from "@/components/ui/button";
import { NativeSelect } from "@/components/ui/native-select";
import { Label } from "@/components/ui/label";
import { useManagedData } from "./managed-data";
export default function PolicySimulation(ctx: Context) {
  const { data, loading, error } = useManagedData<User[]>(
    "/v1/local/users",
    ctx.revision,
    [],
  );
  const [decision, setDecision] = useState<boolean | null>(null),
    [busy, setBusy] = useState(false);
  async function simulate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    setBusy(true);
    setDecision(null);
    try {
      await ctx.run(async () => {
        const result = await api<{ allowed: boolean }>(
          "/v1/local/policy-simulations",
          "POST",
          {
            operation: "demo.resource.read",
            resource_reference: String(form.get("resource")),
            subject_user_id: String(form.get("subject")),
          },
        );
        setDecision(result.allowed);
      });
    } finally {
      setBusy(false);
    }
  }
  const viewers = data.filter(
    (user) => user.role === "viewer" && !user.disabled,
  );
  return (
    <section className="space-y-5" aria-label="Simulación de permisos">
      <h2>Comprobar permisos sin modificarlos</h2>
      <p className="text-sm text-muted-foreground">
        Evalúa las concesiones actuales de un usuario de esta organización sobre
        los recursos ficticios. La simulación no concede acceso ni consulta
        sistemas externos.
      </p>
      {loading ? (
        <p role="status">Cargando usuarios…</p>
      ) : error ? (
        <p role="alert">{error}</p>
      ) : !viewers.length ? (
        <p>
          Crea un usuario de consulta desde Usuarios para probar sus permisos.
        </p>
      ) : (
        <form
          onSubmit={(event) => void simulate(event)}
          className="max-w-lg space-y-4"
        >
          <div className="space-y-2">
            <Label htmlFor="policy-subject">Usuario de consulta</Label>
            <NativeSelect id="policy-subject" name="subject">
              {viewers.map((user) => (
                <option key={user.id} value={user.id}>
                  {user.display_name}
                </option>
              ))}
            </NativeSelect>
          </div>
          <div className="space-y-2">
            <Label htmlFor="policy-resource">Recurso ficticio</Label>
            <NativeSelect id="policy-resource" name="resource">
              <option value="demo-resource-001">
                Recurso de demostración A
              </option>
              <option value="demo-resource-002">
                Recurso de demostración B
              </option>
            </NativeSelect>
          </div>
          <Button disabled={busy} type="submit">
            {busy ? "Comprobando…" : "Simular permiso"}
          </Button>
        </form>
      )}
      {decision !== null && (
        <p role="status">
          {decision
            ? "Permitido por la concesión vigente."
            : "Sin autorización vigente para esta operación y recurso."}
        </p>
      )}
    </section>
  );
}
