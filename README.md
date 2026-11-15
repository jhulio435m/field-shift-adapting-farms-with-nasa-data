# Field Shift: Adapting Farms with NASA Data

Agricultural decision-support prototype for farmers exploring climate-resilient crop rotations. Field Shift translates NASA satellite & agro-climatic observations into clear planting comparisons, scenario stress tests (normal climate vs. `-25%` drought), and accessible voice guidance.

## Data Sources & Simulation Transparency

- **30-year agro-climatology (historical baseline):** NASA POWER Agroclimatology API (`T2M`, `T2M_MIN`, `T2M_MAX`, `PRECTOTCORR`, `GWETTOP`, `GWETROOT`, `EVPTRNS`, `ALLSKY_SFC_SW_DWN`, `RH2M`). If NASA POWER is unavailable, the backend returns HTTP `503` instead of fabricating synthetic climate readings; the frontend falls back only to explicitly labeled **verified historical captures** saved earlier for continuity during demos.
- **20-year climate trend & soil correlation (2005–2024):** NASA POWER Monthly historical series (`_pearson_r` rain-to-root-moisture correlation and decade temperature slope).
- **Dated vegetation pass:** Recent cloud-filtered Sentinel-2 L2A point NDVI (`<25%` cloud cover) displaying its exact observation date.
- **Map overlays (explicit mosaic dates):** NASA GIBS WMTS layers (GPM IMERG precipitation, SMAP L4 root-zone soil moisture, MODIS LST nighttime temp, MODIS NDVI 16-Day, MODIS Bands 7-2-1) with selectable dated mosaics (`2026-07-01`, `2026-02-15`, `2025-07-01`).
- **Terrain & 7-day forecast:** Copernicus/SRTM DEM slope (`%` and erosion risk) and 7-day Open-Meteo weather alerts.
- **Kusi Kusi Mobile Simulator:** Local browser simulation (rule-grounded responses + Web Speech API audio) using the live NASA diagnosis computed by the backend.

> **Disclaimer:** This prototype supports field exploration and **does not** replace farmer judgment or local agronomic review (seed availability, soil lab checks, planting dates).

---

## Run with Docker (Recommended for Demo)

Builds the Vite + React + TypeScript frontend (`frontend/dist`) and serves it directly through FastAPI on port `8000`:

```sh
docker compose up -d --build field-shift
```

Open `http://127.0.0.1:8000` (or verify health at `http://127.0.0.1:8000/health`), then click **Entrar a la Demo en 1 Clic** to load both contrasting Peruvian fields without phone verification:
- **Lote Mantaro — Papa** (`-12.0650, -75.2049`, Andean highlands, rainfed)
- **Valle Piura — Arroz** (`-5.1945, -80.6328`, coastal valley, irrigated)

---

## Run Locally (Development)

### 1. Build React Frontend
Note: `app/main.py` serves `frontend/dist/index.html` when built; make sure `npm run build` is executed so the React app (rather than legacy static HTML) is served.

```sh
cd frontend
npm ci
npm run build
```

### 2. Run FastAPI Backend & Tests

```sh
cd backend
uv sync --dev
uv run uvicorn app.main:app --host 127.0.0.1 --port 8000
```

Run backend test suite:

```sh
cd backend
uv run pytest -q
```

Open `http://127.0.0.1:8000`. OpenAPI schema is available at `http://127.0.0.1:8000/openapi.json`.

---

## Kusi Kusi Onboarding Bridge

Field Shift stores farmer profiles locally in `backend/.local-data/` (excluded from Git). Real WhatsApp/Telegram identity verification connects via Kusi Kusi (`POST /v1/onboarding/confirm` with `FIELD_SHIFT_KUSI_VERIFY_KEY`), while `/v1/onboarding/demo` provides instant 1-click access for hackathon demonstrations.
