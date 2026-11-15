"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { api, setCsrf } from "@/api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";

type Mode = { mode: "local" | "admin"; oidc_enabled: boolean };
type Membership = {
  organization_id: string;
  organization: string;
  role: "owner" | "editor" | "approver" | "viewer";
};
type AdminSession = {
  memberships: Membership[];
  csrf_token: string;
  expires_at: number;
};

const roleLabels: Record<Membership["role"], string> = {
  owner: "Propietario",
  editor: "Editor",
  approver: "Aprobador",
  viewer: "Lector",
};

export default function AdminConsole() {
  const [mode, setMode] = useState<Mode | null>(null);
  const [session, setSession] = useState<AdminSession | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [callbackFailed, setCallbackFailed] = useState(false);

  const refresh = async () => {
    setError("");
    const currentMode = await api<Mode>("/v1/auth/mode");
    setMode(currentMode);
    if (currentMode.mode !== "admin") {
      setSession(null);
      setCsrf("");
      return;
    }
    try {
      const current = await api<AdminSession>("/v1/admin/auth/session");
      setSession(current);
      setCsrf(current.csrf_token);
    } catch {
      setSession(null);
      setCsrf("");
    }
  };

  useEffect(() => {
    setCallbackFailed(
      new URLSearchParams(window.location.search).get("error") ===
        "login_failed",
    );
    let active = true;
    void refresh()
      .catch(() => {
        if (active)
          setError("No se pudo consultar el servicio de administración.");
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <main className="mx-auto flex min-h-svh max-w-3xl items-center px-5">
        <p role="status" className="text-sm text-muted-foreground">
          Comprobando acceso…
        </p>
      </main>
    );
  }

  if (mode?.mode === "local") {
    return (
      <main className="mx-auto flex min-h-svh max-w-3xl items-center px-5 py-12">
        <Card className="w-full">
          <CardHeader>
            <Badge variant="secondary" className="w-fit">
              Workspace local
            </Badge>
            <CardTitle>
              El acceso corporativo no está activo en modo local
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-5">
            <p className="text-sm text-muted-foreground">
              Este entorno usa autenticación local y no contacta proveedores de
              identidad.
            </p>
            <Button asChild variant="outline">
              <Link href="/conversaciones">
                Ir al workspace de demostración
              </Link>
            </Button>
          </CardContent>
        </Card>
      </main>
    );
  }

  if (error) {
    return (
      <main className="mx-auto flex min-h-svh max-w-3xl items-center px-5">
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      </main>
    );
  }

  return (
    <main className="mx-auto flex min-h-svh max-w-3xl items-center px-5 py-12">
      <Card className="w-full">
        <CardHeader className="space-y-3">
          <Badge variant="outline" className="w-fit">
            Plano de administración
          </Badge>
          <CardTitle>
            {session ? "Tus organizaciones" : "Iniciar sesión"}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-5">
          {callbackFailed && !session && (
            <p role="alert" className="text-sm text-destructive">
              No se pudo validar el acceso. Inténtalo de nuevo o contacta al
              administrador.
            </p>
          )}
          {session ? (
            <>
              <p className="text-sm text-muted-foreground">
                Estas membresías se asignan en QORI. El proveedor de identidad
                no cambia tus roles.
              </p>
              {session.memberships.length > 0 ? (
                <div className="divide-y rounded-lg border">
                  {session.memberships.map((membership) => (
                    <div
                      key={membership.organization_id}
                      className="flex flex-wrap items-center justify-between gap-3 p-4"
                    >
                      <span className="font-medium">
                        {membership.organization}
                      </span>
                      <Badge variant="secondary">
                        {roleLabels[membership.role]}
                      </Badge>
                    </div>
                  ))}
                </div>
              ) : (
                <p
                  role="status"
                  className="rounded-lg border p-4 text-sm text-muted-foreground"
                >
                  Tu identidad está activa, pero aún no tiene organizaciones
                  asignadas. Contacta al administrador de QORI.
                </p>
              )}
              <Separator />
              <Button
                variant="outline"
                onClick={() => {
                  void api("/v1/admin/auth/logout", "POST")
                    .then(() => {
                      setSession(null);
                      setCsrf("");
                    })
                    .catch((cause: unknown) => {
                      setError(
                        cause instanceof Error
                          ? cause.message
                          : "No se pudo cerrar la sesión.",
                      );
                    });
                }}
              >
                Cerrar sesión
              </Button>
            </>
          ) : mode?.oidc_enabled ? (
            <>
              <p className="text-sm text-muted-foreground">
                Continúa con la identidad corporativa configurada para esta
                instancia.
              </p>
              <Button asChild>
                <a href="/v1/admin/auth/oidc/start">
                  Continuar con el inicio corporativo
                </a>
              </Button>
            </>
          ) : (
            <p role="status" className="text-sm text-muted-foreground">
              El inicio corporativo aún no está configurado para esta instancia.
            </p>
          )}
        </CardContent>
      </Card>
    </main>
  );
}
