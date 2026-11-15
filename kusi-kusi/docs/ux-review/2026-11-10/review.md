# Revisión de experiencia del piloto

Método: dos revisiones independientes (ux_review: diseño y código; ux_evidence: detector y navegador), sintetizadas con inspección del código y captura móvil. Sin cambios al producto. Fecha local: 2026-11-10.

La interfaz tiene una identidad sobria y componentes consistentes, pero su composición sigue la organización de módulos más que las tareas del operador. El problema principal es la falta de un recorrido desde configurar una fuente hasta obtener un resultado útil y comprensible.

## Evidencia y alcance

- Detector sobre web/admin/src: sin hallazgos ([]).
- verify_managed_integrations_browser.py: passed, 10 comprobaciones y seis capturas nuevas. Base desechable y tráfico loopback; ninguna llamada externa.
- Navegador ejecutado sobre build existente mediante next start; no se recompiló. Código fuente revisado por separado. No se certifica correspondencia de todo el build con cada archivo actual.
- Cobertura visual: Conexiones y Operación del piloto, escritorio 1440 px y móvil 390 px. Conversaciones y navegación también revisadas en código.
- No es un estudio con usuarios ni una certificación WCAG. No se midieron rendimiento, lector de pantalla o contraste completo.
- No se alteraron servicios existentes, credenciales, proveedores ni bases reales. Los procesos temporales del runner se cierran al terminar.
- No se inyectó overlay de Impeccable ni se abrió una sesión visible para el usuario: evidencia mediante Chromium headless y PNG. No se inició live-server.

## Prioridades

1. **P1: recorrido inicial inconexo.** El menú contiene 14 destinos planos. Operación/Operación del piloto y Conexiones/Integraciones requieren conocer su diferencia. Proponer un Inicio con siguiente paso y navegación agrupada por tarea; conservar acceso a funciones existentes.
2. **P1: falta una primera consulta útil desde la web.** Conexiones permite guardar y simular campos, pero no conduce a consultar una fuente. El endpoint /v1/local/managed-queries existe sin consumidor en web/admin/src. Conversaciones sigue ofreciendo videos, diseños e Instagram. Crear un recorrido explícito de consulta sintética con selección de fuente, recurso autorizado, resultado y evidencia. No anunciar consultas remotas ni reemplazar el comportamiento del chat con texto que prometa funciones inexistentes.
3. **P1: ejecuciones difíciles de reconocer.** Las filas muestran estado, fecha y revisión; falta un nombre seguro de operación o sistema que permita distinguir tareas. Añadir metadatos autorizados sin cuerpos de mensaje ni datos personales innecesarios.
4. **P2: feedback fuera de contexto.** El aviso «Evaluación sintética guardada» sigue visible en Permisos. Contextualizar o limpiar el aviso al cambiar sección. Persistir la sección en URL para recarga y navegación atrás.
5. **P2: resultados poco adaptados al móvil.** La tabla de 24 casos exige desplazamiento horizontal para ver resultado y evidencia. Mostrar resumen por sistema y fallos primero; abrir cada caso para detalle. Usar nombres humanos para recursos, dejando identificadores en detalles.

## Valoración heurística

Escala cualitativa de calidad 0–4, 4 mejor. Se convierte la revisión de severidad independiente mediante 4 menos severidad. No es una métrica validada con usuarios.

| Heurística | Calidad |
|---|---:|
| Visibilidad del estado | 2 |
| Lenguaje del usuario | 2 |
| Control y libertad | 3 |
| Consistencia | 2 |
| Prevención de errores | 3 |
| Reconocimiento antes que recuerdo | 1 |
| Eficiencia | 2 |
| Minimalismo | 2 |
| Recuperación de errores | 2 |
| Ayuda contextual | 2 |
| Total | 21/40 |

Fortalezas: paleta y jerarquía visual contenidas; estados expresados con texto; diferencia explícita entre simulación y actividad real; consentimiento y borrado visibles.

Primer usuario: no sabe qué abrir primero ni qué resultado puede obtener hoy. Operador recurrente: cuesta reconocer tareas y leer resultados rápidamente; un aviso anterior puede interpretarse como resultado actual.

## Criterios para una mejora posterior

- Desde Inicio, llegar a una consulta sintética autorizada sin conocer nombres internos de módulos.
- Diferenciar configuración guardada, comprobación sintética, verificación remota y activación; las dos últimas continúan pendientes y bloqueadas localmente.
- Conservar el contexto al recargar o volver atrás; los avisos pertenecen a la operación que los generó.
- Reconocer una tarea sin exponer contenido privado.
- En móvil, ver resultado y siguiente acción sin desplazar horizontalmente la página ni una tabla para el resumen.
- Validar estos recorridos con usuarios representativos antes de afirmar que la web es intuitiva.

Preguntas omitidas: el objetivo del piloto y la preferencia por avanzar autónomamente ya están establecidos. Se propone priorizar orientación y primera consulta útil.
