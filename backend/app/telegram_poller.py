"""Background Telegram long-polling worker for local/hackathon live bot replies."""

from __future__ import annotations

import asyncio
import logging
import os
import re
from typing import Any

import httpx

from .nasa_power import build_diagnosis, fetch_parcel_environment
from .forecast_alerts import fetch_weekly_alerts
from .rotation_engine import compare_rotations

logger = logging.getLogger(__name__)
COORD_PATTERN = re.compile(r"(-?\d{1,2}\.\d+)\s*[, ]\s*(-?\d{1,3}\.\d+)")


async def _handle_message(msg: dict[str, Any]) -> tuple[int, str] | None:
    chat = msg.get("chat") or {}
    chat_id = chat.get("id")
    if chat_id is None:
        return None

    text = str(msg.get("text") or "").strip()
    loc = msg.get("location") or {}
    lat: float | None = loc.get("latitude")
    lon: float | None = loc.get("longitude")

    if lat is None or lon is None:
        match = COORD_PATTERN.search(text)
        if match:
            lat = float(match.group(1))
            lon = float(match.group(2))

    if lat is None or lon is None:
        return (
            int(chat_id),
            "Hola, soy Field Shift (NASA + Alertas de Campo).\n"
            "Enviame tu ubicacion de Telegram (clip -> Ubicacion) o escribe tus coordenadas "
            "(ejemplo: -12.065, -75.205) para avisarte si habra lluvia o tormenta y sugerirte rotaciones.",
        )

    (raw, live, source, terrain), alerts = await asyncio.gather(
        fetch_parcel_environment(lat, lon),
        fetch_weekly_alerts(lat, lon),
    )
    if not live:
        return int(chat_id), "NASA POWER no esta disponible. Intenta de nuevo; no generamos recomendaciones con datos inventados."
    diag = build_diagnosis(
        lat=lat,
        lon=lon,
        raw_params=raw,
        live_nasa_data=live,
        data_source=source,
        terrain=terrain,
    )
    diag.weekly_alerts = alerts
    strategies = compare_rotations(
        diagnosis=diag,
        current_crop="maize_continuous",
        irrigation="rainfed",
        priority="water",
        soil_texture="unknown",
        has_livestock=False,
    )
    best = next((s for s in strategies if not s.is_current), strategies[0])
    reply = (
        f"Parcela ({diag.lat}, {diag.lon})\n"
        f"Terreno: {f'{diag.terrain.elevation_m} m, pendiente {diag.terrain.slope_pct}%' if diag.terrain.live_terrain_data else 'sin datos de elevacion'}\n\n"
        f"ALERTA 7 DIAS: {alerts.farmer_alert_message}\n\n"
        f"CLIMA NASA: {diag.farmer_summary}\n\n"
        f"IDEA PARA EVALUAR (indice orientativo {best.fit_score}/100): {best.name}\n"
        f"Secuencia: {' -> '.join(best.sequence)}\n"
        f"{best.why_suggested_farmer}\n\n"
        f"Nota: {diag.disclaimer}"
    )
    return int(chat_id), reply


async def run_telegram_polling(stop_event: asyncio.Event) -> None:
    """Poll Telegram Bot API getUpdates until stop_event is set."""
    token = os.environ.get("TELEGRAM_BOT_TOKEN", "").strip()
    if not token:
        return

    base_url = f"https://api.telegram.org/bot{token}"
    offset = 0
    logger.info("Telegram polling worker started.")

    async with httpx.AsyncClient(timeout=35.0) as client:
        try:
            await client.post(f"{base_url}/deleteWebhook")
        except Exception as exc:
            logger.warning("Could not clear Telegram webhook: %s", exc)

        while not stop_event.is_set():
            try:
                resp = await client.get(
                    f"{base_url}/getUpdates",
                    params={"offset": offset, "timeout": 20},
                )
                if resp.status_code != 200:
                    await asyncio.sleep(3.0)
                    continue
                updates = resp.json().get("result", [])
                for upd in updates:
                    offset = max(offset, int(upd["update_id"]) + 1)
                    msg = upd.get("message") or upd.get("edited_message")
                    if not msg:
                        continue
                    handled = await _handle_message(msg)
                    if handled is None:
                        continue
                    chat_id, reply_text = handled
                    await client.post(
                        f"{base_url}/sendMessage",
                        json={"chat_id": chat_id, "text": reply_text},
                    )
            except asyncio.CancelledError:
                break
            except Exception as exc:
                logger.warning("Telegram polling error: %s", exc)
                await asyncio.sleep(3.0)
