"""FastAPI entrypoint for the Field Shift NASA Geo-Agro Decision-Support Engine."""

from __future__ import annotations

import asyncio
from contextlib import asynccontextmanager
import hashlib
import hmac
import os
from pathlib import Path
import time
from uuid import uuid4

from fastapi import Cookie, FastAPI, Header, HTTPException, Request, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from .correlation import fetch_historical_correlation
from .forecast_alerts import WeeklyAlertSummary, fetch_weekly_alerts
from .nasa_power import build_diagnosis, fetch_parcel_environment
from .localization import alert_en, comparison_en, diagnosis_en
from . import onboarding
from .rotation_engine import compare_rotations
from .schemas import (
    AlertCheckRequest,
    BotConfigRequest,
    KusiCapabilityRequest,
    KusiCapabilityResponse,
    ParcelDiagnoseRequest,
    ParcelDiagnoseResponse,
    RotationCompareRequest,
    RotationCompareResponse,
    FarmerStartRequest,
    KusiVerifyProof,
    ClaimRequest,
    ParcelCreateRequest,
    ScenarioRequest,
    DemoStartRequest,
)

STATIC_DIR = Path(__file__).parent / "static"
FRONTEND_DIST_DIR = Path(__file__).resolve().parents[2] / "frontend" / "dist"

@asynccontextmanager
async def lifespan(_: FastAPI):
    # Kusi Kusi owns messaging channels and AI responses; this service only supplies evidence.
    yield


app = FastAPI(
    title="Field Shift — Sistema Adaptador al Cambio Climatico (NASA)",
    version="1.1.0",
    lifespan=lifespan,
    description=(
        "Motor Geo-Agro de apoyo a la decision para rotacion de cultivos basado en "
        "NASA POWER (clima, humedad de suelo, evapotranspiracion), DEM/SRTM (pendiente/erosion) "
        "y alertas de lluvia/tormenta/helada a 7 dias. Integrable via OpenAPI con Kusi Kusi y Telegram."
    ),
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=[],
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)


if (FRONTEND_DIST_DIR / "assets").is_dir():
    app.mount("/assets", StaticFiles(directory=FRONTEND_DIST_DIR / "assets"), name="frontend-assets")


@app.get("/", include_in_schema=False)
async def serve_ui() -> FileResponse:
    dist_index = FRONTEND_DIST_DIR / "index.html"
    if dist_index.is_file():
        return FileResponse(dist_index)
    return FileResponse(STATIC_DIR / "index.html")


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok", "service": "field-shift-nasa", "version": "1.1.0"}


@app.post("/v1/alerts/check", response_model=WeeklyAlertSummary)
async def check_weekly_alerts(req: AlertCheckRequest) -> WeeklyAlertSummary:
    alerts = await fetch_weekly_alerts(req.lat, req.lon)
    if req.language == "en":
        alert_en(alerts)
    return alerts


@app.post("/v1/parcel/diagnose", response_model=ParcelDiagnoseResponse)
async def diagnose_parcel(req: ParcelDiagnoseRequest) -> ParcelDiagnoseResponse:
    (raw, live, source, terrain), alerts, corr = await asyncio.gather(
        fetch_parcel_environment(req.lat, req.lon),
        fetch_weekly_alerts(req.lat, req.lon),
        fetch_historical_correlation(req.lat, req.lon),
    )
    if not live:
        raise HTTPException(status_code=503, detail="NASA POWER no disponible; no se genera un diagnostico con datos sinteticos")
    diag = build_diagnosis(
        lat=req.lat,
        lon=req.lon,
        raw_params=raw,
        live_nasa_data=live,
        data_source=source,
        terrain=terrain,
        soil_texture=req.soil_texture,
        rain_delta_pct=req.rain_delta_pct,
        temp_delta_c=req.temp_delta_c,
    )
    diag.weekly_alerts = alerts
    diag.historical_correlation = corr
    if req.language == "en":
        diagnosis_en(diag, req.rain_delta_pct, req.temp_delta_c)
    return diag


@app.post("/v1/rotations/compare", response_model=RotationCompareResponse)
async def compare_parcel_rotations(req: RotationCompareRequest) -> RotationCompareResponse:
    (raw, live, source, terrain), alerts, corr = await asyncio.gather(
        fetch_parcel_environment(req.lat, req.lon),
        fetch_weekly_alerts(req.lat, req.lon),
        fetch_historical_correlation(req.lat, req.lon),
    )
    if not live:
        raise HTTPException(status_code=503, detail="NASA POWER no disponible; no se generan recomendaciones con datos sinteticos")
    diagnosis = build_diagnosis(
        lat=req.lat,
        lon=req.lon,
        raw_params=raw,
        live_nasa_data=live,
        data_source=source,
        terrain=terrain,
        soil_texture=req.soil_texture,
        rain_delta_pct=req.rain_delta_pct,
        temp_delta_c=req.temp_delta_c,
    )
    diagnosis.weekly_alerts = alerts
    diagnosis.historical_correlation = corr
    strategies = compare_rotations(
        diagnosis=diagnosis,
        current_crop=req.current_crop,
        irrigation=req.irrigation,
        priority=req.priority,
        soil_texture=req.soil_texture,
        has_livestock=req.has_livestock,
    )
    comparison = RotationCompareResponse(
        diagnosis=diagnosis,
        strategies=strategies,
        human_judgment_reminder=(
            "Indice y calendario orientativos basados en reglas; faltan fechas de siembra, datos de suelo y validacion local. "
            "La herramienta no reemplaza la experiencia del agricultor ni la revision del agronomo local."
        ),
    )
    if req.language == "en":
        comparison_en(comparison, req.current_crop, req.soil_texture,
                      req.rain_delta_pct, req.temp_delta_c)
    return comparison


@app.post("/v1/scenarios/simulate", response_model=RotationCompareResponse)
async def simulate_climate_scenario(req: RotationCompareRequest) -> RotationCompareResponse:
    """Simula cambios de lluvia (%) y temperatura (C) en tiempo real."""
    return await compare_parcel_rotations(req)


@app.post("/v1/kusi/capability", response_model=KusiCapabilityResponse)
async def execute_for_kusi_kusi(req: KusiCapabilityRequest) -> KusiCapabilityResponse:
    """Structured evidence for Kusi Kusi; the AI layer writes the user-facing answer."""
    comparison = await compare_parcel_rotations(
        RotationCompareRequest(
            lat=req.lat,
            lon=req.lon,
            current_crop=req.current_crop,
            irrigation=req.irrigation,
            priority=req.priority,
            soil_texture=req.soil_texture,
            has_livestock=req.has_livestock,
            language=req.language,
        )
    )
    diag = comparison.diagnosis
    return KusiCapabilityResponse(
        strategies_count=len(comparison.strategies),
        live_nasa_data=diag.live_nasa_data,
        live_terrain_data=diag.terrain.live_terrain_data,
        raw_comparison=comparison,
    )


@app.post("/v1/telegram/webhook", deprecated=True)
async def telegram_webhook() -> None:
    """Legacy route disabled: messaging and AI interpretation belong to Kusi Kusi."""
    raise HTTPException(status_code=410, detail="Usa el canal Telegram verificado en Kusi Kusi")


@app.post("/v1/onboarding/start")
async def start_onboarding(req: FarmerStartRequest) -> dict:
    result = onboarding.start(
        req.name,
        req.phone,
        req.model_dump(exclude={"name", "phone"}),
    )
    if os.environ.get("FIELD_SHIFT_DEMO_MODE") == "1":
        key = onboarding.verifier_key()
        when = int(time.time())
        event_id = f"demo-{uuid4()}"
        identity = req.phone or f"tg:{req.telegram_handle.strip().lower()}"
        msg = f"{result['challenge']}\n{identity}\n{when}\n{event_id}".encode()
        sig = hmac.new(key, msg, hashlib.sha256).hexdigest()
        result["claim_code"] = onboarding.confirm(
            result["challenge"], identity, when, event_id, sig
        )
        result["demo_mode"] = True
    return result


@app.post("/v1/onboarding/confirm")
async def confirm_from_kusi(req: KusiVerifyProof) -> dict:
    """Trusted Kusi service only: return a one-use code for the verified sender."""
    code = onboarding.confirm(req.challenge, req.phone, req.verified_at,
                              req.event_id, req.signature)
    return {"claim_code": code, "expires_in_seconds": 600}


@app.post("/v1/onboarding/claim")
async def claim_session(req: ClaimRequest, response: Response, request: Request) -> dict:
    token, csrf = onboarding.claim(req.code)
    response.set_cookie("field_shift_session", token, httponly=True, samesite="lax",
                        secure=request.url.scheme == "https", max_age=86400)
    return {"status": "verified", "csrf": csrf}


@app.post("/v1/onboarding/demo")
async def demo_onboarding(req: DemoStartRequest, response: Response, request: Request) -> dict:
    token, csrf = onboarding.start_demo(req.model_dump())
    response.set_cookie("field_shift_session", token, httponly=True, samesite="lax",
                        secure=request.url.scheme == "https", max_age=86400)
    return {"status": "verified", "csrf": csrf, "demo_mode": True}


@app.post("/v1/onboarding/logout")
async def logout_session(
    response: Response,
    field_shift_session: str | None = Cookie(default=None),
) -> dict:
    onboarding.logout(field_shift_session)
    response.delete_cookie("field_shift_session")
    return {"status": "logged_out"}


@app.get("/v1/me")
async def me(field_shift_session: str | None = Cookie(default=None)) -> dict:
    farmer = onboarding.farmer(field_shift_session)
    return {
        "name": farmer["name"],
        "phone_last4": farmer["phone_last4"],
        "csrf": farmer["csrf"],
        "bot_config": onboarding.bot_config(farmer),
        "parcels": onboarding.list_parcels(farmer["id"]),
    }


@app.put("/v1/bot/config")
async def update_bot_config(
    req: BotConfigRequest,
    field_shift_session: str | None = Cookie(default=None),
    x_csrf_token: str | None = Header(default=None),
) -> dict:
    farmer = onboarding.farmer(field_shift_session)
    onboarding.require_csrf(farmer, x_csrf_token)
    return onboarding.update_bot_config(farmer["id"], req.model_dump())


@app.get("/v1/bot/kusi-source")
async def get_kusi_source_config(
    parcel_id: str | None = None,
    field_shift_session: str | None = Cookie(default=None),
) -> dict:
    farmer = onboarding.farmer(field_shift_session)
    cfg = onboarding.bot_config(farmer)
    parcels = onboarding.list_parcels(farmer["id"])
    target = (
        next((p for p in parcels if p["id"] == parcel_id), None)
        if parcel_id
        else (parcels[0] if parcels else None)
    )
    return {
        "protocol": "field_shift_loopback",
        "enabled": cfg["alerts_enabled"],
        "channel": cfg["preferred_channel"],
        "telegram_handle": cfg["telegram_handle"],
        "alert_frequency": cfg["alert_frequency"],
        "label": target["name"] if target else farmer["name"],
        "endpoint": "http://127.0.0.1:8000/v1/kusi/capability",
        "parcel": {
            "lat": target["lat"] if target else -12.065,
            "lon": target["lon"] if target else -75.2049,
            "current_crop": target["current_crop"] if target else "maize_continuous",
            "irrigation": target["irrigation"] if target else "rainfed",
            "priority": target["priority"] if target else "water",
            "soil_texture": target["soil_texture"] if target else "unknown",
            "has_livestock": bool(target["has_livestock"]) if target else False,
            "audience": cfg["bot_audience"],
            "language": cfg["bot_language"],
        },
    }


@app.post("/v1/parcels")
async def create_parcel(req: ParcelCreateRequest,
                        field_shift_session: str | None = Cookie(default=None),
                        x_csrf_token: str | None = Header(default=None)) -> dict:
    farmer = onboarding.farmer(field_shift_session)
    onboarding.require_csrf(farmer, x_csrf_token)
    return onboarding.add_parcel(farmer["id"], req.model_dump(exclude={"rain_delta_pct", "temp_delta_c"}))


@app.put("/v1/parcels/{parcel_id}")
async def update_registered_parcel(
    parcel_id: str,
    req: ParcelCreateRequest,
    field_shift_session: str | None = Cookie(default=None),
    x_csrf_token: str | None = Header(default=None),
) -> dict:
    farmer = onboarding.farmer(field_shift_session)
    onboarding.require_csrf(farmer, x_csrf_token)
    return onboarding.update_parcel(
        farmer["id"],
        parcel_id,
        req.model_dump(exclude={"rain_delta_pct", "temp_delta_c"}),
    )


@app.delete("/v1/parcels/{parcel_id}")
async def delete_registered_parcel(
    parcel_id: str,
    field_shift_session: str | None = Cookie(default=None),
    x_csrf_token: str | None = Header(default=None),
) -> dict:
    farmer = onboarding.farmer(field_shift_session)
    onboarding.require_csrf(farmer, x_csrf_token)
    onboarding.delete_parcel(farmer["id"], parcel_id)
    return {"status": "deleted", "id": parcel_id}


@app.post("/v1/parcels/{parcel_id}/analyze", response_model=RotationCompareResponse)
async def analyze_registered_parcel(parcel_id: str,
                                    scenario: ScenarioRequest,
                                    field_shift_session: str | None = Cookie(default=None),
                                    x_csrf_token: str | None = Header(default=None)) -> RotationCompareResponse:
    farmer = onboarding.farmer(field_shift_session)
    onboarding.require_csrf(farmer, x_csrf_token)
    parcel = onboarding.get_parcel(farmer["id"], parcel_id)
    return await compare_parcel_rotations(RotationCompareRequest(
        **{**parcel, **scenario.model_dump()}))
