"""Clients for NASA POWER Agroclimatology API and SRTM/DEM Terrain Slope."""

from __future__ import annotations

import asyncio
import math
import httpx

from .schemas import MonthlyClimate, ParcelDiagnoseResponse, SoilTextureType, TerrainInfo

MONTH_LABELS = [
    "Ene", "Feb", "Mar", "Abr", "May", "Jun",
    "Jul", "Ago", "Sep", "Oct", "Nov", "Dic",
]
MONTH_KEYS = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN",
    "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
]
DAYS_IN_MONTH = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]

NASA_POWER_URL = "https://power.larc.nasa.gov/api/temporal/climatology/point"
NASA_PARAMS = "T2M,T2M_MIN,T2M_MAX,PRECTOTCORR,RH2M,ALLSKY_SFC_SW_DWN,GWETTOP,GWETROOT,EVPTRNS"
ELEVATION_URL = "https://api.open-meteo.com/v1/elevation"


def _synthetic_climatology(lat: float) -> dict[str, dict[str, float]]:
    """Deterministic latitude-based fallback used only when NASA POWER is unreachable."""
    abs_lat = abs(lat)
    hemisphere_sign = 1.0 if lat >= 0 else -1.0
    base_temp = max(4.0, 28.0 - 0.42 * abs_lat)
    amp = min(14.0, 2.0 + 0.22 * abs_lat)

    result: dict[str, dict[str, float]] = {
        "T2M": {},
        "T2M_MIN": {},
        "T2M_MAX": {},
        "PRECTOTCORR": {},
        "RH2M": {},
        "ALLSKY_SFC_SW_DWN": {},
        "GWETTOP": {},
        "GWETROOT": {},
        "EVPTRNS": {},
    }
    for idx, key in enumerate(MONTH_KEYS):
        phase = ((idx + 1) - (7 if hemisphere_sign > 0 else 1)) * (2 * math.pi / 12.0)
        t_mean = base_temp + amp * math.cos(phase)
        precip_day = max(0.3, 3.2 + 2.4 * math.cos(phase - 0.5))
        wetness = max(0.2, min(0.85, 0.45 + 0.2 * math.cos(phase - 0.5)))
        et_day = max(0.2, min(4.5, 0.8 + 0.9 * math.cos(phase)))
        result["T2M"][key] = round(t_mean, 2)
        result["T2M_MIN"][key] = round(t_mean - 5.5, 2)
        result["T2M_MAX"][key] = round(t_mean + 6.0, 2)
        result["PRECTOTCORR"][key] = round(precip_day, 2)
        result["RH2M"][key] = round(max(35.0, min(85.0, 62.0 + 12.0 * math.cos(phase - 0.5))), 1)
        result["ALLSKY_SFC_SW_DWN"][key] = round(max(2.0, 5.2 + 1.5 * math.cos(phase)), 2)
        result["GWETTOP"][key] = round(wetness, 2)
        result["GWETROOT"][key] = round(min(0.9, wetness + 0.05), 2)
        result["EVPTRNS"][key] = round(et_day, 2)
    return result


async def fetch_nasa_power_climatology(
    lat: float,
    lon: float,
    timeout_seconds: float = 8.0,
) -> tuple[dict[str, dict[str, float]], bool, str]:
    """Fetch 30-year monthly climatology from NASA POWER Agroclimatology API."""
    params = {
        "parameters": NASA_PARAMS,
        "community": "AG",
        "longitude": round(lon, 4),
        "latitude": round(lat, 4),
        "format": "JSON",
    }
    try:
        async with httpx.AsyncClient(timeout=timeout_seconds) as client:
            resp = await client.get(NASA_POWER_URL, params=params)
            resp.raise_for_status()
            payload = resp.json()
            parameter_data = payload["properties"]["parameter"]
            return parameter_data, True, "NASA POWER Agroclimatology (Climatology API)"
    except Exception:
        return ({}, False, "NASA POWER no disponible")


async def fetch_terrain_info(
    lat: float,
    lon: float,
    timeout_seconds: float = 6.0,
) -> TerrainInfo:
    """Fetch elevation and compute local slope (%) from 4-neighbor DEM points (~111 m spacing)."""
    delta = 0.001  # ~111 m
    lats = [lat, min(89.9, lat + delta), max(-89.9, lat - delta), lat, lat]
    lons = [lon, lon, lon, min(179.9, lon + delta), max(-179.9, lon - delta)]
    try:
        async with httpx.AsyncClient(timeout=timeout_seconds) as client:
            resp = await client.get(
                ELEVATION_URL,
                params={
                    "latitude": ",".join(f"{x:.5f}" for x in lats),
                    "longitude": ",".join(f"{x:.5f}" for x in lons),
                },
            )
            resp.raise_for_status()
            elevs = resp.json()["elevation"]
            c_elev, n_elev, s_elev, e_elev, w_elev = [float(v) for v in elevs[:5]]
            dz_ns = (n_elev - s_elev) / 222.0
            cos_lat = max(0.2, math.cos(math.radians(lat)))
            dz_ew = (e_elev - w_elev) / (222.0 * cos_lat)
            slope_pct = round(math.hypot(dz_ns, dz_ew) * 100.0, 1)
            erosion = "high" if slope_pct >= 8.0 else ("moderate" if slope_pct >= 3.0 else "low")
            return TerrainInfo(
                elevation_m=round(c_elev, 1),
                slope_pct=slope_pct,
                erosion_risk=erosion,
                terrain_source="Copernicus/SRTM DEM (Open-Meteo Elevation)",
                live_terrain_data=True,
            )
    except Exception:
        return TerrainInfo(
            elevation_m=None,
            slope_pct=None,
            erosion_risk="unknown",
            terrain_source="Sin datos de elevacion (timeout)",
            live_terrain_data=False,
        )


async def fetch_parcel_environment(
    lat: float,
    lon: float,
) -> tuple[dict[str, dict[str, float]], bool, str, TerrainInfo]:
    """Fetch NASA POWER climatology and DEM terrain slope concurrently."""
    (raw, live, source), terrain = await asyncio.gather(
        fetch_nasa_power_climatology(lat, lon),
        fetch_terrain_info(lat, lon),
    )
    return raw, live, source, terrain


def build_diagnosis(
    lat: float,
    lon: float,
    raw_params: dict[str, dict[str, float]],
    live_nasa_data: bool,
    data_source: str,
    terrain: TerrainInfo,
    soil_texture: SoilTextureType = "unknown",
    rain_delta_pct: float = 0.0,
    temp_delta_c: float = 0.0,
) -> ParcelDiagnoseResponse:
    """Compute farmer- and agronomist-facing vulnerability indicators from NASA + terrain data."""
    rain_factor = max(0.2, 1.0 + rain_delta_pct / 100.0)
    et_temp_factor = max(0.85, 1.0 + 0.04 * temp_delta_c)
    monthly: list[MonthlyClimate] = []

    dry_months: list[str] = []
    frost_months: list[str] = []
    heat_months: list[str] = []
    bare_months: list[str] = []

    annual_precip_mm = 0.0
    annual_et_mm = 0.0
    temp_sum = 0.0
    wet_top_sum = 0.0
    wet_root_sum = 0.0

    for idx, key in enumerate(MONTH_KEYS):
        label = MONTH_LABELS[idx]
        t_mean = float(raw_params["T2M"].get(key, 18.0)) + temp_delta_c
        t_min = float(raw_params["T2M_MIN"].get(key, t_mean - 5.0)) + temp_delta_c
        t_max = float(raw_params["T2M_MAX"].get(key, t_mean + 5.0)) + temp_delta_c
        precip_day = max(0.0, float(raw_params["PRECTOTCORR"].get(key, 2.0)) * rain_factor)
        precip_month = precip_day * DAYS_IN_MONTH[idx]
        et_day = max(0.0, float(raw_params.get("EVPTRNS", {}).get(key, 1.2)) * et_temp_factor)
        et_month = et_day * DAYS_IN_MONTH[idx]
        water_bal = precip_month - et_month

        wet_top = max(0.0, min(1.0, float(raw_params.get("GWETTOP", {}).get(key, 0.45)) * (rain_factor ** 0.4)))
        wet_root = max(0.0, min(1.0, float(raw_params.get("GWETROOT", {}).get(key, 0.48)) * (rain_factor ** 0.3)))
        solar = float(raw_params["ALLSKY_SFC_SW_DWN"].get(key, 4.5))
        rh = float(raw_params["RH2M"].get(key, 60.0))

        frost = t_min <= 2.0
        heat = t_max >= 31.0 or t_mean >= 26.5
        dry = precip_month < 45.0 or wet_top < 0.35

        if dry:
            dry_months.append(label)
            bare_months.append(label)
        if frost:
            frost_months.append(label)
        if heat:
            heat_months.append(label)

        annual_precip_mm += precip_month
        annual_et_mm += et_month
        temp_sum += t_mean
        wet_top_sum += wet_top
        wet_root_sum += wet_root

        monthly.append(
            MonthlyClimate(
                month=idx + 1,
                month_label=label,
                temp_mean_c=round(t_mean, 1),
                temp_min_c=round(t_min, 1),
                temp_max_c=round(t_max, 1),
                precip_mm_day=round(precip_day, 2),
                precip_mm_month=round(precip_month, 1),
                evapotranspiration_mm_day=round(et_day, 2),
                evapotranspiration_mm_month=round(et_month, 1),
                water_balance_mm=round(water_bal, 1),
                soil_moisture_top=round(wet_top, 2),
                soil_moisture_root=round(wet_root, 2),
                solar_kwh_m2_day=round(solar, 2),
                rh_pct=round(rh, 1),
                frost_risk=frost,
                heat_stress=heat,
                dry_stress=dry,
            )
        )

    annual_temp = round(temp_sum / 12.0, 1)
    annual_precip_mm = round(annual_precip_mm, 0)
    annual_et_mm = round(annual_et_mm, 0)
    annual_balance = round(annual_precip_mm - annual_et_mm, 0)
    avg_top = round(wet_top_sum / 12.0, 2)
    avg_root = round(wet_root_sum / 12.0, 2)

    parts_farmer: list[str] = []
    if dry_months:
        parts_farmer.append(
            f"Hay {len(dry_months)} meses secos ({', '.join(dry_months[:5])}"
            + ("..." if len(dry_months) > 5 else "")
            + ") donde el suelo pierde humedad rapido y conviene protegerlo con cobertura."
        )
    else:
        parts_farmer.append("La lluvia se reparte bien durante el ano, aunque conviene cuidar el drenaje.")

    if terrain.live_terrain_data and terrain.erosion_risk in ("moderate", "high"):
        parts_farmer.append(
            f"Tu terreno tiene una pendiente de {terrain.slope_pct}% (riesgo {terrain.erosion_risk} de erosion): "
            "evita dejar el suelo desnudo cuando vengan lluvias fuertes."
        )
    if frost_months:
        parts_farmer.append(f"Cuidado con heladas en {', '.join(frost_months)}.")
    if heat_months:
        parts_farmer.append(f"Calor fuerte para cultivos sensibles en {', '.join(heat_months)}.")

    farmer_summary = " ".join(parts_farmer)

    terrain_summary = (f"Altitud={terrain.elevation_m:.0f} m, Pendiente={terrain.slope_pct:.1f}% ({terrain.erosion_risk})"
                       if terrain.live_terrain_data else "Altitud, pendiente y erosion desconocidas")
    agronomist_summary = (
        f"{data_source} | {terrain_summary}. "
        f"P_anual={annual_precip_mm:.0f} mm, ET_anual={annual_et_mm:.0f} mm, Balance={annual_balance:+.0f} mm, "
        f"T_media={annual_temp:.1f} C, Humedad_suelo(GWETTOP={avg_top:.2f}, GWETROOT={avg_root:.2f}). "
        f"Escenario: lluvia {rain_delta_pct:+.0f}%, temp {temp_delta_c:+.1f} C. "
        f"Meses secos: {len(dry_months)}, heladas: {len(frost_months)}, estres termico: {len(heat_months)}."
    )

    return ParcelDiagnoseResponse(
        lat=round(lat, 4),
        lon=round(lon, 4),
        data_source=data_source,
        live_nasa_data=live_nasa_data,
        terrain=terrain,
        soil_texture=soil_texture,
        annual_precip_mm=annual_precip_mm,
        annual_et_mm=annual_et_mm,
        annual_water_balance_mm=annual_balance,
        annual_temp_mean_c=annual_temp,
        avg_soil_moisture_top=avg_top,
        avg_soil_moisture_root=avg_root,
        dry_months=dry_months,
        frost_risk_months=frost_months,
        heat_stress_months=heat_months,
        bare_soil_risk_months=bare_months,
        farmer_summary=farmer_summary,
        agronomist_summary=agronomist_summary,
        monthly=monthly,
        disclaimer=(
            "Herramienta de apoyo basada en observaciones satelitales de escala regional. "
            "No reemplaza el criterio del agricultor ni la validacion de un agronomo en campo."
        ),
    )
