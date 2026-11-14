# Integración con el Kusi Kusi existente

Field Shift aporta datos y comparaciones. Kusi Kusi conserva la identidad, los
canales, el intérprete y las autorizaciones. Este puente solo implementa el RPC
de fuentes adaptativas `discover/read`; no envía mensajes, instala webhooks ni
acepta ubicaciones o identidades del texto del chat.

## Registro y prueba del número

El navegador inicia `POST /v1/onboarding/start` con nombre y teléfono E.164.
Recibe un `challenge` válido diez minutos. **Esto no envía un WhatsApp.** El
transporte verificado de Kusi Kusi debe recibir ese identificador desde el mismo
remitente, resolver su vínculo personal y llamar por una conexión interna a
`POST /v1/onboarding/confirm` con:

```json
{"challenge":"...","phone":"+51999999999","verified_at":1700000000,"event_id":"id-estable-del-evento","signature":"hex-hmac-sha256"}
```

La firma es HMAC-SHA256 hexadecimal de los bytes UTF-8
`challenge + "\n" + phone + "\n" + verified_at + "\n" + event_id`, usando
`FIELD_SHIFT_KUSI_VERIFY_KEY` (mínimo 32 caracteres), compartida solo entre
servicios. Kusi Kusi debe firmar **después** de verificar el remitente y su
vínculo; nunca sobre un número incluido en el texto del chat. La respuesta
incluye `claim_code` de un solo uso que Kusi Kusi debe entregar a ese mismo
remitente. El usuario lo introduce en la web: `/v1/onboarding/claim` crea la
sesión HttpOnly. El secreto no se envía al navegador ni se versiona.

El callback real de Kusi Kusi y el envío del código todavía no están instalados
en su checkout. Sin `FIELD_SHIFT_KUSI_VERIFY_KEY`, el inicio se niega con 503;
no existe un modo que declare un teléfono verificado sin prueba. La UI puede
mostrar el flujo, pero no completar el registro por WhatsApp hasta conectar
ese servicio. SQLite queda en `backend/.local-data/farmers.sqlite3` (0600) o
en `FIELD_SHIFT_DB`; no mezclar esa base con el almacenamiento de Kusi Kusi.

## Fuente de una parcela registrada

1. Ejecutar Field Shift en `127.0.0.1` (por ejemplo,
   `cd backend && .venv/bin/uvicorn app.main:app --host 127.0.0.1 --port 8000`).
2. Guardar un archivo JSON privado por parcela/actor, con permisos `0600`, fuera
   de Git. Ejemplo **ficticio**:

   ```json
   {
     "protocol": "field_shift_loopback",
     "enabled": true,
     "label": "Parcela piloto",
     "endpoint": "http://127.0.0.1:8000/v1/kusi/capability",
     "parcel": {
       "lat": -12.065,
       "lon": -75.205,
       "current_crop": "potato_continuous",
       "irrigation": "rainfed",
       "priority": "soil_health",
       "soil_texture": "unknown",
       "has_livestock": false,
       "audience": "farmer"
     }
   }
   ```

3. En la configuración **privada y autorizada** del actor de Kusi Kusi, añadir
   una fuente (sin sustituir las existentes):

   ```json
   {"id":"field_shift_pilot","label":"Field Shift / Parcela piloto","kind":"field_shift_loopback","config_file":"/ruta/absoluta/privada/field-shift-source.json"}
   ```

   Ajustar `bridge_script` a la ruta absoluta de
   `backend/kusi_dispatch.py`. Este despachador conserva los protocolos
   `frappe_ssh` y `graphql_loopback` mediante el puente actual de Kusi Kusi.
   Hacer copia del runtime privado antes de editarlo. No copiar la identidad,
   modelo, fuentes ni claves a este repositorio.

4. Si el runtime usa `access`/ACL, conceder explícitamente los dos recursos
   `field_shift_pilot::climate_months` y
   `field_shift_pilot::rotation_options` al principal verificado mediante el
   flujo de Kusi Kusi. No omitir `access` ni rebajar `KUSI_ADAPTIVE_REQUIRE_ACL`.
   Un canal solo puede usar esta fuente después de su vinculación personal
   verificada. El worker local de Kusi Kusi actualmente solo permite fuentes
   `graphql_loopback`; esta fuente corresponde al runtime de staging con ACL.

## Datos y límites

`climate_months` ofrece 12 filas con lluvia, evapotranspiración, balance,
temperatura, humedad y riesgos. `rotation_options` ofrece las alternativas y
su índice orientativo. El índice **no es probabilidad de éxito**. El puente
rechaza respuestas sin datos NASA reales, configuraciones legibles por otros,
redirecciones, destinos externos, planes con coordenadas, campos no publicados,
filtros inválidos y resultados de más de 1 MiB. El modelo solo ve campos
analíticos; las explicaciones completas, vacíos de datos y revisión agronómica
permanecen en la respuesta HTTP `/v1/kusi/capability` para un consumidor
autorizado. Esta fuente analítica no basta por sí sola para que el chat entregue
el informe agronómico completo.

## Comprobación

Desde `backend/`:

```sh
.venv/bin/python -m pytest tests/test_kusi_bridge.py tests/test_data_gaps.py -q
```

Después de registrar la fuente en Kusi Kusi, ejecutar una consulta real del
actor autorizado y comprobar catálogo, lectura y evidencia. Esa aceptación
requiere acceso de escritura al runtime de Kusi Kusi, su principal/ACL y un
servicio Field Shift en loopback. No queda acreditada por las pruebas de este
repositorio.
