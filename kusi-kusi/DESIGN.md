# Kusi Kusi: interfaz de operación

Panel administrativo en español para personal de organizaciones. Prioridad: seguir una solicitud desde el archivo hasta las dos aprobaciones y su comprobante. El modo local y sus límites se muestran en la navegación, el asistente, las piezas y las publicaciones.

Dirección Operate / code-led. Fondo claro cálido, tinta verde oscura, divisores discretos y una barra lateral estable. Tipografía del sistema para lectura de formularios y tablas. Un único acento verde, sin gradientes, animaciones decorativas ni sombras. Los estados usan texto además del color.

Conversaciones usa lista y conversación; archivos usa miniaturas; trabajos y publicaciones usan lista y detalle. Los detalles mantienen visibles los parámetros antes de aprobar. Formularios nativos, etiquetas asociadas, foco visible, mensajes de error junto al contenido, controles inaccesibles deshabilitados según el rol. El servidor aplica los mismos permisos. En móvil la navegación pasa a una franja desplazable; las tablas conservan desplazamiento horizontal propio.

Ninguna contraseña ni clave administrativa se guarda en localStorage. Sesión HttpOnly, CSRF en memoria. La UI no sugiere que una simulación se publicó realmente.

## Migración Next/shadcn

El panel utiliza App Router y shadcn/ui con Tailwind 4 y Lucide: Sidebar/Sheet responsivos, formularios, Native Select, tablas, tarjetas, Checkbox y Alert Dialog. Las variables heredadas de las superficies se separan de los tokens de shadcn para mantener contraste. Se conserva la paleta aprobada, la misma operación y las dos aprobaciones independientes.
