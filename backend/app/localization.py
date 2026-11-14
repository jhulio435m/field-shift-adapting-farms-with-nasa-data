"""Deterministic English presentation of the same evidence and strategy scores.

Spanish remains the canonical calculation path. No translator receives parcel
coordinates, personal data or a prompt, and numerical values are unchanged.
"""

from __future__ import annotations

from .correlation import HistoricalCorrelationSummary
from .forecast_alerts import WeeklyAlertSummary
from .rotation_engine import _agro_zone
from .schemas import ParcelDiagnoseResponse, RotationCompareResponse, RotationStrategy

MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
MONTH_MAP = dict(zip(["Ene", "Feb", "Mar", "Abr", "May", "Jun", "Jul", "Ago", "Sep", "Oct", "Nov", "Dic"], MONTHS))
CROPS = {
    "maize_continuous": "Continuous maize", "potato_continuous": "Continuous potato",
    "wheat_continuous": "Continuous wheat or cereal", "rice_continuous": "Continuous rice",
    "mixed_vegetables": "Mixed vegetables",
}
ZONE_NAMES = {
    "highland_cold": "cold highland zone", "warm_tropical": "warm tropical zone",
    "temperate": "temperate zone",
}
PLAN = {
    "highland_cold": {
        "drought_resilient": ("Highland water-saving rotation: quinoa + barley or tarwi",
                              ["Quinoa (lower water demand)", "Frost-tolerant barley or tarwi"], "Quinoa / barley"),
        "soil_regenerative": ("Highland soil-building rotation: broad beans or tarwi + oat-vetch",
                              ["Broad beans or tarwi (nitrogen-fixing)", "Oat + vetch cover"], "Oat / vetch"),
        "forage_income": ("Highland forage rotation: oat-vetch + dormant alfalfa",
                          ["Oat + forage vetch", "Dormant alfalfa or clover if irrigated"], "Oat / vetch"),
    },
    "warm_tropical": {
        "drought_resilient": ("Warm-zone water-saving rotation: cowpea + sorghum or millet",
                              ["Cowpea or mung bean (short cycle)", "Sorghum or pearl millet"], "Sorghum / cowpea"),
        "soil_regenerative": ("Tropical soil-building rotation: sunn hemp or cowpea + cover",
                              ["Cowpea or sunn hemp (nitrogen-fixing)", "Brachiaria or living residue cover"], "Cowpea / cover"),
        "forage_income": ("Warm-zone forage and income rotation: forage sorghum + cut grass",
                          ["Forage sorghum", "Cut grass / maralfalfa"], "Forage sorghum"),
    },
    "temperate": {
        "drought_resilient": ("Temperate water-saving rotation: chickpea or lentil + short-cycle cereal",
                              ["Drought-tolerant chickpea or lentil", "Short-cycle wheat or sorghum"], "Chickpea / wheat"),
        "soil_regenerative": ("Temperate soil-building rotation: nitrogen-fixing legume + cover",
                              ["Bean, vetch or clover (nitrogen-fixing)", "Oat + forage radish cover"], "Oat / vetch cover"),
        "forage_income": ("Temperate forage and income rotation: oat-vetch + alfalfa",
                          ["Oat + forage vetch", "Alfalfa or ryegrass if irrigated"], "Oat / alfalfa"),
    },
}


def alert_en(alert: WeeklyAlertSummary) -> None:
    if not alert.live_forecast:
        alert.source = "7-day forecast unavailable"
        alert.farmer_alert_message = "The 7-day rain, storm and frost forecast could not be retrieved."
        return
    alert.source = "Open-Meteo (7-day forecast)"
    storm = [day.date for day in alert.days if day.is_storm]
    frost = [day.date for day in alert.days if day.is_frost]
    rain = [day.date for day in alert.days if day.is_rain and not day.is_storm]
    if storm:
        message = f"STORM / HEAVY RAIN ALERT ({', '.join(storm)}): check drainage, avoid applying inputs before rain and protect sloping soil."
    elif frost:
        message = f"FROST ALERT ({', '.join(frost)}): temperatures near or below 2 °C; protect sensitive crops."
    elif rain:
        message = f"RAIN NOTICE this week ({len(rain)} likely rainy days): consider available moisture when planning planting."
    else:
        message = "No storms, heavy rain or frost are forecast in the next 7 days."
    if storm and frost:
        message += f" Frost is also possible on {', '.join(frost)}."
    alert.farmer_alert_message = message
    for day in alert.days:
        code = day.weather_code
        day.condition_label = (
            "Thunderstorm" if code in (95, 96, 99) else
            "Rain" if code in (61, 63, 65, 80, 81, 82) else
            "Drizzle" if code in (51, 53, 55) else
            "Snow / hail" if code in (71, 73, 75, 85, 86) else
            "Cloudy" if code in (1, 2, 3) else "Clear"
        )


def historical_en(history: HistoricalCorrelationSummary) -> None:
    if not history.live_historical:
        history.correlation_interpretation_farmer = "The historical NASA POWER series is unavailable right now. No trend is inferred."
        history.correlation_interpretation_agronomist = "Historical NASA POWER series unavailable; trend and correlation are not reported."
        return
    history.correlation_interpretation_farmer = (
        f"In the 2005–2024 series, temperature changed {history.temp_trend_c_per_decade:+.2f} °C per decade "
        f"and recent rainfall changed {history.precip_change_pct_recent_5y:+.1f}% "
        f"({history.drought_years_last_20} relatively dry years). Rainfall and estimated surface soil moisture "
        f"had a monthly correlation (Pearson r={history.rain_soil_moisture_corr:.2f}); this regional association does not measure "
        "your parcel's water retention or prove causation."
    )
    ndvi = (f" Point NDVI={history.latest_ndvi} on {history.latest_ndvi_date}."
            if history.latest_ndvi is not None else " Point NDVI unavailable.")
    history.correlation_interpretation_agronomist = (
        f"NASA POWER 2005–2024 ({history.years_analyzed} annual means): temperature trend "
        f"{history.temp_trend_c_per_decade:+.2f} °C/decade; recent rainfall change "
        f"{history.precip_change_pct_recent_5y:+.1f}%; relatively dry years "
        f"{history.drought_years_last_20}; Pearson r(PRECTOTCORR, modelled GWETTOP)="
        f"{history.rain_soil_moisture_corr:.2f}. Sentinel-2 L2A sample: "
        f"{history.sentinel2_clear_scenes_12m} low-cloud scenes.{ndvi} "
        "Correlation is descriptive, not causal; field soil measurements are still needed."
    )


def diagnosis_en(d: ParcelDiagnoseResponse, rain_delta: float = 0, temp_delta: float = 0) -> None:
    for month in d.monthly:
        month.month_label = MONTH_MAP.get(month.month_label, month.month_label)
    for attr in ("dry_months", "frost_risk_months", "heat_stress_months", "bare_soil_risk_months"):
        setattr(d, attr, [MONTH_MAP.get(item, item) for item in getattr(d, attr)])
    parts = []
    if d.dry_months:
        parts.append(f"{len(d.dry_months)} typically dry months ({', '.join(d.dry_months[:5])}) may leave the soil exposed; consider cover crops.")
    else:
        parts.append("Rainfall is more evenly spread through the year; drainage still needs local review.")
    if d.terrain.live_terrain_data and d.terrain.erosion_risk in ("moderate", "high"):
        parts.append(f"The estimated slope is {d.terrain.slope_pct}% ({d.terrain.erosion_risk} erosion risk); avoid bare soil before heavy rain.")
    if d.frost_risk_months:
        parts.append(f"Frost-sensitive crops need attention in {', '.join(d.frost_risk_months)}.")
    if d.heat_stress_months:
        parts.append(f"Heat-sensitive crops need attention in {', '.join(d.heat_stress_months)}.")
    d.farmer_summary = " ".join(parts)
    terrain = (f"elevation {d.terrain.elevation_m:.0f} m, estimated slope {d.terrain.slope_pct:.1f}% ({d.terrain.erosion_risk} erosion risk)"
               if d.terrain.live_terrain_data else "elevation and slope unavailable")
    d.agronomist_summary = (
        f"{d.data_source}; {terrain}. Annual rain {d.annual_precip_mm:.0f} mm, ET "
        f"{d.annual_et_mm:.0f} mm, water balance {d.annual_water_balance_mm:+.0f} mm; "
        f"mean temperature {d.annual_temp_mean_c:.1f} °C; modelled GWETTOP "
        f"{d.avg_soil_moisture_top:.2f}, GWETROOT {d.avg_soil_moisture_root:.2f}. "
        f"Scenario: rain {rain_delta:+.0f}%, temperature {temp_delta:+.1f} °C. "
        f"Dry months {len(d.dry_months)}, frost-risk months {len(d.frost_risk_months)}, "
        f"heat-stress months {len(d.heat_stress_months)}."
    )
    d.disclaimer = ("Decision support from regional satellite and model data. "
                    "Farmer judgment and local agronomic review remain essential.")
    if d.weekly_alerts:
        alert_en(d.weekly_alerts)
    if d.historical_correlation:
        historical_en(d.historical_correlation)
    if not d.terrain.live_terrain_data:
        d.terrain.terrain_source = "Elevation data unavailable"


def strategy_en(strategy: RotationStrategy, d: ParcelDiagnoseResponse, crop_key: str,
                soil_texture: str) -> None:
    zone = _agro_zone(d)
    crop = CROPS[crop_key]
    dry = len(d.dry_months)
    frost = len(d.frost_risk_months)
    heat = len(d.heat_stress_months)
    elev = f"{d.terrain.elevation_m:.0f} m" if d.terrain.elevation_m is not None else "elevation unavailable"
    slope = f"{d.terrain.slope_pct:.1f}%" if d.terrain.slope_pct is not None else "slope unavailable"
    zone_text = f"{ZONE_NAMES[zone]} ({elev}, {d.annual_temp_mean_c:.1f} °C)"
    soil_gap = ("Soil texture is unknown; field texture, pH and organic matter need testing."
                if soil_texture == "unknown" else
                f"Farmer-reported soil texture: {soil_texture}; field pH and organic matter still need testing.")
    strategy.information_gaps = [soil_gap]
    if strategy.id == "current_baseline":
        strategy.name = f"Current rotation: {crop}"
        strategy.sequence = [crop, crop, "Fallow / bare soil"]
        strategy.why_suggested_farmer = (f"This is familiar and may fit existing equipment, but repeating the same crop "
                                         f"can leave soil exposed in {dry} dry months and allow pests to build up.")
        strategy.why_suggested_agronomist = (f"Baseline {crop} in the {zone_text}: {dry} dry months, annual water balance "
                                            f"{d.annual_water_balance_mm:+.0f} mm, estimated slope {slope}.")
        strategy.evidence = [f"NASA POWER: {dry} dry months; modelled mean GWETTOP {d.avg_soil_moisture_top:.2f}.",
                             f"Terrain: {elev}, estimated slope {slope} ({d.terrain.erosion_risk} erosion risk)."]
        strategy.benefits = ["Existing equipment and knowledge may be usable.", "Existing buyers may already be known."]
        strategy.information_gaps.insert(0, "Current pest and nematode pressure has not been measured in the field.")
        strategy.local_review_needed = ["Check yield changes over the last three seasons.",
                                        "Compare local fertilizer costs with a legume rotation."]
        cover = "Bare soil / residue"
    else:
        name, sequence, cover = PLAN[zone][strategy.id]
        strategy.name = name
        strategy.sequence = [crop, *sequence]
        if strategy.id == "drought_resilient":
            strategy.why_suggested_farmer = (f"Crops suited to the {zone_text} may use less water "
                                             "and make better use of remaining soil moisture.")
            strategy.why_suggested_agronomist = (f"Adapted to the {zone_text}; annual ET {d.annual_et_mm:.0f} mm "
                                                f"and modelled root-zone GWETROOT {d.avg_soil_moisture_root:.2f}.")
            strategy.evidence = [f"NASA POWER and terrain: {dry} dry months, {heat} heat-stress months, {elev}.",
                                 f"Modelled mean root-zone moisture GWETROOT {d.avg_soil_moisture_root:.2f}."]
            strategy.benefits = ["May reduce irrigation demand or dry-year exposure.",
                                 "May interrupt crop-specific pest cycles.", "May diversify income."]
            strategy.information_gaps.append("Local seed availability and buyers for the alternative crop are unconfirmed.")
            strategy.local_review_needed = ["Confirm a suitable local variety and planting date with an agronomist.",
                                            "Check whether existing equipment can handle the alternative crop."]
        elif strategy.id == "soil_regenerative":
            strategy.why_suggested_farmer = ("Legumes and cover crops can protect exposed soil, retain moisture "
                                             "and contribute nitrogen, subject to local conditions.")
            strategy.why_suggested_agronomist = (f"In the {zone_text}, continuous cover may reduce erosion risk "
                                                f"({d.terrain.erosion_risk}; estimated slope {slope}) and support infiltration.")
            strategy.evidence = [f"Terrain: {elev}, estimated slope {slope}.",
                                 f"NASA POWER: {len(d.bare_soil_risk_months)} months with potential bare-soil exposure."]
            strategy.benefits = ["Legumes may add nitrogen and reduce fertilizer demand.",
                                 "Cover may improve rainfall infiltration and protect soil.",
                                 "Rotation may interrupt weed and soil pest cycles."]
            strategy.information_gaps.append("Local cover-crop and legume seed availability is unconfirmed.")
            strategy.local_review_needed = ["Plan when to cut or incorporate the cover before the main crop.",
                                            "Check starting soil moisture if irrigation is unavailable."]
        else:
            strategy.why_suggested_farmer = ("A forage crop may feed livestock or provide extra income "
                                             "while keeping soil covered, if local demand and water allow.")
            strategy.why_suggested_agronomist = (f"Adapted to the {zone_text} ({frost} frost-risk months); "
                                                "forage and legumes may diversify income and cover soil.")
            strategy.evidence = [f"NASA POWER: {frost} frost-risk months; annual water balance "
                                 f"{d.annual_water_balance_mm:+.0f} mm.",
                                 "Forage cover may reduce surface soil loss between growing seasons."]
            strategy.benefits = ["Potential livestock feed or forage sales.",
                                 "Potential fit with local climate, subject to field review.",
                                 "May reduce reliance on the main harvest for income."]
            strategy.information_gaps.append("Local forage demand and storage capacity are unknown.")
            strategy.local_review_needed = ["Check fencing and grazing pressure to avoid soil compaction.",
                                            "Confirm local forage prices and buyers."]
    for month in strategy.monthly_calendar:
        is_dry = month.month_label in d.dry_months
        month.month_label = MONTH_MAP.get(month.month_label, month.month_label)
        month.crop_or_cover = ("Fallow / frost risk" if month.status == "frost_risk" else
                               cover if month.status in ("bare_risk", "water_stress") or
                               (is_dry and strategy.id != "current_baseline") else crop)


def comparison_en(result: RotationCompareResponse, crop_key: str, soil_texture: str,
                  rain_delta: float, temp_delta: float) -> None:
    for strategy in result.strategies:
        strategy_en(strategy, result.diagnosis, crop_key, soil_texture)
    diagnosis_en(result.diagnosis, rain_delta, temp_delta)
    result.human_judgment_reminder = (
        "The index and calendar are rule-based guides. Planting dates, soil measurements and local validation "
        "are still needed. Farmer judgment and local agronomic review remain essential."
    )
