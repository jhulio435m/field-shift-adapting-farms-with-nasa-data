# Research — 010

Fuentes primarias, revisión de licencias y comparación: [GITHUB_REFERENCES.md](../../docs/GITHUB_REFERENCES.md). Investigación consolidada con revisión independiente de arquitectura y código de configuración actual.

## Decisiones

1. **Decision**: ciclo configuración → verificación → habilitación ligado a revisión, organización y versión de adaptador. **Rationale**: evitar carreras de rotación y confusión simulación/producción. **Alternatives considered**: activar al guardar o confiar solo en token válido; descartadas por autorización insuficiente.
2. **Decision**: manifiestos propios inspirados en Activepieces y gestión de conexiones inspirada en Nango. **Rationale**: preservar contratos y bóveda existentes. **Alternatives considered**: embeber sus motores; introduce operación adicional y revisión de licencias sin necesidad demostrada.
3. **Decision**: adoptar patrones de gateway de OpenClaw y continuidad de Hermes con contexto registrado, caducidad y revocación. **Rationale**: mejorar UX sin compartir memoria o conceder herramientas arbitrarias. **Alternatives considered**: gateway independiente con autoridad propia; duplicaría políticas.
4. **Decision**: adaptar checkpoints/interrupciones de LangGraph a persistencia existente. **Rationale**: conservar idempotencia y aprobaciones. **Alternatives considered**: introducir runtime Python; no requerido. Un checkpoint no prueba exactamente una vez.
5. **Decision**: evaluación y evidencia inspiradas en Dify; revisión por herramienta inspirada en n8n. **Rationale**: detectar regresiones de acceso/calidad. **Alternatives considered**: copiar UI/código; condiciones de licencia requieren evaluación por archivo y uso.
6. **Decision**: documentar ADR de Rig y Cedar; mantener proveedores/ACL actuales inicialmente. **Rationale**: Rig main revisado pide Rust 1.97.1 frente a 1.95.0; Cedar no resuelve identidad ni filtrado por recurso. **Alternatives considered**: actualización implícita o segundo motor de políticas; descartadas sin beneficio medido.
7. **Decision**: probes solo lectura e identidad + alcance por separado. **Rationale**: HTTP 200 no demuestra respuesta correcta ni acceso a recursos. **Alternatives considered**: verificar solo disponibilidad; insuficiente. Contratos exactos de instalaciones se documentan como fixtures versionados y se validan remotamente solo en entorno autorizado.

## Dependencias externas

No se requiere acceso real para completar aceptación local. TLS, identidades esperadas, permisos remotos, rate limits y contratos de instalación se registran como evidencia pendiente hasta probarlos. Long polling y webhooks reales siguen sin activarse en local. No se instala software externo como parte del diseño.

## ADR-010-01 — Dependencias y licencias (aceptada para implementación local)

Mantener contratos propios y adaptar patrones sin copiar código de los motores. No añadir Activepieces/Nango/Dify/n8n como runtimes: licencias por archivo y costes operativos no justifican su incorporación para tres adaptadores. Fuentes y restricciones en docs/GITHUB_REFERENCES.md. Mantener Rust 1.95.0; Rig main revisado requiere 1.97.1 y no aporta una necesidad actual que justifique actualizar. Reconsiderar una release compatible con ensayo aislado y comparación de código eliminado antes de añadir dependencia. Mantener ACL actuales y simulador de solo lectura; Cedar se reconsidera si aparecen políticas no expresables o dificultad demostrada de auditoría. No cambiar autoridad SQL ni identidad por introducir un motor.
