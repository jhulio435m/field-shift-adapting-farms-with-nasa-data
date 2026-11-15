# Quickstart: demo de pagos

1. Arranca PostgreSQL y aplica migraciones como se indica en `docs/LOCAL_WORKSPACE.md`.
2. Habilita `KUSI_LOCAL_MODE=true` con API en `127.0.0.1` y `KUSI_LOCAL_ORIGIN=http://127.0.0.1:3000`.
3. Instala Tesseract con idioma `spa`; la demo no llama a servicios OCR externos.
4. Inicia API y Next con `./scripts/run_local.sh`.
5. Abre `http://127.0.0.1:3000/` o `/tramites`. No solicita login y solo muestra fixtures sintéticos.
6. Para mostrar el historial, consulta con el DNI ficticio `70000001`; prueba `UNCP-EXP-004821` o `V999-00004821`, y sube “Recibo sintético de demostración”. No ingreses un DNI real.
7. Para WhatsApp local, habilita un webhook con fixture de firma y vincula el remitente mediante el panel local. Envía `mis pagos` o `UNCP-EXP-004821`. La respuesta se queda simulada.

El archivo PNG sintético no es comprobante real. La foto que el usuario compartió como referencia no está en el repositorio. Los eventos de imagen de WhatsApp se reciben, pero aún no se descargan ni se procesan por OCR.

Para servir una instancia efímera y aislada de la demo sin usar los datos locales habituales, ejecuta `python3 scripts/verify_payment_demo_browser.py --serve` y abre `http://127.0.0.1:18088/tramites`. Detenerla con Ctrl+C elimina su esquema temporal de PostgreSQL y sus archivos de prueba.
