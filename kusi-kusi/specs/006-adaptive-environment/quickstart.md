# Validation quickstart

1. Ejecutar cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings
   y cargo test --workspace.
2. Ejecutar `python3 scripts/verify_adaptive.py --allow-provider --result <archivo-privado>`
   en staging autorizado: dos fuentes, tercer recurso agregado durante la prueba y
   revocación. Los cambios de campo se prueban en `crates/application/tests/adaptive.rs`.
3. Validar decisiones/argumentos inválidos, límites y datos truncados antes de leer.
4. Probar conversaciones de tres turnos y variantes de lenguaje reservadas. Indicar
   si se usó interpretación simulada o un proveedor real; no equiparar ambas evidencias.
5. En staging explícito, descubrir catálogo de la fuente autorizada y resolver una
   consulta natural, una continuación y una proyección sin nombres de negocio en el motor.
6. Suscribirse sobre dos recursos, reentregar eventos, cancelar y repetir. Revisar
   estados de entrega y fallos ambiguos sin duplicar efectos conocidos.

Usar perfiles privados. No imprimir credenciales ni filas personales. Los resultados
se cotejan con SC-001–008; registrar cualquier criterio pendiente sin marcarlo completo.
