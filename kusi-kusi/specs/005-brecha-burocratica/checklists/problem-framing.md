# Problem Framing Checklist: Acompañamiento de trámites interrumpidos

**Purpose**: Revisar calidad, alcance y verificabilidad de los requisitos del caso UNCP para la hackatón.
**Created**: 2026-11-06
**Feature**: [spec.md](../spec.md)

**Note**: Checklist de calidad de requisitos generado para el feature.
**Review Ownership**: La lista revisa la redacción y cobertura de requisitos, no si el código ya está implementado.

## Cobertura y claridad

- [x] CHK001 ¿El caso de uso identifica la universidad, el procedimiento PA57002335, el monto S/ 198 y la interrupción por salud sin inventar detalles? [Completeness, Spec §Contexto]
- [x] CHK002 ¿Se distingue la constancia de expedito de la emisión del título profesional y de la devolución de tasas? [Clarity, Spec §User Story 1]
- [x] CHK003 ¿Cada hecho del caso especifica si viene de la persona, recibo, TUPA o personal administrativo? [Clarity, Spec §FR-004]
- [x] CHK004 ¿Se explica qué ocurre cuando el TUPA no decide la vigencia/reutilización del pago o la excepción por salud? [Exception Flow, Spec §FR-009]

## Fuentes y autoridad

- [x] CHK005 ¿Cada requisito que menciona reglas institucionales exige versión, aprobación/vigencia confirmada y área responsable? [Completeness, Spec §FR-002]
- [x] CHK006 ¿La spec impide que la IA declare una mala interpretación, titulación o devolución como decisión final? [Consistency, Spec §FR-009–FR-010]
- [x] CHK007 ¿La respuesta administrativa, el envío simulado y la decisión formal se diferencian claramente? [Clarity, Spec §User Story 3]

## Privacidad y seguridad

- [x] CHK008 ¿El alcance excluye datos reales y fija qué información mínima de salud puede registrarse? [Privacy, Spec §FR-008, FR-011]
- [x] CHK009 ¿El consentimiento identifica el contenido, evidencia y destinatario antes de cada envío? [Authorization, Spec §FR-012]
- [x] CHK010 ¿Se especifica que sesión/organización autorizan el caso y que texto/archivos no conceden permisos ni disparan acciones? [Security, Spec §FR-007, FR-013]
- [x] CHK011 ¿Se prohíbe transmisión a proveedores de IA/analítica externos y se diferencia inferencia local de la guía determinista? [Privacy, Plan §Constraints]

## Resultados medibles

- [x] CHK012 ¿Los criterios de éxito miden comprensión del siguiente paso, respaldo de fuentes, consentimiento y atribución administrativa? [Measurability, Spec §Success Criteria]
- [x] CHK013 ¿El prototipo queda acotado a datos sintéticos y una ruta de atención compatible con lo que la interfaz y API soportan? [Scope, Spec §Assumptions]
