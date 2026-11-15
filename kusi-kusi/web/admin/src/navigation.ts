import type { View } from "./types";
export const destinations: Record<View, { path: string; title: string }> = {
  home: { path: "/inicio", title: "Inicio" },
  chat: { path: "/conversaciones", title: "Conversaciones" },
  assets: { path: "/archivos", title: "Archivos" },
  jobs: { path: "/trabajos", title: "Trabajos creativos" },
  publications: { path: "/publicaciones", title: "Publicaciones" },
  users: { path: "/usuarios", title: "Usuarios" },
  sessions: { path: "/sesiones", title: "Sesiones" },
  pilot: { path: "/piloto", title: "Operación del piloto" },
  connections: { path: "/conexiones", title: "Conexiones" },
  academic: { path: "/vinculos-academicos", title: "Vínculos académicos" },
  resourceAccess: { path: "/accesos", title: "Accesos a recursos" },
  integrations: { path: "/integraciones", title: "Catálogo de capacidades" },
  activity: { path: "/operacion", title: "Actividad del sistema" },
  workspaces: { path: "/organizaciones", title: "Organizaciones" },
  cases: { path: "/tramites", title: "Trámites y orientación" },
};
export function viewFromPath(path: string): View | undefined {
  return (Object.keys(destinations) as View[]).find(
    (key) => destinations[key].path === path,
  );
}

export const navigationGroups: { title: string; views: View[] }[] = [
  { title: "Tu trabajo", views: ["home", "pilot", "chat", "cases"] },
  {
    title: "Sistemas y acceso",
    views: ["connections", "resourceAccess", "integrations", "academic"],
  },
  { title: "Contenido", views: ["assets", "jobs", "publications"] },
  {
    title: "Administración",
    views: ["users", "sessions", "activity", "workspaces"],
  },
];
