# Spec 001: Field Shift — Sistema Adaptador al Cambio Climático (MVP)

## 1. Contexto y Metodología (Soft -> Hard Systems)

### 1.1 Soft Systems: *Weltanschauungen* y Situación Problemática
La agricultura enfrenta variabilidad climática extrema, escasez de agua y degradación del suelo. Coexisten visiones en tensión:
- **Sustento inmediato:** *"Hay que sembrar lo que rinda y se venda hoy."*
- **Cuidado del suelo:** *"Si sembramos siempre lo mismo, la tierra se cansa y se erosiona."*
- **Incertidumbre climática:** *"El clima ya no avisa como antes; detectamos el daño cuando ya es tarde."*
- **Miedo al cambio:** *"Cambiar de cultivo arriesga agua, maquinaria y comprador."*
- **Soberanía del agricultor:** *"La herramienta debe ayudar y explicar en palabras claras, nunca reemplazar el criterio del agricultor ni del agrónomo."*

### 1.2 Hard Systems: Transformación `S0 -> Sistema -> S1`
- **S0 (Situación Actual):** Agricultor con **poca adaptabilidad al cambio climático** (toma decisiones por costumbre o presión de mercado, sin visibilidad oportuna de tendencias hídricas/térmicas ni comparativa segura de rotaciones).
- **Sistema:** **Sistema Adaptador al Cambio Climático** (Motor Geo-Agro NASA + Capa Conversacional/Gobernanza Kusi Kusi + Visualizador Geoespacial).
- **S1 (Situación Deseada):** Agricultor **capaz de adaptarse al cambio climático** (evalúa vulnerabilidades de su parcela, compara estrategias de rotación con evidencia transparente y valida decisiones con apoyo local).

---

## 2. Usuarios y Actores
1. **Agricultor / Productor:** Ingresa ubicación de su parcela, rotación actual, tipo de riego y prioridades; recibe diagnóstico y opciones en lenguaje cotidiano.
2. **Agrónomo / Extensionista Local:** Revisa la evidencia técnica (índices, anomalías, incertidumbre) y valida en campo los vacíos de información.
3. **Kusi Kusi (Capa Integradora):** Conecta canales cercanos al agricultor (Telegram / WhatsApp / Web) con el Motor Geo-Agro NASA vía contrato OpenAPI.

### Recorrido acordado del agricultor

1. Completa un formulario breve con nombre y WhatsApp en formato internacional.
2. Kusi Kusi verifica que controla ese número mediante el canal vinculado. El
   número escrito en el formulario no concede acceso por sí solo.
3. Registra una o varias parcelas con ubicación, cultivo actual, riego, suelo y
   prioridad. Puede seleccionar cada parcela para compararla por separado.
4. Para cada parcela consulta NASA y compara la rotación actual con alternativas,
   evidencia, vacíos de información, calendario y escenarios climáticos.
5. El agrónomo o agricultor revisa la decisión localmente; la herramienta no
   declara una siembra segura ni sustituye la experiencia de campo.

---

## 3. Requerimientos Funcionales (MVP Global)

- **RF-01 (Selección Global de Parcela):** El usuario puede seleccionar cualquier coordenada `(lat, lon)` del planeta en un mapa interactivo o enviarla desde Kusi Kusi.
- **RF-02 (Perfil y Prioridades del Agricultor):** Captura rotación actual, disponibilidad de riego (`rainfed`, `limited`, `irrigated`) y prioridad principal (`water`, `soil_health`, `pest_disruption`, `forage_income`).
- **RF-03 (Diagnóstico Climático NASA en Vivo):** Consulta **NASA POWER** para obtener climatología histórica mensual (temperatura, precipitación, radiación, humedad) y detectar meses de **sequía/estrés hídrico**, **estrés térmico**, **riesgo de heladas** y **ventanas de suelo desnudo**.
- **RF-04 (Comparador de ≥ 3 Estrategias de Rotación):** Evalúa la **rotación actual** frente a al menos **2 estrategias alternativas** (Ahorro de Agua, Regeneración de Suelo/Nitrógeno, Mixta/Forraje).
- **RF-05 (Explicabilidad e Incertidumbre Obligatoria):** Cada estrategia debe incluir:
  - Explicación en **lenguaje agricultor** y **lenguaje técnico/agrónomo**.
  - Evidencia basada en datos NASA.
  - Beneficios esperados (agua, materia orgánica, fijación de nitrógeno, control de plagas, forraje/mercado).
  - **Vacíos de información** (*information gaps*) y puntos que requieren **revisión local**.
- **RF-06 (Simulador de Escenarios Climáticos):** Sliders para ajustar cambio en precipitación (`-40%` a `+30%`) y temperatura (`0°C` a `+3.5°C`) recalculando el riesgo de las rotaciones en tiempo real.
- **RF-07 (Calendario de 12 Meses):** Visualización mensual de cobertura del suelo vs. meses de suelo desnudo/vulnerable.
- **RF-08 (Contrato HTTP / OpenAPI para Kusi Kusi):** Endpoints REST documentados (`/v1/parcel/diagnose`, `/v1/rotations/compare`, `/v1/scenarios/simulate`) consumibles por adaptadores externos.
- **RF-09 (Evidencia visible):** Mostrar en ambas vistas la tendencia de temperatura de la serie 2005–2024, la correlación mensual lluvia–humedad superficial estimada y el NDVI puntual Sentinel-2 más reciente disponible, con fecha, fuente y límites de interpretación. Si falta un dato, mostrarlo como no disponible y no como cero.
- **RF-10 (Dos idiomas):** Interfaz y explicaciones del diagnóstico y las rotaciones en español e inglés, manteniendo los mismos indicadores y cálculos. El idioma se envía explícitamente al API; español es el valor por defecto.

---

## 4. Requerimientos No Funcionales y Restricciones
- **Restricción de Fuentes:** Solo datos abiertos de NASA, USGS, USDA, ESA/Copernicus, ECMWF y FAO (ningún recurso ni servicio proveniente de China).
- **Apoyo, no reemplazo:** Toda salida debe recordar explícitamente que la decisión final pertenece al agricultor y al experto local.
- **Tolerancia a fallos:** Si la API externa de NASA POWER no responde o hay timeout, el servicio debe indicar claramente el estado de conectividad y degradar de forma transparente sin inventar datos ocultos.
