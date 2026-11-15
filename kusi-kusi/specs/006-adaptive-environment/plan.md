# Implementation Plan: Autonomía y adaptación al entorno

**Branch**: `006-adaptive-environment` (contexto de Spec Kit; checkout sin Git operativo)
**Date**: 2026-11-09 | **Spec**: [spec.md](spec.md)

## Summary

Construir un motor Rust de catálogo, propuestas tipadas, lectura autorizada y análisis
reutilizable. Un adaptador Frappe descubre metadatos/permisos sin nombres de negocio.
Un adaptador de manifiestos permite probar entornos distintos y cambios de esquema sin
red. Telegram será transporte hacia el mismo ejecutable de aplicación, no un motor
sectorial Python. Los eventos usan suscripciones genéricas, contexto registrado y
entregas durables con cancelación/revalidación.

## Technical Context

**Language/Version**: Rust 2024; Python estándar como puente de transporte Frappe/Telegram.
**Primary Dependencies**: serde, async-trait, chrono, reqwest y el puerto LlmProvider existente.
**Storage**: archivos JSON privados por contexto autorizado, reemplazo atómico y claims
exclusivos; formatos versionados documentados. No se altera el esquema PostgreSQL existente.
**Testing**: cargo fmt/clippy/test; fixtures de tres recursos y dos fuentes; lenguaje natural
con catálogo disponible/variantes reservadas; bridge Frappe y receptor de staging explícito.
**Target Platform**: Linux; proceso aislado de staging, sin alterar el workspace local.
**Project Type**: aplicación modular con ejecutable JSON stdin/stdout y adaptadores.
**Performance Goals**: límites configurados de llamadas, filas, campos y horizonte temporal;
fallo o aclaración antes de resultados incompletos. Sin objetivo de carga no comprobado.
**Constraints**: ninguna URL, credencial, SQL o comando desde propuestas del modelo;
catálogo y permisos renovados antes de leer; datos personales excluidos por política inicial;
metadatos externos tratados como datos no confiables; escrituras no implementadas.
**Scale/Scope**: hasta 4 consultas, 8 filtros por consulta, 8 análisis por consulta, 10 000
filas totales y 366 días de proyección. Adaptación por metadatos, no código por negocio.

## Constitution Check

| Principio | Antes/después del diseño | Decisión |
|---|---|---|
| Núcleo genérico | PASS | Contratos sin DocTypes ni nombres sectoriales |
| Capas | PASS | Dominio: tipos/análisis; aplicación: puertos y autorización; infraestructura: proveedores/almacenamiento/conectores; server: composición |
| Tenant y privacidad | PASS | Contexto de ejecución procede de configuración privada vinculada al chat verificado; nunca del mensaje |
| Automatización segura | PASS | Solo lecturas y suscripciones delegadas; sin SQL generado; staging separado del modo local |
| Interfaces fallibles | PASS | Errores tipados, contrato JSON versionado, formatos persistidos versionados |
| Evidencia | PASS | Criterios SC-001–008 trazables; fixtures no sustituyen evidencia de lenguaje natural o conexión real |

## Project Structure

### Documentation

`specs/006-adaptive-environment/{spec,plan,research,data-model,quickstart,tasks}.md`
y `contracts/adaptive.md`.

### Source Code

- `crates/domain/src/adaptive/{mod,analysis}.rs`: catálogo, filtros, planes, interpretaciones, operaciones y motor determinista de evidencias.
- `crates/application/src/adaptive/{mod,render}.rs`: puertos hexagonales, descubrimiento, planificación, lectura, contexto, auditoría, eventos y presentación de evidencia.
- `crates/infrastructure/src/adaptive/{mod,sources,planner,state}.rs`: adaptadores por puerto (manifiestos/puente Frappe `Sources`, intérprete LLM `LanguagePlanner`, estado privado y auditoría `FileState`).
- `apps/server/src/bin/adaptive_assistant.rs`: composición explícita de staging y contrato CLI.
- `scripts/adaptive_frappe_bridge.py`: RPC fijo discover/read/watch, permiso Frappe por usuario.
- `scripts/telegram_adaptive.py`: transporte de mensajes/eventos hacia el ejecutable.
- `scripts/run_telegram_staging.py`: solo enrutamiento genérico y entrega del canal ya vinculado.
- `scripts/verify_adaptive.py`: aceptación reproducible con fuentes sintéticas y variantes.

## Complexity Tracking

No excepción constitucional. Un protocolo nuevo puede requerir su adaptador; nuevos
recursos del protocolo existente no requieren modificar núcleo, prompts ni puentes.
