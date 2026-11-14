# Plan Técnico 001: Arquitectura Híbrida (Motor Geo-Agro NASA + Kusi Kusi)

## 1. Decisiones de Arquitectura

Se adopta una **arquitectura híbrida desacoplada en 3 capas**:

1. **Motor Geo-Agro NASA (`backend/`, Python + FastAPI):**
   - Responsable de la ingesta de datos climáticos/satelitales, cálculo de indicadores agroclimáticos, comparación de ≥ 3 estrategias de rotación y simulación de escenarios.
   - Expone contrato OpenAPI (`/openapi.json`) consumible tanto por el frontend web como por **Kusi Kusi** (Rust).
2. **Interfaz Web de Mapa y Comparador (`backend/app/static/index.html` para el MVP rápido):**
   - Mapa interactivo **Leaflet** con capas globales de **NASA GIBS** (MODIS/VIIRS NDVI, GPM precipitación, SMAP humedad) + OpenStreetMap.
   - Selector de modo **Vista Agricultor / Vista Agrónomo**.
   - Sliders de escenario climático y calendario visual de 12 meses.
3. **Integración con Kusi Kusi (`/home/blink/kusi-kusi`):**
   - Kusi Kusi es dueño del registro, la identidad verificada, los canales y la respuesta de IA. Field Shift expone hechos y estrategias orientativas en `/v1/kusi/capability`; no autentica al agricultor ni envía mensajes.
   - El asistente debe descubrir la capacidad autorizada, consultar los datos estructurados y redactar en lenguaje cotidiano con cifras y fuente. Si faltan datos NASA, comunica la falta de información y pide reintentar. Nunca presenta el índice orientativo como probabilidad ni confirma siembra sin revisión local.
   - La parcela y prioridades se registran en el formulario móvil de Field Shift tras verificar el WhatsApp mediante Kusi Kusi. El canal solo identifica al usuario si existe un vínculo verificado; una ubicación enviada por chat no crea identidad ni acceso. La actualización desde el canal queda pendiente de autorización y conexión real.
   - Field Shift guarda perfiles y hasta ocho parcelas por perfil en SQLite local. Una prueba HMAC de Kusi Kusi, basada en el remitente WhatsApp ya verificado, habilita un código de acceso de un solo uso. El navegador lo canjea por una cookie HttpOnly; los cambios y análisis de parcelas usan CSRF y comprueban pertenencia. Falta conectar el emisor real de esa prueba en Kusi Kusi.

## 2. Datasets Globales Utilizados (MVP)
- **NASA POWER API** (`power.larc.nasa.gov`): Series climatológicas mensuales globales (`T2M`, `T2M_MIN`, `T2M_MAX`, `PRECTOTCORR`, `RH2M`, `ALLSKY_SFC_SW_DWN`).
- **NASA GIBS WMTS** (`gibs.earthdata.nasa.gov`): Capas visuales globales en el mapa (vegetación NDVI, precipitación GPM, temperatura superficial).
- **Contexto de Vacíos Locales:** Propiedades finas de suelo (pH, textura exacta, compactación) y precios locales de mercado se marcan explícitamente como `information_gaps` para validación local.

## 3. Contratos API

- `GET /health` -> Estado del servicio y conectividad.
- `POST /v1/parcel/diagnose` -> Diagnóstico agroclimático de `(lat, lon)` con indicadores de estrés hídrico, térmico, heladas y suelo desnudo.
- `POST /v1/rotations/compare` -> Compara la rotación actual contra alternativas adaptadas a riego y prioridades del agricultor.
- `POST /v1/scenarios/simulate` -> Recalcula vulnerabilidad y ranking de rotaciones bajo delta de lluvia (`%`) y temperatura (`°C`).
