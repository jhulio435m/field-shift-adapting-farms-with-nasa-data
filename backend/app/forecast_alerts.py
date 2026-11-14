"""7-day rain, storm, and frost alert service (Open-Meteo / ECMWF-GFS)."""

from __future__ import annotations

import httpx
from pydantic import BaseModel

FORECAST_URL = "https://api.open-meteo.com/v1/forecast"
STORM_WMO_CODES = {95, 96, 99}


class DailyForecastItem(BaseModel):
    date: str
    temp_min_c: float
    temp_max_c: float
    precip_mm: float
    precip_prob_pct: int
    weather_code: int
    condition_label: str
    is_storm: bool
    is_rain: bool
    is_frost: bool


class WeeklyAlertSummary(BaseModel):
    live_forecast: bool
    source: str
    rain_expected: bool
    storm_expected: bool
    frost_expected: bool
    alert_level: str  # "ok" | "rain" | "frost" | "storm"
    farmer_alert_message: str
    days: list[DailyForecastItem]


def _wmo_label(code: int) -> str:
    if code in STORM_WMO_CODES:
        return "Tormenta electrica"
    if code in (61, 63, 65, 80, 81, 82):
        return "Lluvia"
    if code in (51, 53, 55):
        return "Llovizna"
    if code in (71, 73, 75, 85, 86):
        return "Nieve / granizo"
    if code in (1, 2, 3):
        return "Nublado"
    return "Despejado"


async def fetch_weekly_alerts(lat: float, lon: float, timeout_seconds: float = 6.0) -> WeeklyAlertSummary:
    """Fetch 7-day forecast and compute actionable rain/storm/frost alerts for the farmer."""
    try:
        async with httpx.AsyncClient(timeout=timeout_seconds) as client:
            resp = await client.get(
                FORECAST_URL,
                params={
                    "latitude": round(lat, 4),
                    "longitude": round(lon, 4),
                    "daily": "weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max",
                    "timezone": "auto",
                },
            )
            resp.raise_for_status()
            daily = resp.json()["daily"]

        dates = daily["time"]
        codes = daily["weather_code"]
        t_maxs = daily["temperature_2m_max"]
        t_mins = daily["temperature_2m_min"]
        precips = daily["precipitation_sum"]
        probs = daily.get("precipitation_probability_max", [0] * len(dates))

        days: list[DailyForecastItem] = []
        storm_days: list[str] = []
        rain_days: list[str] = []
        frost_days: list[str] = []

        for i in range(min(7, len(dates))):
            code = int(codes[i] or 0)
            p_mm = float(precips[i] or 0.0)
            prob = int(probs[i] or 0)
            t_min = float(10.0 if t_mins[i] is None else t_mins[i])
            t_max = float(20.0 if t_maxs[i] is None else t_maxs[i])

            is_storm = code in STORM_WMO_CODES or p_mm >= 20.0
            is_rain = is_storm or p_mm >= 2.0 or prob >= 55
            is_frost = t_min <= 2.0

            if is_storm:
                storm_days.append(dates[i])
            elif is_rain:
                rain_days.append(dates[i])
            if is_frost:
                frost_days.append(dates[i])

            days.append(
                DailyForecastItem(
                    date=dates[i],
                    temp_min_c=round(t_min, 1),
                    temp_max_c=round(t_max, 1),
                    precip_mm=round(p_mm, 1),
                    precip_prob_pct=prob,
                    weather_code=code,
                    condition_label=_wmo_label(code),
                    is_storm=is_storm,
                    is_rain=is_rain,
                    is_frost=is_frost,
                )
            )

        storm_expected = len(storm_days) > 0
        rain_expected = len(rain_days) > 0 or storm_expected
        frost_expected = len(frost_days) > 0

        messages: list[str] = []
        if storm_expected:
            messages.append(
                f"ALERTA DE TORMENTA / LLUVIA FUERTE ({', '.join(storm_days)}): "
                "revisa drenajes, evita aplicar insumos antes de la lluvia y protege suelos en pendiente."
            )
            alert_level = "storm"
        elif frost_expected:
            messages.append(
                f"ALERTA DE HELADA ({', '.join(frost_days)}): "
                "temperaturas cercanas o menores a 2 °C; protege cultivos sensibles."
            )
            alert_level = "frost"
        elif rain_expected:
            messages.append(
                f"AVISO DE LLUVIA esta semana ({len(rain_days)} dias con lluvia probable): "
                "buen momento para aprovechar humedad si estas por sembrar."
            )
            alert_level = "rain"
        else:
            messages.append("Sin tormentas, lluvias fuertes ni heladas en los proximos 7 dias.")
            alert_level = "ok"

        if storm_expected and frost_expected:
            messages.append(f"Tambien hay riesgo de helada en {', '.join(frost_days)}.")

        return WeeklyAlertSummary(
            live_forecast=True,
            source="Open-Meteo (ECMWF/GFS Pronostico 7 dias)",
            rain_expected=rain_expected,
            storm_expected=storm_expected,
            frost_expected=frost_expected,
            alert_level=alert_level,
            farmer_alert_message=" ".join(messages),
            days=days,
        )
    except Exception:
        return WeeklyAlertSummary(
            live_forecast=False,
            source="Pronostico 7 dias no disponible (sin conexion)",
            rain_expected=False,
            storm_expected=False,
            frost_expected=False,
            alert_level="ok",
            farmer_alert_message="No se pudo consultar el pronostico de tormentas/lluvia de los proximos 7 dias.",
            days=[],
        )
