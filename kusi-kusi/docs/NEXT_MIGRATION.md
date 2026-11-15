# Migración del panel a Next.js, Turbopack y shadcn/ui

Se reemplazó el frontend Vite por Next.js 16 App Router, TypeScript y Tailwind CSS 4. `next dev --turbopack` y `next build --turbopack` usan explícitamente Turbopack. El diseño conserva la identidad sobria de Kusi Kusi; la navegación y los controles ahora usan componentes de shadcn/ui incorporados mediante su CLI oficial.

## Organización

- `web/admin/src/app`: layout, rutas de pantalla, errores/404 y Route Handlers del proxy.
- `web/admin/src/components/ui`: código de shadcn para Sidebar/Sheet, Button, Input, Textarea, Label, Native Select, Table, Card, Badge, Checkbox, Alert, Alert Dialog, Collapsible y estados de carga. Los componentes están dentro del repositorio; no son una dependencia visual remota.
- `web/admin/src/components/workspace.tsx`: sesión, navegación, permisos visibles y composición del espacio.
- `web/admin/src/features`: conversaciones, archivos, trabajos, publicaciones, usuarios, conexiones, integraciones, operación y organizaciones.
- `web/admin/src/lib/backend.ts`: proxy exclusivamente a un destino configurado por el servidor.
- `web/admin/components.json`: configuración shadcn; Lucide es la biblioteca de iconos del panel.

Las pantallas tienen URLs reales: `/conversaciones`, `/archivos`, `/trabajos`, `/publicaciones`, `/usuarios`, `/conexiones`, `/integraciones`, `/operacion` y `/organizaciones`. Enlaces a registros incluyen `?selected=<uuid>`. Se mantienen recarga directa, historial del navegador y rutas inexistentes con 404.

## Frontera con Rust

Next atiende `http://127.0.0.1:3000`; Rust conserva `http://127.0.0.1:8080`. Los Route Handlers reenvían `/v1/*`, `/local/*`, `/health/*` y `/openapi.yaml` al `KUSI_API_ORIGIN` fijo, sin permitir elegir otro destino mediante la URL de la petición. No se envían claves administrativas al navegador ni se inyectan claves de servidor en el proxy.

Las cookies de sesión siguen siendo HttpOnly y SameSite Strict. El proxy conserva cookies, CSRF y Origin sin inventar una identidad. Rust verifica rol, tenant y recursos. Las respuestas de API son `no-store`; imágenes y videos siguen protegidos por sesión. La autorización se mantiene en Rust; ocultar una sección o control no se considera un permiso.

El bind del backend y el origen del frontend deben ser ambos loopback; pueden tener puertos diferentes. Rust compara cada Origin recibido con el origen configurado. Ningún puerto público se habilitó y no se activaron proveedores externos.

## Arranque

```sh
docker compose up -d postgres
./scripts/run_local.sh
```

El script compila Next con Turbopack y Rust, inicia el backend con origen permitido `http://127.0.0.1:3000` y arranca el servidor standalone Next en 3000. Al terminar limpia su proceso de backend. Los puertos deben estar libres antes de ejecutarlo.

Si todavía no existe la cuenta local: `python3 scripts/setup_local.py` usando la clave de la organización aprovisionada. Conserva las cuentas existentes. El archivo privado `.local-data/login.json` mantiene usuario, organización y contraseña; su URL ahora apunta a 3000.

Para desarrollar el panel, mantén el backend con `KUSI_LOCAL_MODE=true` y `KUSI_LOCAL_ORIGIN=http://127.0.0.1:3000`; ejecuta `npm run dev --prefix web/admin`. El build de producción se sirve con `npm run start --prefix web/admin` usando `.next/standalone`; no utiliza `next start` ni una exportación HTML estática.

## Docker y CI

Se separaron las imágenes de API y web: `infra/docker/Dockerfile` y `Dockerfile.web`. La imagen web usa Node 24, runtime standalone sin privilegios root y `KUSI_API_ORIGIN` en el servidor. No necesita reconstruirse para cambiar ese destino. El perfil Compose `panel` añade el servidor Next; el modo API convencional de Compose no activa por sí solo las sesiones locales ni sustituye el OIDC pendiente. Para la demostración local autenticada usa el script del host, que configura ambos orígenes loopback.

Comprobaciones: build Next/Turbopack, TypeScript, formato, regresión Rust, API local, recorrido Chromium y pruebas del proxy. La evidencia real de esta migración está en VALIDATION.md. Los proveedores y el paso a producción conservan los límites de ROADMAP.md y SECURITY.md.

## Fuentes oficiales consultadas

- [Next.js Turbopack](https://nextjs.org/docs/app/api-reference/turbopack).
- [shadcn/ui para Next.js](https://ui.shadcn.com/docs/installation/next).
