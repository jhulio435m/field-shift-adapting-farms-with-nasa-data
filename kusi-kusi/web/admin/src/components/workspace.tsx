"use client";
import { useState, useEffect, useCallback } from "react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import {
  House,
  MessageCircle,
  Images,
  Film,
  CalendarDays,
  Users,
  Plug,
  Activity,
  Building2,
  GraduationCap,
  FileQuestion,
  KeyRound,
  Monitor,
  LogOut,
  RefreshCw,
  ArrowUpRight,
} from "lucide-react";
import { api, setCsrf, label, type User } from "@/api";
import { getForm, text } from "@/ui";
import { destinations, viewFromPath, navigationGroups } from "@/navigation";
import type { View } from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
  useSidebar,
} from "@/components/ui/sidebar";
import PilotHome from "@/features/PilotHome";
import Chat from "@/features/Chat";
import Assets from "@/features/Assets";
import Jobs from "@/features/Jobs";
import Publications from "@/features/Publications";
import UsersPage from "@/features/UsersPage";
import Sessions from "@/features/Sessions";
import Connections from "@/features/Connections";
import AcademicDemo from "@/features/AcademicDemo";
import PilotOperations from "@/features/PilotOperations";
import ResourceAccess from "@/features/ResourceAccess";
import Integrations from "@/features/Integrations";
import Operations from "@/features/Operations";
import Workspaces from "@/features/Workspaces";
import BureaucraticCases from "@/features/BureaucraticCases";
const icons = {
  home: House,
  chat: MessageCircle,
  assets: Images,
  jobs: Film,
  publications: CalendarDays,
  users: Users,
  sessions: Monitor,
  connections: Plug,
  pilot: Activity,
  academic: GraduationCap,
  resourceAccess: KeyRound,
  integrations: ArrowUpRight,
  activity: Activity,
  workspaces: Building2,
  cases: FileQuestion,
};
function Navigation({
  user,
  view,
  logout,
}: {
  user: User;
  view: View;
  logout: () => void;
}) {
  const { isMobile, setOpenMobile } = useSidebar();
  return (
    <Sidebar collapsible="icon" variant="inset">
      <SidebarHeader className="px-3 py-5">
        <Link
          href="/inicio"
          className="flex items-center gap-3 px-1 font-semibold tracking-tight"
        >
          <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground">
            k.
          </span>
          <span className="truncate text-xl group-data-[collapsible=icon]:hidden">
            QORI
          </span>
        </Link>
        <div className="mt-5 rounded-lg border bg-background/60 p-3 group-data-[collapsible=icon]:hidden">
          <p className="truncate text-sm font-semibold">{user.organization}</p>
          <p className="text-xs text-muted-foreground">Espacio de trabajo</p>
        </div>
      </SidebarHeader>
      <SidebarContent>
        {navigationGroups.map((group) => (
          <SidebarGroup key={group.title}>
            <SidebarGroupLabel>{group.title}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {group.views
                  .filter(
                    (id) =>
                      (id !== "workspaces" || user.platform_admin) &&
                      (id !== "users" || user.role === "owner") &&
                      (id !== "academic" || user.role === "owner"),
                  )
                  .map((id) => {
                    const Icon = icons[id];
                    return (
                      <SidebarMenuItem key={id}>
                        <SidebarMenuButton
                          asChild
                          isActive={view === id}
                          tooltip={destinations[id].title}
                        >
                          <Link
                            href={destinations[id].path}
                            onClick={() => {
                              if (isMobile) setOpenMobile(false);
                            }}
                            aria-current={view === id ? "page" : undefined}
                          >
                            <Icon />
                            <span>{destinations[id].title}</span>
                          </Link>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    );
                  })}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        ))}
      </SidebarContent>
      <SidebarFooter>
        <Separator />
        <div className="flex items-center gap-3 px-2 py-3">
          <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-sidebar-accent font-medium">
            {user.display_name[0]}
          </div>
          <div className="min-w-0 flex-1 group-data-[collapsible=icon]:hidden">
            <p className="truncate text-xs font-semibold">
              {user.display_name}
            </p>
            <p className="text-xs text-muted-foreground">{label(user.role)}</p>
          </div>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Cerrar sesión"
            title="Cerrar sesión"
            className="shrink-0 group-data-[collapsible=icon]:hidden"
            onClick={logout}
          >
            <LogOut />
          </Button>
        </div>
      </SidebarFooter>
    </Sidebar>
  );
}
export default function Workspace() {
  const router = useRouter(),
    pathname = usePathname(),
    query = useSearchParams();
  const view = viewFromPath(pathname) || "home";
  const selected = query.get("selected") || "";
  const [user, setUser] = useState<User | null>(null),
    [loading, setLoading] = useState(true),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [revision, refresh] = useState(0),
    [signingIn, setSigningIn] = useState(false);
  const run = useCallback(async (action: () => Promise<void>) => {
    setError("");
    try {
      await action();
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "No se pudo completar la acción.",
      );
    }
  }, []);
  useEffect(() => {
    let active = true;
    api<{ user: User; csrf_token: string }>("/local/auth/session")
      .then((s) => {
        if (active) {
          setUser(s.user);
          setCsrf(s.csrf_token);
        }
      })
      .catch(() => {})
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    const expired = () => {
      setUser(null);
      setCsrf("");
      setError("Tu sesión venció. Vuelve a iniciar sesión.");
    };
    window.addEventListener("kusi:unauthorized", expired);
    return () => window.removeEventListener("kusi:unauthorized", expired);
  }, []);
  useEffect(() => {
    setNotice("");
    setError("");
  }, [pathname, selected]);
  const updated = () => refresh((n) => n + 1);
  const go = (next: View, id = "") => {
    setError("");
    setNotice("");
    router.push(
      destinations[next].path +
        (id ? "?selected=" + encodeURIComponent(id) : ""),
    );
  };
  const ctx = { user: user!, run, revision, updated, go, selected, setNotice };
  if (loading)
    return (
      <div className="mx-auto max-w-5xl space-y-6 p-8" role="status">
        <span className="sr-only">Abriendo QORI…</span>
        <Skeleton className="h-8 w-44" />
        <Skeleton className="h-96 w-full" />
      </div>
    );
  if (!user)
    return (
      <main className="grid min-h-svh lg:grid-cols-2">
        <section className="flex flex-col justify-between bg-sidebar p-8 lg:p-16">
          <Link href="/inicio" className="text-3xl font-bold tracking-tight">
            QORI
          </Link>
          <div className="my-12 max-w-lg">
            <h1 className="text-4xl font-semibold leading-tight tracking-tight lg:text-5xl">
              Tu operación,
              <br />
              en una conversación.
            </h1>
            <p className="mt-6 max-w-sm text-muted-foreground">
              Archivos, solicitudes y aprobaciones conectados con los sistemas
              de tu organización.
            </p>
          </div>
          <div>
            <Badge variant="outline">Entorno local</Badge>
            <p className="mt-2 text-xs text-muted-foreground">
              Sin publicaciones reales ni consumo externo.
            </p>
          </div>
        </section>
        <section className="flex items-center justify-center p-6 lg:p-12">
          <Card className="w-full max-w-md border-0 shadow-none">
            <CardHeader className="px-0">
              <CardTitle className="text-2xl">
                <h1>Iniciar sesión</h1>
              </CardTitle>
              <CardDescription>
                Ingresa a tu espacio de trabajo con tu cuenta local.
              </CardDescription>
            </CardHeader>
            <CardContent className="px-0">
              <form
                className="space-y-5"
                onSubmit={(e) => {
                  const f = getForm(e);
                  setSigningIn(true);
                  void run(async () => {
                    const s = await api<{ user: User; csrf_token: string }>(
                      "/local/auth/login",
                      "POST",
                      {
                        organization: text(f, "organization"),
                        username: text(f, "username"),
                        password: text(f, "password"),
                      },
                    );
                    setUser(s.user);
                    setCsrf(s.csrf_token);
                  }).finally(() => setSigningIn(false));
                }}
              >
                <div className="space-y-2">
                  <Label htmlFor="organization">Organización</Label>
                  <Input
                    id="organization"
                    name="organization"
                    autoComplete="organization"
                    placeholder="mi-organizacion"
                    required
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="username">Usuario</Label>
                  <Input
                    id="username"
                    name="username"
                    autoComplete="username"
                    required
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="password">Contraseña</Label>
                  <Input
                    id="password"
                    name="password"
                    type="password"
                    autoComplete="current-password"
                    required
                  />
                </div>
                {error && (
                  <Alert variant="destructive">
                    <AlertDescription>{error}</AlertDescription>
                  </Alert>
                )}
                <Button className="w-full" disabled={signingIn}>
                  {signingIn ? "Ingresando…" : "Entrar"}
                  <ArrowUpRight />
                </Button>
              </form>
            </CardContent>
          </Card>
        </section>
      </main>
    );
  const restricted =
    (view === "users" && user.role !== "owner") ||
    (view === "academic" && user.role !== "owner") ||
    (view === "workspaces" && !user.platform_admin);
  return (
    <SidebarProvider>
      <Navigation
        user={user}
        view={view}
        logout={() =>
          void run(async () => {
            await api("/local/auth/logout", "POST");
            setUser(null);
            setCsrf("");
          })
        }
      />
      <SidebarInset>
        <header className="flex h-14 shrink-0 items-center justify-between gap-3 border-b px-4 md:px-8">
          <div className="flex items-center gap-3">
            <SidebarTrigger aria-label="Abrir navegación" />
            <Separator orientation="vertical" className="h-4" />
            <span className="text-xs text-muted-foreground">
              {user.organization}
            </span>
          </div>
          <Badge variant="secondary" className="text-[10px] sm:text-xs">
            Simulación · sin publicaciones reales
          </Badge>
        </header>
        <main className="kk-page flex-1 p-4 md:p-8">
          <div className="mb-7 flex items-center justify-between gap-4">
            <h1 className="text-2xl font-semibold tracking-tight">
              {destinations[view].title}
            </h1>
            <Button
              variant="outline"
              size="icon"
              aria-label="Actualizar"
              onClick={updated}
            >
              <RefreshCw />
            </Button>
          </div>
          {error && (
            <Alert variant="destructive" className="mb-5">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          {notice && (
            <Alert className="mb-5">
              <AlertDescription>{notice}</AlertDescription>
            </Alert>
          )}
          {restricted ? (
            <Alert variant="destructive">
              <AlertDescription>
                Tu rol no permite administrar esta sección.
              </AlertDescription>
            </Alert>
          ) : view === "home" ? (
            <PilotHome {...ctx} />
          ) : view === "chat" ? (
            <Chat {...ctx} />
          ) : view === "assets" ? (
            <Assets {...ctx} />
          ) : view === "jobs" ? (
            <Jobs {...ctx} />
          ) : view === "publications" ? (
            <Publications {...ctx} />
          ) : view === "users" ? (
            <UsersPage {...ctx} />
          ) : view === "sessions" ? (
            <Sessions {...ctx} />
          ) : view === "pilot" ? (
            <PilotOperations {...ctx} />
          ) : view === "connections" ? (
            <Connections {...ctx} />
          ) : view === "academic" ? (
            <AcademicDemo {...ctx} />
          ) : view === "resourceAccess" ? (
            <ResourceAccess {...ctx} />
          ) : view === "integrations" ? (
            <Integrations {...ctx} />
          ) : view === "activity" ? (
            <Operations {...ctx} />
          ) : view === "cases" ? (
            <BureaucraticCases {...ctx} />
          ) : (
            <Workspaces {...ctx} />
          )}
        </main>
      </SidebarInset>
    </SidebarProvider>
  );
}
