"""20-year NASA POWER historical trend & rainfall-soil-moisture correlation + Sentinel-2 STAC availability."""

from __future__ import annotations

import asyncio
import datetime
import math
import httpx
from pydantic import BaseModel

NASA_MONTHLY_URL = "https://power.larc.nasa.gov/api/temporal/monthly/point"
STAC_SEARCH_URL = "https://planetarycomputer.microsoft.com/api/stac/v1/search"
TITILER_POINT_URL = "https://planetarycomputer.microsoft.com/api/data/v1/item/point"


class HistoricalCorrelationSummary(BaseModel):
    live_historical: bool
    years_analyzed: int
    temp_trend_c_per_decade: float
    precip_change_pct_recent_5y: float
    rain_soil_moisture_corr: float
    drought_years_last_20: int
    sentinel2_clear_scenes_12m: int
    latest_ndvi: float | None = None
    latest_ndvi_date: str | None = None
    vegetation_status: str = "sin_dato"
    correlation_interpretation_farmer: str
    correlation_interpretation_agronomist: str


def _pearson_r(xs: list[float], ys: list[float]) -> float:
    n = len(xs)
    if n < 3:
        return 0.0
    mx = sum(xs) / n
    my = sum(ys) / n
    num = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    den_x = math.sqrt(sum((x - mx) ** 2 for x in xs))
    den_y = math.sqrt(sum((y - my) ** 2 for y in ys))
    if den_x == 0.0 or den_y == 0.0:
        return 0.0
    return max(-1.0, min(1.0, num / (den_x * den_y)))


def _linear_slope_per_decade(years: list[int], values: list[float]) -> float:
    n = len(years)
    if n < 3:
        return 0.0
    mx = sum(years) / n
    my = sum(values) / n
    num = sum((y - mx) * (v - my) for y, v in zip(years, values))
    den = sum((y - mx) ** 2 for y in years)
    if den == 0.0:
        return 0.0
    return (num / den) * 10.0


async def _fetch_sentinel2_ndvi(lat: float, lon: float) -> tuple[int, float | None, str | None, str]:
    """Count recent low-cloud Sentinel-2 L2A scenes and compute 10m NDVI for the latest clear pass."""
    try:
        end_date = datetime.datetime.now(datetime.timezone.utc).date()
        start_date = end_date - datetime.timedelta(days=365)
        async with httpx.AsyncClient(timeout=6.0) as client:
            resp = await client.post(
                STAC_SEARCH_URL,
                json={
                    "collections": ["sentinel-2-l2a"],
                    "intersects": {"type": "Point", "coordinates": [round(lon, 4), round(lat, 4)]},
                    "datetime": f"{start_date}T00:00:00Z/{end_date}T23:59:59Z",
                    "query": {"eo:cloud_cover": {"lt": 25}},
                    "sortby": [{"field": "properties.datetime", "direction": "desc"}],
                    "limit": 24,
                },
            )
            resp.raise_for_status()
            features = resp.json().get("features", [])
            count = len(features)
            if not features:
                return 0, None, None, "sin_dato"

            latest = features[0]
            item_id = latest["id"]
            dt = str(latest.get("properties", {}).get("datetime", ""))[:10]
            pt_resp = await client.get(
                f"{TITILER_POINT_URL}/{round(lon, 5)},{round(lat, 5)}",
                params={"collection": "sentinel-2-l2a", "item": item_id, "assets": ["B04", "B08"]},
            )
            pt_resp.raise_for_status()
            vals = pt_resp.json().get("values", [])
            if len(vals) >= 2 and vals[0] is not None and vals[1] is not None:
                b04, b08 = float(vals[0]), float(vals[1])
                if not all(math.isfinite(value) and value >= 0 for value in (b04, b08)) or b04 + b08 <= 0:
                    return count, None, dt, "sin_dato"
                ndvi = round((b08 - b04) / (b08 + b04), 3)
                if ndvi < 0.20:
                    status = "suelo_desnudo_o_sin_cobertura"
                elif ndvi < 0.45:
                    status = "cobertura_baja_o_estres"
                else:
                    status = "vegetacion_vigorosa"
                return count, ndvi, dt, status
            return count, None, dt, "sin_dato"
    except Exception:
        return 0, None, None, "sin_dato"


async def fetch_historical_correlation(lat: float, lon: float) -> HistoricalCorrelationSummary:
    """Analyze 2005-2024 NASA POWER monthly series and Sentinel-2 10m NDVI."""
    power_task = _fetch_power_20y(lat, lon)
    s2_task = _fetch_sentinel2_ndvi(lat, lon)
    power_data, (s2_count, ndvi, ndvi_date, veg_status) = await asyncio.gather(power_task, s2_task)

    if power_data is None:
        return HistoricalCorrelationSummary(
            live_historical=False,
            years_analyzed=0,
            temp_trend_c_per_decade=0.0,
            precip_change_pct_recent_5y=0.0,
            rain_soil_moisture_corr=0.0,
            drought_years_last_20=0,
            sentinel2_clear_scenes_12m=s2_count,
            latest_ndvi=ndvi,
            latest_ndvi_date=ndvi_date,
            vegetation_status=veg_status,
            correlation_interpretation_farmer="No se pudo consultar la tendencia historica de 20 anos en este momento.",
            correlation_interpretation_agronomist="Serie historica 2005-2024 no disponible (timeout).",
        )

    t2m = power_data.get("T2M", {})
    prec = power_data.get("PRECTOTCORR", {})
    gwet = power_data.get("GWETTOP", {})

    monthly_rain: list[float] = []
    monthly_wet: list[float] = []
    yearly_temp: dict[int, list[float]] = {}
    yearly_rain: dict[int, list[float]] = {}

    for ym, p_val in prec.items():
        if len(ym) != 6 or ym.endswith("13"):
            continue
        year = int(ym[:4])
        p = float(p_val)
        w = float(gwet.get(ym, -999))
        t = float(t2m.get(ym, -999))
        if p >= 0 and w >= 0:
            monthly_rain.append(p)
            monthly_wet.append(w)
        if t > -100:
            yearly_temp.setdefault(year, []).append(t)
        if p >= 0:
            yearly_rain.setdefault(year, []).append(p)

    years = sorted(y for y in yearly_temp if len(yearly_temp[y]) >= 10)
    annual_temps = [sum(yearly_temp[y]) / len(yearly_temp[y]) for y in years]
    temp_trend = round(_linear_slope_per_decade(years, annual_temps), 2)

    rain_years = sorted(y for y in yearly_rain if len(yearly_rain[y]) >= 10)
    annual_rains = {y: (sum(yearly_rain[y]) / len(yearly_rain[y])) * 365.0 for y in rain_years}
    if len(rain_years) >= 8:
        baseline_rains = [annual_rains[y] for y in rain_years[:-5]]
        recent_rains = [annual_rains[y] for y in rain_years[-5:]]
        base_avg = sum(baseline_rains) / len(baseline_rains)
        rec_avg = sum(recent_rains) / len(recent_rains)
        precip_change_pct = round(((rec_avg - base_avg) / max(1.0, base_avg)) * 100.0, 1)
        mean_all_rain = sum(annual_rains.values()) / len(annual_rains)
        drought_years = sum(1 for v in annual_rains.values() if v < 0.8 * mean_all_rain)
    else:
        precip_change_pct = 0.0
        drought_years = 0

    corr = round(_pearson_r(monthly_rain, monthly_wet), 2)

    corr_farmer = (f"La lluvia y la humedad superficial estimada varian juntas (r={corr:.2f}). "
                   "Esta correlacion regional no mide la capacidad de retencion de tu parcela.")

    trend_farmer = (
        f"En la serie 2005-2024 la tendencia de temperatura fue {temp_trend:+.2f} °C por decada "
        f"y la lluvia reciente cambio {precip_change_pct:+.1f}% ({drought_years} anos secos)."
    )

    ndvi_note_agro = f" | Sentinel-2 NDVI={ndvi} ({ndvi_date}, {veg_status})" if ndvi is not None else ""
    return HistoricalCorrelationSummary(
        live_historical=True,
        years_analyzed=len(years),
        temp_trend_c_per_decade=temp_trend,
        precip_change_pct_recent_5y=precip_change_pct,
        rain_soil_moisture_corr=corr,
        drought_years_last_20=drought_years,
        sentinel2_clear_scenes_12m=s2_count,
        latest_ndvi=ndvi,
        latest_ndvi_date=ndvi_date,
        vegetation_status=veg_status,
        correlation_interpretation_farmer=f"{trend_farmer} {corr_farmer}",
        correlation_interpretation_agronomist=(
            f"NASA POWER 20y ({len(years)} anos): tendencia termica {temp_trend:+.2f} C/decada, "
            f"delta lluvia ultimos 5 anos {precip_change_pct:+.1f}%, anos secos (<80% media): {drought_years}, "
            f"Pearson r(PRECTOTCORR, GWETTOP)={corr:.2f}. Escenas Sentinel-2 L2A (<25% nubes): {s2_count}{ndvi_note_agro}."
        ),
    )


async def _fetch_power_20y(lat: float, lon: float) -> dict[str, dict[str, float]] | None:
    try:
        async with httpx.AsyncClient(timeout=8.0) as client:
            resp = await client.get(
                NASA_MONTHLY_URL,
                params={
                    "parameters": "T2M,PRECTOTCORR,GWETTOP",
                    "community": "AG",
                    "longitude": round(lon, 4),
                    "latitude": round(lat, 4),
                    "start": "2005",
                    "end": "2024",
                    "format": "JSON",
                },
            )
            resp.raise_for_status()
            return resp.json()["properties"]["parameter"]
    except Exception:
        return None
