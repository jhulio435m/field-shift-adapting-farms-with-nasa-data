export type Language = 'es' | 'en';
export type ViewMode = 'farmer' | 'agronomist';
export type BotChannelType = 'whatsapp' | 'telegram' | 'both';
export type AlertFrequencyType = 'immediate' | 'weekly' | 'on_demand';

export interface BotConfig {
  preferred_channel: BotChannelType;
  telegram_handle: string;
  bot_language: Language;
  bot_audience: ViewMode;
  alert_frequency: AlertFrequencyType;
  alerts_enabled: boolean;
}

export type CropKey =
  | 'maize_continuous'
  | 'potato_continuous'
  | 'wheat_continuous'
  | 'rice_continuous'
  | 'mixed_vegetables';

export type IrrigationType = 'rainfed' | 'limited' | 'irrigated';
export type SoilTextureType = 'unknown' | 'sandy' | 'loam' | 'clay';
export type PriorityType = 'water' | 'soil_health' | 'pest_disruption' | 'forage_income';

export interface Parcel {
  id: string;
  name: string;
  lat: number;
  lon: number;
  current_crop: CropKey;
  irrigation: IrrigationType;
  soil_texture: SoilTextureType;
  priority: PriorityType;
  has_livestock: boolean;
  area_ha?: number | null;
  planting_month?: number | null;
  polygon?: [number, number][] | null;
}

export interface FarmerProfile {
  name: string;
  phone_last4: string;
  csrf: string;
  bot_config: BotConfig;
  parcels: Parcel[];
}

export interface MonthlyClimate {
  month: number;
  month_label: string;
  temp_mean_c: number;
  temp_min_c: number;
  temp_max_c: number;
  precip_mm_day: number;
  precip_mm_month: number;
  evapotranspiration_mm_day: number;
  evapotranspiration_mm_month: number;
  water_balance_mm: number;
  soil_moisture_top: number;
  soil_moisture_root: number;
  solar_kwh_m2_day: number;
  rh_pct: number;
  frost_risk: boolean;
  heat_stress: boolean;
  dry_stress: boolean;
}

export interface TerrainInfo {
  elevation_m: number | null;
  slope_pct: number | null;
  erosion_risk: 'low' | 'moderate' | 'high' | 'unknown';
  terrain_source: string;
  live_terrain_data: boolean;
}

export interface ForecastDay {
  date: string;
  precip_mm: number;
  precip_prob_pct: number;
  temp_min_c: number;
  temp_max_c: number;
  is_rain: boolean;
  is_storm: boolean;
  is_frost: boolean;
}

export interface WeeklyAlertSummary {
  source: string;
  alert_level: 'normal' | 'rain' | 'frost' | 'storm';
  farmer_alert_message: string;
  days: ForecastDay[];
}

export interface HistoricalCorrelationSummary {
  live_historical: boolean;
  years_analyzed: number;
  temp_trend_c_per_decade: number;
  precip_change_pct_recent_5y: number;
  rain_soil_moisture_corr: number;
  drought_years_last_20: number;
  sentinel2_clear_scenes_12m: number;
  latest_ndvi: number | null;
  latest_ndvi_date: string | null;
  vegetation_status: string;
  correlation_interpretation_farmer: string;
  correlation_interpretation_agronomist: string;
}

export interface ParcelDiagnosis {
  lat: number;
  lon: number;
  data_source: string;
  live_nasa_data: boolean;
  terrain: TerrainInfo;
  soil_texture: SoilTextureType;
  annual_precip_mm: number;
  annual_et_mm: number;
  annual_water_balance_mm: number;
  annual_temp_mean_c: number;
  avg_soil_moisture_top: number;
  avg_soil_moisture_root: number;
  dry_months: string[];
  frost_risk_months: string[];
  heat_stress_months: string[];
  bare_soil_risk_months: string[];
  farmer_summary: string;
  agronomist_summary: string;
  weekly_alerts: WeeklyAlertSummary | null;
  historical_correlation: HistoricalCorrelationSummary | null;
  monthly: MonthlyClimate[];
  disclaimer: string;
}

export interface MonthCoverage {
  month: number;
  month_label: string;
  status: 'covered' | 'bare_risk' | 'water_stress' | 'frost_risk';
  crop_or_cover: string;
}

export interface RotationStrategy {
  id: string;
  name: string;
  is_current: boolean;
  sequence: string[];
  fit_score: number;
  water_demand_level: 'low' | 'medium' | 'high';
  why_suggested_farmer: string;
  why_suggested_agronomist: string;
  evidence: string[];
  benefits: string[];
  information_gaps: string[];
  local_review_needed: string[];
  monthly_calendar: MonthCoverage[];
}

export interface RotationCompareResponse {
  diagnosis: ParcelDiagnosis;
  strategies: RotationStrategy[];
  human_judgment_reminder: string;
}
