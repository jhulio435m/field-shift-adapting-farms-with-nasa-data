# Product
<!-- impeccable:product-schema 1 -->

## Platform
web

## Stack
User-selected Rust modular monolith, PostgreSQL and Next.js + TypeScript + shadcn/ui (user-selected migration from React/Vite). The local panel uses the existing API and simulated providers.

## Users
People trying to complete an institutional process, including students and graduates, need plain-language guidance and a clear next step. Administrative staff need a reliable case history, evidence and a way to respond. Organization operators configure approved sources, workflows and permissions.

## Product Purpose
Kusi Kusi helps people move through complex processes by connecting existing systems, institutional rules and human support. AI explains requirements in plain language, organizes evidence and case history, and prepares next actions with sources. The institution and its authorized staff retain decisions and approvals.

## Hackathon Scenario
The hackathon prototype focuses on payment traceability. The team is developing a student payment platform that lists a student's payments and authoritative payment/process states. Kusi Kusi connects to that platform and returns the payment history or validates a receipt/payment code through WhatsApp. The demo uses a local clone with synthetic records, including the UNCP S/ 198 PA57002335 example. The payment platform does not exist yet, so no real platform or ADESA connection is available. A no-login demo screen is loopback-only and exposes only the fixed synthetic dataset.

## Operating Context
Local review on the user's Linux workstation, without paid providers. The backend already supports tenant APIs, capability execution, creative jobs and social OAuth. Real app registration, credentials and live publication are not available.

## Capabilities and Constraints
No tenant or student identity comes from an unverified prompt. Payment validation and process status come from the payment platform adapter, not an LLM. Ask only for data needed to locate a payment; do not retain receipt images or unnecessary identity details. Simulated people and payments must be labeled. The demo page is unauthenticated only because it is loopback-bound and returns fixed synthetic data. Authenticated workspace routes retain their existing session, CSRF and RBAC controls. Keep secrets out of the browser and logs.

## Brand Commitments
Name: Kusi Kusi. Spanish interface. User prefers practical operation and low visual noise.

## Evidence on Hand
Existing Rust workspace, API contracts and documented local test evidence. The official UNCP TUPA 2024, approved by Resolution 3226-R-2024, is downloaded in `specs/005-brecha-burocratica/sources/`. It lists PA57002335 at S/ 198. A user-supplied receipt image is only a format reference and contains personal data; it will not be copied into fixtures. The payment-platform clone and every student/payment record are synthetic. No production integration, customer metrics or production readiness claims.

## Product Principles
Bridge people and institutions through understandable, traceable steps. Respect the official process and leave consequential decisions with authorized people. Ground explanations in supplied institutional sources; show uncertainty when evidence is missing. Require permission and confirmation for actions. Show actual state and recoverable failures. Keep the local workflow usable without external credentials.

## Piloto de integraciones elegido (2026-11-10)

El usuario eligió Plane y ERP CONEIMERA como sistemas y Telegram como canal. Operadores propietarios configuran APIs y tokens desde Conexiones. Guardar configuración local cifrada no activa proveedores ni envíos. Se mantienen núcleo genérico, sesiones/CSRF/RBAC, autorización por recurso y separación entre guardado, prueba y activación. El escenario de pagos anterior sigue siendo un fixture independiente, no el límite del producto.
