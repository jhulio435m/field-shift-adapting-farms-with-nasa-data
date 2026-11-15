"""Pydantic models for the Field Shift Geo-Agro Decision-Support API."""

from typing import Literal
from pydantic import BaseModel, Field

from .correlation import HistoricalCorrelationSummary
from .forecast_alerts import WeeklyAlertSummary


IrrigationType = Literal["rainfed", "limited", "irrigated"]
PriorityType = Literal["water", "soil_health", "pest_disruption", "forage_income"]
SoilTextureType = Literal["unknown", "sandy", "loam", "clay"]
CropKey = Literal[
    "maize_continuous",
    "potato_continuous",
    "wheat_continuous",
    "rice_continuous",
    "mixed_vegetables",
]
Language = Literal["es", "en"]
BotChannelType = Literal["whatsapp", "telegram", "both"]
AlertFrequencyType = Literal["immediate", "weekly", "on_demand"]
AudienceType = Literal["farmer", "agronomist"]


class ParcelDiagnoseRequest(BaseModel):
    lat: float = Field(..., ge=-90.0, le=90.0, description="Parcel latitude")
    lon: float = Field(..., ge=-180.0, le=180.0, description="Parcel longitude")
    current_crop: CropKey = Field(default="maize_continuous")
    soil_texture: SoilTextureType = Field(default="unknown")
    rain_delta_pct: float = Field(default=0.0, ge=-50.0, le=50.0)
    temp_delta_c: float = Field(default=0.0, ge=-2.0, le=5.0)
    language: Language = "es"


class MonthlyClimate(BaseModel):
    month: int = Field(..., ge=1, le=12)
    month_label: str
    temp_mean_c: float
    temp_min_c: float
    temp_max_c: float
    precip_mm_day: float
    precip_mm_month: float
    evapotranspiration_mm_day: float
    evapotranspiration_mm_month: float
    water_balance_mm: float
    soil_moisture_top: float
    soil_moisture_root: float
    solar_kwh_m2_day: float
    rh_pct: float
    frost_risk: bool
    heat_stress: bool
    dry_stress: bool


class TerrainInfo(BaseModel):
    elevation_m: float | None
    slope_pct: float | None
    erosion_risk: Literal["low", "moderate", "high", "unknown"]
    terrain_source: str
    live_terrain_data: bool


class ParcelDiagnoseResponse(BaseModel):
    lat: float
    lon: float
    data_source: str
    live_nasa_data: bool
    terrain: TerrainInfo
    soil_texture: SoilTextureType
    annual_precip_mm: float
    annual_et_mm: float
    annual_water_balance_mm: float
    annual_temp_mean_c: float
    avg_soil_moisture_top: float
    avg_soil_moisture_root: float
    dry_months: list[str]
    frost_risk_months: list[str]
    heat_stress_months: list[str]
    bare_soil_risk_months: list[str]
    farmer_summary: str
    agronomist_summary: str
    weekly_alerts: WeeklyAlertSummary | None = None
    historical_correlation: HistoricalCorrelationSummary | None = None
    monthly: list[MonthlyClimate]
    disclaimer: str


class RotationCompareRequest(BaseModel):
    lat: float = Field(..., ge=-90.0, le=90.0)
    lon: float = Field(..., ge=-180.0, le=180.0)
    current_crop: CropKey = Field(default="maize_continuous")
    irrigation: IrrigationType = Field(default="rainfed")
    priority: PriorityType = Field(default="water")
    soil_texture: SoilTextureType = Field(default="unknown")
    has_livestock: bool = Field(default=False)
    rain_delta_pct: float = Field(default=0.0, ge=-50.0, le=50.0)
    temp_delta_c: float = Field(default=0.0, ge=-2.0, le=5.0)
    language: Language = "es"


class MonthCoverage(BaseModel):
    month: int
    month_label: str
    status: Literal["covered", "bare_risk", "water_stress", "frost_risk"]
    crop_or_cover: str


class RotationStrategy(BaseModel):
    id: str
    name: str
    is_current: bool
    sequence: list[str]
    fit_score: int = Field(..., ge=0, le=100)
    water_demand_level: Literal["low", "medium", "high"]
    why_suggested_farmer: str
    why_suggested_agronomist: str
    evidence: list[str]
    benefits: list[str]
    information_gaps: list[str]
    local_review_needed: list[str]
    monthly_calendar: list[MonthCoverage]


class RotationCompareResponse(BaseModel):
    diagnosis: ParcelDiagnoseResponse
    strategies: list[RotationStrategy] = Field(..., min_length=3)
    human_judgment_reminder: str


class KusiCapabilityRequest(BaseModel):
    """Compatible input payload for Kusi Kusi HTTP capability execution."""

    lat: float = Field(..., ge=-90.0, le=90.0)
    lon: float = Field(..., ge=-180.0, le=180.0)
    current_crop: CropKey = Field(default="maize_continuous")
    irrigation: IrrigationType = Field(default="rainfed")
    priority: PriorityType = Field(default="water")
    soil_texture: SoilTextureType = Field(default="unknown")
    has_livestock: bool = Field(default=False)
    audience: Literal["farmer", "agronomist"] = Field(default="farmer")
    language: Language = "es"


class KusiCapabilityResponse(BaseModel):
    """Structured evidence for the Kusi Kusi assistant; no canned answer."""

    strategies_count: int
    live_nasa_data: bool
    live_terrain_data: bool
    raw_comparison: RotationCompareResponse


class AlertCheckRequest(BaseModel):
    lat: float = Field(..., ge=-90.0, le=90.0)
    lon: float = Field(..., ge=-180.0, le=180.0)
    language: Language = "es"


class TelegramWebhookResponse(BaseModel):
    ok: bool
    chat_id: int | None = None
    sent_to_telegram: bool
    reply_text: str


class FarmerStartRequest(BaseModel):
    name: str = Field(..., min_length=2, max_length=100)
    phone: str = Field(default="", max_length=16)
    preferred_channel: BotChannelType = Field(default="whatsapp")
    telegram_handle: str = Field(default="", max_length=64)
    bot_language: Language = Field(default="es")
    bot_audience: AudienceType = Field(default="farmer")
    alert_frequency: AlertFrequencyType = Field(default="immediate")
    alerts_enabled: bool = Field(default=True)


class BotConfigRequest(BaseModel):
    preferred_channel: BotChannelType = Field(default="whatsapp")
    telegram_handle: str = Field(default="", max_length=64)
    bot_language: Language = Field(default="es")
    bot_audience: AudienceType = Field(default="farmer")
    alert_frequency: AlertFrequencyType = Field(default="immediate")
    alerts_enabled: bool = Field(default=True)


class KusiVerifyProof(BaseModel):
    challenge: str
    phone: str
    verified_at: int
    event_id: str
    signature: str


class ClaimRequest(BaseModel):
    code: str


class ParcelCreateRequest(RotationCompareRequest):
    name: str = Field(..., min_length=1, max_length=80)
    area_ha: float | None = Field(default=None, ge=0.01, le=100000.0)
    planting_month: int | None = Field(default=None, ge=1, le=12)
    polygon: list[list[float]] | None = Field(
        default=None,
        description="Optional polygon vertices [[lat, lon], ...]",
    )


class ScenarioRequest(BaseModel):
    rain_delta_pct: float = Field(default=0.0, ge=-50.0, le=50.0)
    temp_delta_c: float = Field(default=0.0, ge=-2.0, le=5.0)
    language: Language = "es"


class DemoStartRequest(BaseModel):
    name: str = Field(default="María Quispe", min_length=2, max_length=100)
    phone: str = Field(default="+51987654321", max_length=16)
    preferred_channel: BotChannelType = Field(default="whatsapp")
    telegram_handle: str = Field(default="@maria_mantaro", max_length=64)
    bot_language: Language = Field(default="es")
    bot_audience: AudienceType = Field(default="farmer")
    alert_frequency: AlertFrequencyType = Field(default="immediate")
    alerts_enabled: bool = Field(default=True)
    seed_parcels: bool = Field(default=True)
