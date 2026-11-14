"""Crop rotation strategy comparison engine grounded in NASA climate, soil moisture, terrain & agro-ecological zone."""

from __future__ import annotations

from .schemas import (
    CropKey,
    IrrigationType,
    MonthCoverage,
    ParcelDiagnoseResponse,
    PriorityType,
    RotationStrategy,
    SoilTextureType,
)

CURRENT_CROP_NAMES: dict[CropKey, str] = {
    "maize_continuous": "Maiz continuo",
    "potato_continuous": "Papa continua",
    "wheat_continuous": "Trigo / cereal continuo",
    "rice_continuous": "Arroz continuo",
    "mixed_vegetables": "Hortalizas mixtas",
}


def _agro_zone(diagnosis: ParcelDiagnoseResponse) -> str:
    """Classify parcel into cold/highland, temperate, or warm/tropical using elevation & mean temp."""
    elev = diagnosis.terrain.elevation_m
    temp = diagnosis.annual_temp_mean_c
    if (elev is not None and elev >= 2400.0) or temp < 13.0:
        return "highland_cold"
    if temp >= 20.0 and elev is not None and elev < 1800.0:
        return "warm_tropical"
    return "temperate"


def _build_calendar(
    diagnosis: ParcelDiagnoseResponse,
    cover_in_dry: bool,
    primary_label: str,
    secondary_label: str,
) -> list[MonthCoverage]:
    calendar: list[MonthCoverage] = []
    for m in diagnosis.monthly:
        if m.frost_risk and not cover_in_dry:
            status = "frost_risk"
            crop = "Descanso / riesgo helada"
        elif m.dry_stress:
            if cover_in_dry:
                status = "covered"
                crop = secondary_label
            else:
                status = "bare_risk"
                crop = "Suelo desnudo / rastrojo"
        else:
            status = "covered"
            crop = primary_label
        calendar.append(
            MonthCoverage(
                month=m.month,
                month_label=m.month_label,
                status=status,
                crop_or_cover=crop,
            )
        )
    return calendar


def compare_rotations(
    diagnosis: ParcelDiagnoseResponse,
    current_crop: CropKey,
    irrigation: IrrigationType,
    priority: PriorityType,
    soil_texture: SoilTextureType,
    has_livestock: bool,
) -> list[RotationStrategy]:
    """Generate and rank 4 rotation strategies adapted to the parcel's altitude, climate & soil."""
    current_name = CURRENT_CROP_NAMES[current_crop]
    dry_count = len(diagnosis.dry_months)
    frost_count = len(diagnosis.frost_risk_months)
    heat_count = len(diagnosis.heat_stress_months)
    slope = diagnosis.terrain.slope_pct
    erosion = diagnosis.terrain.erosion_risk
    zone = _agro_zone(diagnosis)
    elevation_label = f"{diagnosis.terrain.elevation_m:.0f} m" if diagnosis.terrain.elevation_m is not None else "altitud sin dato"
    slope_label = f"{slope:.1f}%" if slope is not None else "sin dato"

    # Zone-specific crops that actually grow at the parcel's altitude and temperature
    if zone == "highland_cold":
        zone_label = f"zona altoandina/fria ({elevation_label}, {diagnosis.annual_temp_mean_c:.1f} C)"
        water_name = "Estrategia Agua Segura de Altura: Quinua + Cebada / Tarwi"
        water_seq = [current_name, "Quinua (bajo consumo de agua)", "Cebada o Tarwi resistente a heladas"]
        water_cover = "Quinua / Cebada"

        soil_name = "Estrategia Suelo Vivo de Altura: Habas / Tarwi + Avena-Vicia"
        soil_seq = [current_name, "Habas o Tarwi (fijador de nitrogeno)", "Avena + Vicia (cobertura protectora)"]
        soil_cover = "Avena / Vicia"

        forage_name = "Estrategia Mixta Forraje de Altura: Avena-Vicia + Alfalfa dormante"
        forage_seq = [current_name, "Avena + Vicia forrajera", "Alfalfa dormante o Trebol (si hay riego)"]
        forage_cover = "Avena / Vicia"
    elif zone == "warm_tropical":
        zone_label = f"zona calida/tropical ({elevation_label}, {diagnosis.annual_temp_mean_c:.1f} C)"
        water_name = "Estrategia Agua Segura: Caupi + Sorgo / Mijo tolerante a calor"
        water_seq = [current_name, "Frijol caupi / mung (ciclo corto)", "Sorgo o mijo perla"]
        water_cover = "Sorgo / Caupi"

        soil_name = "Estrategia Suelo Vivo Tropical: Crotalaria / Caupi + Cobertura"
        soil_seq = [current_name, "Caupi o Crotalaria (fijador de N)", "Cobertura de braquiaria / rastrojo vivo"]
        soil_cover = "Caupi / Cobertura"

        forage_name = "Estrategia Mixta Forraje + Ingreso: Sorgo forrajero + Pasto de corte"
        forage_seq = [current_name, "Sorgo forrajero", "Pasto de corte / Maralfalfa"]
        forage_cover = "Sorgo forrajero"
    else:
        zone_label = f"zona templada ({elevation_label}, {diagnosis.annual_temp_mean_c:.1f} C)"
        water_name = "Estrategia Agua Segura: Garbanzo / Lenteja + Sorgo o Trigo corto"
        water_seq = [current_name, "Garbanzo / Lenteja tolerante a sequia", "Trigo o sorgo de ciclo corto"]
        water_cover = "Garbanzo / Trigo"

        soil_name = "Estrategia Suelo Vivo: Fijador de Nitrogeno + Cobertura Protectora"
        soil_seq = [current_name, "Frijol / Vicia / Trebol (fijador de N)", "Cobertura de avena + nabo forrajero"]
        soil_cover = "Cobertura Avena/Vicia"

        forage_name = "Estrategia Mixta Forraje + Ingreso: Avena-Vicia + Alfalfa"
        forage_seq = [current_name, "Avena + Vicia forrajera", "Alfalfa / Raigras (si hay riego)"]
        forage_cover = "Avena / Alfalfa"

    soil_gap = (
        "No se indico textura del suelo (arenoso/franco/arcilloso); se asume capacidad media de retencion."
        if soil_texture == "unknown"
        else f"Textura reportada por agricultor: {soil_texture}; falta analisis quimico de pH y materia organica."
    )

    # 1. Current baseline
    baseline_score = 52 - min(22, dry_count * 3) - min(12, heat_count * 2)
    if erosion == "high":
        baseline_score -= 10
    elif erosion == "moderate":
        baseline_score -= 5
    if soil_texture == "sandy":
        baseline_score -= 4
    if irrigation == "irrigated":
        baseline_score += 10
    baseline_score = max(15, min(75, baseline_score))

    current_strategy = RotationStrategy(
        id="current_baseline",
        name=f"Rotacion actual: {current_name}",
        is_current=True,
        sequence=[current_name, current_name, "Descanso / suelo desnudo"],
        fit_score=baseline_score,
        water_demand_level="high" if current_crop in ("maize_continuous", "rice_continuous", "potato_continuous") else "medium",
        why_suggested_farmer=(
            "Es lo que ya conoces y tienes equipo para trabajar, pero repetir lo mismo deja el suelo "
            f"expuesto {dry_count} meses secos y facilita que se acumulen plagas."
        ),
        why_suggested_agronomist=(
            f"Linea base ({current_name}) en {zone_label}. Vulnerabilidad ante {dry_count} meses secos, "
            f"balance hidrico anual de {diagnosis.annual_water_balance_mm:+.0f} mm y pendiente {slope_label} ({erosion})."
        ),
        evidence=[
            f"NASA POWER: {dry_count} meses secos; humedad superficial media GWETTOP={diagnosis.avg_soil_moisture_top:.2f}.",
            f"Terreno: altitud {elevation_label}, pendiente {slope_label} (riesgo erosion: {erosion}).",
        ],
        benefits=[
            "Cero curva de aprendizaje operativa.",
            "Maquinaria y compradores ya conocidos.",
        ],
        information_gaps=[
            "No conocemos la carga actual de plagas/nematodos en tu lote.",
            soil_gap,
        ],
        local_review_needed=[
            "Evaluar si el rendimiento de los ultimos 3 anos viene bajando.",
            "Revisar costo real de fertilizantes frente a rotar con leguminosas.",
        ],
        monthly_calendar=_build_calendar(diagnosis, cover_in_dry=False, primary_label=current_name, secondary_label="Suelo desnudo"),
    )

    # 2. Water-saving / Drought-resilient strategy
    water_score = 72 + (14 if priority == "water" else 4) + (8 if irrigation == "rainfed" else 2)
    if dry_count >= 4 or diagnosis.avg_soil_moisture_top < 0.42:
        water_score += 6
    if soil_texture == "sandy":
        water_score += 4
    water_score = min(97, water_score)

    water_strategy = RotationStrategy(
        id="drought_resilient",
        name=water_name,
        is_current=False,
        sequence=water_seq,
        fit_score=water_score,
        water_demand_level="low",
        why_suggested_farmer=(
            f"Propone cultivos adaptados a tu altura y clima ({zone_label}) que aguantan mejor "
            "la falta de lluvia y aprovechan la humedad que queda en el suelo."
        ),
        why_suggested_agronomist=(
            f"Seleccion adaptada a {zone_label}. Reduce demanda evapotranspirativa (ET anual {diagnosis.annual_et_mm:.0f} mm) "
            f"y aprovecha humedad radicular residual (GWETROOT={diagnosis.avg_soil_moisture_root:.2f})."
        ),
        evidence=[
            f"NASA POWER + DEM: {zone_label}, {dry_count} meses secos y {heat_count} meses de calor fuerte.",
            f"Humedad radicular media GWETROOT={diagnosis.avg_soil_moisture_root:.2f} permite ciclo corto post-lluvia.",
        ],
        benefits=[
            "Menor necesidad de riego o menor riesgo en ano seco.",
            "Corta el ciclo de plagas especificas del cultivo principal.",
            "Diversifica ingresos frente a caidas de precio.",
        ],
        information_gaps=[
            soil_gap,
            "Falta confirmar semilla certificada y comprador local para el cultivo alternativo.",
        ],
        local_review_needed=[
            "Consultar con tu agronomo la variedad local exacta para tu altitud y fecha de siembra.",
            "Verificar si tu cosechadora o equipo actual sirve para el cultivo de rotacion.",
        ],
        monthly_calendar=_build_calendar(diagnosis, cover_in_dry=True, primary_label=current_name, secondary_label=water_cover),
    )

    # 3. Soil-regenerating / Nitrogen-fixing + Cover crop strategy
    soil_score = 74 + (15 if priority == "soil_health" else 5) + (10 if priority == "pest_disruption" else 0)
    if erosion in ("moderate", "high"):
        soil_score += 6
    if zone == "highland_cold":
        soil_score += 2
    soil_score = min(98, soil_score)

    soil_strategy = RotationStrategy(
        id="soil_regenerative",
        name=soil_name,
        is_current=False,
        sequence=soil_seq,
        fit_score=soil_score,
        water_demand_level="medium",
        why_suggested_farmer=(
            f"En tu {zone_label}, en vez de dejar la tierra pelada cuando no llueve, pones leguminosas "
            "y cobertura que tapan el suelo, guardan humedad, frenan la erosion y dejan abono natural."
        ),
        why_suggested_agronomist=(
            f"Adaptada a {zone_label}. Mitiga riesgo de erosion ({erosion}, pendiente {slope_label}) mediante "
            "cobertura continua, fijacion biologica de N y mejora de infiltracion."
        ),
        evidence=[
            f"Terreno: altitud {elevation_label}, pendiente {slope_label} ({erosion}).",
            f"NASA POWER: {len(diagnosis.bare_soil_risk_months)} meses con riesgo de suelo desnudo.",
        ],
        benefits=[
            "Aporta nitrogeno natural y reduce gasto en fertilizante quimico.",
            "Mejora infiltracion de lluvia y retencion de humedad.",
            "Rompe ciclos de malezas y plagas del suelo.",
        ],
        information_gaps=[
            soil_gap,
            "Falta validar disponibilidad local de semilla de cobertura o leguminosa.",
        ],
        local_review_needed=[
            "Definir momento de corte o incorporacion de la cobertura antes de la siembra principal.",
            "Evaluar humedad disponible al inicio si no hay riego.",
        ],
        monthly_calendar=_build_calendar(diagnosis, cover_in_dry=True, primary_label=current_name, secondary_label=soil_cover),
    )

    # 4. Forage & Supplemental Income strategy
    forage_score = 68 + (16 if priority == "forage_income" else 3) + (10 if has_livestock else 0)
    if frost_count >= 2:
        forage_score += 4
    forage_score = min(96, forage_score)

    forage_strategy = RotationStrategy(
        id="forage_income",
        name=forage_name,
        is_current=False,
        sequence=forage_seq,
        fit_score=forage_score,
        water_demand_level="medium",
        why_suggested_farmer=(
            "Aprovecha la temporada fresca para sacar pasto o heno adaptado a tu zona para tus animales "
            "(o vender forraje a ganaderos vecinos), protegiendo la tierra y generando ingreso extra."
        ),
        why_suggested_agronomist=(
            f"Adaptada a {zone_label} ({frost_count} meses con Tmin<=2 C). Combina forraje rustico y "
            "leguminosa para diversificar ingresos y cubrir el suelo."
        ),
        evidence=[
            f"NASA POWER: {frost_count} meses con Tmin <= 2 C y balance hidrico {diagnosis.annual_water_balance_mm:+.0f} mm.",
            "La cobertura forrajera reduce perdida superficial de suelo entre campanas.",
        ],
        benefits=[
            "Alimento propio para ganado o venta de forraje/heno.",
            "Buena tolerancia al clima local.",
            "Reduce presion economica sobre la cosecha principal.",
        ],
        information_gaps=[
            soil_gap,
            "Falta conocer demanda local de heno/forraje y capacidad de almacenamiento.",
        ],
        local_review_needed=[
            "Verificar cercos o control de pastoreo para evitar compactacion por pisoteo en suelo humedo.",
            "Consultar precios de forraje en feria o con vecinos ganaderos.",
        ],
        monthly_calendar=_build_calendar(diagnosis, cover_in_dry=True, primary_label=current_name, secondary_label=forage_cover),
    )

    strategies = [water_strategy, soil_strategy, forage_strategy, current_strategy]
    strategies.sort(key=lambda s: (not s.is_current, s.fit_score), reverse=True)
    return strategies
