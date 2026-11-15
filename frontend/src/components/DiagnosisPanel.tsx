import { useState } from 'react';
import {
  AlertTriangle,
  ArrowRight,
  Award,
  BarChart3,
  CalendarDays,
  ChevronDown,
  ChevronUp,
  CloudLightning,
  CloudRain,
  Compass,
  Database,
  Droplets,
  Flame,
  Info,
  Mountain,
  Snowflake,
  Sprout,
  Sun,
  Table2,
  Thermometer,
} from 'lucide-react';
import { tr } from '../i18n';
import type {
  Language,
  MonthlyClimate,
  Parcel,
  ParcelDiagnosis,
  RotationStrategy,
  ViewMode,
} from '../types';
import { RotationList } from './RotationList';

type ChartTab = 'water' | 'soil' | 'temp';
type ExplorerTab = 'rotations' | 'calendar' | 'charts';

interface DiagnosisSummaryCardProps {
  language: Language;
  viewMode: ViewMode;
  loading: boolean;
  error: string | null;
  diagnosis: ParcelDiagnosis | null;
  baselineDiagnosis?: ParcelDiagnosis | null;
  focusedParcel: Parcel | null;
  strategies: RotationStrategy[];
  baselineStrategies?: RotationStrategy[];
  rainDeltaPct: number;
  tempDeltaC: number;
  onChangeScenario?: (rain: number, temp: number) => void;
}

interface DiagnosisExplorerProps {
  language: Language;
  viewMode: ViewMode;
  diagnosis: ParcelDiagnosis | null;
  strategies: RotationStrategy[];
  baselineStrategies?: RotationStrategy[];
  rainDeltaPct?: number;
  tempDeltaC?: number;
  reminder?: string;
}

function formatWeekdayShort(isoDate: string, language: Language): string {
  const dt = new Date(`${isoDate}T12:00:00`);
  if (Number.isNaN(dt.getTime())) return isoDate.slice(5);
  return dt.toLocaleDateString(language === 'en' ? 'en-US' : 'es-PE', {
    weekday: 'short',
  });
}

function MonthlyAnalytics({
  monthly,
  language,
  viewMode,
  diagnosis,
}: {
  monthly: MonthlyClimate[];
  language: Language;
  viewMode: ViewMode;
  diagnosis: ParcelDiagnosis;
}) {
  const [chartTab, setChartTab] = useState<ChartTab>('water');
  const [showTable, setShowTable] = useState(false);
  const [showTech, setShowTech] = useState(false);

  const w = 680;
  const h = 195;
  const leftPad = 40;
  const rightPad = 14;
  const topPad = 16;
  const bottomY = h - 28;
  const plotH = bottomY - topPad;
  const colW = (w - leftPad - rightPad) / 12;

  const maxWater = Math.max(
    100,
    ...monthly.map((m) => Math.max(m.precip_mm_month, m.evapotranspiration_mm_month)),
  );

  const minTempScale = Math.min(0, ...monthly.map((m) => m.temp_min_c)) - 2;
  const maxTempScale = Math.max(32, ...monthly.map((m) => m.temp_max_c)) + 2;
  const tempSpan = Math.max(10, maxTempScale - minTempScale);
  const yForTemp = (t: number) => bottomY - ((t - minTempScale) / tempSpan) * plotH;
  const frostY = yForTemp(2.0);
  const dryThresholdY = bottomY - 0.35 * plotH;

  const hc = diagnosis.historical_correlation;
  const techVisible = viewMode === 'agronomist' || showTech;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div
          className="flex flex-wrap items-center rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-1"
          role="tablist"
          aria-label={tr(language, 'Gráficos climáticos anuales', 'Annual climate charts')}
        >
          <button
            type="button"
            role="tab"
            aria-selected={chartTab === 'water'}
            onClick={() => setChartTab('water')}
            className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer ${
              chartTab === 'water'
                ? 'bg-white text-[#111813] shadow-xs'
                : 'text-[#475569] hover:text-[#111813]'
            }`}
          >
            <CloudRain className="h-4 w-4 text-[#1b4332]" />
            <span>
              {viewMode === 'farmer'
                ? tr(language, 'Lluvia vs. Agua que pide el suelo', 'Rain vs. Crop water demand')
                : tr(language, 'Precipitación vs. ET (mm/mes)', 'Precip vs. ET (mm/mo)')}
            </span>
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={chartTab === 'soil'}
            onClick={() => setChartTab('soil')}
            className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer ${
              chartTab === 'soil'
                ? 'bg-white text-[#111813] shadow-xs'
                : 'text-[#475569] hover:text-[#111813]'
            }`}
          >
            <Droplets className="h-4 w-4 text-[#0284c7]" />
            <span>{tr(language, 'Humedad de la tierra (SMAP)', 'Soil moisture (SMAP)')}</span>
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={chartTab === 'temp'}
            onClick={() => setChartTab('temp')}
            className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer ${
              chartTab === 'temp'
                ? 'bg-white text-[#111813] shadow-xs'
                : 'text-[#475569] hover:text-[#111813]'
            }`}
          >
            <Thermometer className="h-4 w-4 text-[#b45309]" />
            <span>{tr(language, 'Frío, heladas y calor', 'Cold, frost & heat')}</span>
          </button>
        </div>

        <div className="flex flex-wrap items-center gap-3 text-xs font-medium text-[#334155]">
          {chartTab === 'water' && (
            <>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-3 w-3 rounded-xs bg-[#1b4332]" />
                {tr(language, 'Lluvia que cae', 'Rainfall')}
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-3 w-3 rounded-xs bg-[#d97706]" />
                {tr(language, 'Evapotranspiración (ET: agua que pierde el campo)', 'ET (evaporation + plant loss)')}
              </span>
            </>
          )}
          {chartTab === 'soil' && (
            <>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-3 w-3 rounded-xs bg-[#0284c7]" />
                {tr(language, 'Capa superficial (0–5 cm)', 'Topsoil (0–5 cm)')}
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-3 w-3 rounded-xs bg-[#1b4332]" />
                {tr(language, 'Zona de raíces', 'Root zone')}
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-0.5 w-3.5 border-t-2 border-dashed border-[#b45309]" />
                {tr(language, 'Umbral seco (35%)', 'Dry threshold (35%)')}
              </span>
            </>
          )}
          {chartTab === 'temp' && (
            <>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-3 w-3 rounded-xs bg-[#f59e0b]" />
                {tr(language, 'Rango mín–máx mensual', 'Monthly min–max')}
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-2.5 w-2.5 rounded-full bg-[#111813]" />
                {tr(language, 'Media', 'Mean')}
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="h-0.5 w-3.5 border-t-2 border-dashed border-[#1d4ed8]" />
                {tr(language, 'Riesgo helada (2 °C)', 'Frost risk (2 °C)')}
              </span>
            </>
          )}
        </div>
      </div>

      <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3">
        <svg viewBox={`0 0 ${w} ${h}`} className="block h-auto w-full">
          <line x1={leftPad} y1={bottomY} x2={w - rightPad} y2={bottomY} stroke="#d4d8d1" />
          <line
            x1={leftPad}
            y1={topPad + plotH / 2}
            x2={w - rightPad}
            y2={topPad + plotH / 2}
            stroke="#e8ebe6"
            strokeDasharray="3 3"
          />

          {chartTab === 'water' && (
            <>
              <text x={leftPad - 5} y={bottomY + 3} fontSize={10} textAnchor="end" fill="#475569">
                0
              </text>
              <text
                x={leftPad - 5}
                y={topPad + plotH / 2 + 3}
                fontSize={10}
                textAnchor="end"
                fill="#475569"
              >
                {Math.round(maxWater / 2)}
              </text>
              <text x={leftPad - 5} y={topPad + 5} fontSize={10} textAnchor="end" fill="#475569">
                {Math.round(maxWater)}
              </text>
              {monthly.map((m, i) => {
                const x = leftPad + i * colW + 6;
                const pH = Math.max(3, (m.precip_mm_month / maxWater) * plotH);
                const eH = Math.max(3, (m.evapotranspiration_mm_month / maxWater) * plotH);
                return (
                  <g key={m.month}>
                    <rect x={x} y={bottomY - pH} width={16} height={pH} fill="#1b4332" rx={3}>
                      <title>
                        {m.month_label}: {tr(language, 'Lluvia', 'Rain')} {m.precip_mm_month} mm
                      </title>
                    </rect>
                    <rect x={x + 18} y={bottomY - eH} width={16} height={eH} fill="#d97706" rx={3}>
                      <title>
                        {m.month_label}: {tr(language, 'Evapotranspiración ET', 'ET Demand')}{' '}
                        {m.evapotranspiration_mm_month} mm
                      </title>
                    </rect>
                    <text x={x + 17} y={h - 9} fontSize={11} fontWeight={600} textAnchor="middle" fill="#334155">
                      {m.month_label}
                    </text>
                  </g>
                );
              })}
            </>
          )}

          {chartTab === 'soil' && (
            <>
              <text x={leftPad - 5} y={bottomY + 3} fontSize={10} textAnchor="end" fill="#475569">
                0%
              </text>
              <text
                x={leftPad - 5}
                y={topPad + plotH / 2 + 3}
                fontSize={10}
                textAnchor="end"
                fill="#475569"
              >
                50%
              </text>
              <text x={leftPad - 5} y={topPad + 5} fontSize={10} textAnchor="end" fill="#475569">
                100%
              </text>
              <line
                x1={leftPad}
                y1={dryThresholdY}
                x2={w - rightPad}
                y2={dryThresholdY}
                stroke="#b45309"
                strokeWidth={1.3}
                strokeDasharray="4 3"
              />
              {monthly.map((m, i) => {
                const x = leftPad + i * colW + 6;
                const topPct = Math.round(m.soil_moisture_top * 100);
                const rootPct = Math.round(m.soil_moisture_root * 100);
                const tH = Math.max(3, m.soil_moisture_top * plotH);
                const rH = Math.max(3, m.soil_moisture_root * plotH);
                return (
                  <g key={m.month}>
                    <rect x={x} y={bottomY - tH} width={16} height={tH} fill="#0284c7" rx={3}>
                      <title>
                        {m.month_label}: {tr(language, 'Superficial', 'Topsoil')} {topPct}%
                      </title>
                    </rect>
                    <rect x={x + 18} y={bottomY - rH} width={16} height={rH} fill="#1b4332" rx={3}>
                      <title>
                        {m.month_label}: {tr(language, 'Raíz', 'Root')} {rootPct}%
                      </title>
                    </rect>
                    <text x={x + 17} y={h - 9} fontSize={11} fontWeight={600} textAnchor="middle" fill="#334155">
                      {m.month_label}
                    </text>
                  </g>
                );
              })}
            </>
          )}

          {chartTab === 'temp' && (
            <>
              <text x={leftPad - 5} y={bottomY + 3} fontSize={10} textAnchor="end" fill="#475569">
                {Math.round(minTempScale)}°
              </text>
              <text
                x={leftPad - 5}
                y={topPad + plotH / 2 + 3}
                fontSize={10}
                textAnchor="end"
                fill="#475569"
              >
                {Math.round((minTempScale + maxTempScale) / 2)}°
              </text>
              <text x={leftPad - 5} y={topPad + 5} fontSize={10} textAnchor="end" fill="#475569">
                {Math.round(maxTempScale)}°
              </text>
              {frostY >= topPad && frostY <= bottomY && (
                <line
                  x1={leftPad}
                  y1={frostY}
                  x2={w - rightPad}
                  y2={frostY}
                  stroke="#1d4ed8"
                  strokeWidth={1.3}
                  strokeDasharray="4 3"
                />
              )}
              {monthly.map((m, i) => {
                const cx = leftPad + i * colW + colW / 2;
                const yMax = yForTemp(m.temp_max_c);
                const yMin = yForTemp(m.temp_min_c);
                const yMean = yForTemp(m.temp_mean_c);
                const barH = Math.max(5, yMin - yMax);
                return (
                  <g key={m.month}>
                    <rect
                      x={cx - 9}
                      y={yMax}
                      width={18}
                      height={barH}
                      fill={m.frost_risk ? '#93c5fd' : m.heat_stress ? '#fca5a5' : '#fcd34d'}
                      stroke={m.frost_risk ? '#1d4ed8' : m.heat_stress ? '#b91c1c' : '#d97706'}
                      strokeWidth={1}
                      rx={4}
                    >
                      <title>
                        {m.month_label}: {m.temp_min_c}° – {m.temp_max_c}°C ({m.temp_mean_c}°C)
                      </title>
                    </rect>
                    <circle cx={cx} cy={yMean} r={3.2} fill="#111813" />
                    <text x={cx} y={h - 9} fontSize={11} fontWeight={600} textAnchor="middle" fill="#334155">
                      {m.month_label}
                    </text>
                  </g>
                );
              })}
            </>
          )}
        </svg>
      </div>

      <div className="flex flex-wrap items-center gap-2 pt-1">
        <button
          type="button"
          aria-expanded={showTable}
          onClick={() => setShowTable((prev) => !prev)}
          className="inline-flex items-center gap-1.5 rounded-md border border-[#d6dad3] bg-[#fafaf8] px-3 py-1.5 text-xs font-semibold text-[#111813] hover:bg-[#e6f4ea] cursor-pointer"
        >
          <Table2 className="h-4 w-4 text-[#1b4332]" />
          <span>
            {showTable
              ? tr(language, 'Ocultar tabla numérica mensual', 'Hide monthly numbers table')
              : tr(language, 'Ver tabla numérica completa', 'View full monthly table')}
          </span>
          {showTable ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
        </button>

        {viewMode === 'farmer' && (
          <button
            type="button"
            aria-expanded={showTech}
            onClick={() => setShowTech((prev) => !prev)}
            className="inline-flex items-center gap-1.5 rounded-md border border-[#d6dad3] bg-[#fafaf8] px-3 py-1.5 text-xs font-semibold text-[#334155] hover:bg-[#f0f2ee] hover:text-[#111813] cursor-pointer"
          >
            <Database className="h-4 w-4 text-[#1b4332]" />
            <span>
              {showTech
                ? tr(language, 'Ocultar tendencia 2005–2024 y ficha NASA', 'Hide 2005–2024 trend & NASA specs')
                : tr(language, 'Ver tendencia 2005–2024 y ficha NASA', 'View 2005–2024 trend & NASA specs')}
            </span>
            {showTech ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
          </button>
        )}
      </div>

      {showTable && (
        <div className="overflow-x-auto rounded-lg border border-[#e3e6e1]">
          <table className="w-full min-w-[680px] border-collapse text-left text-xs">
            <thead>
              <tr className="border-b border-[#e3e6e1] bg-[#fafaf8] text-[#334155]">
                <th className="px-3 py-2 font-semibold">
                  {tr(language, 'Indicador mensual', 'Monthly metric')}
                </th>
                {monthly.map((m) => (
                  <th key={m.month} className="px-1.5 py-2 text-center font-mono font-semibold text-[#111813]">
                    {m.month_label}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-[#e3e6e1] bg-white">
              <tr>
                <td className="px-3 py-1.5 font-medium text-[#111813]">
                  {tr(language, 'Lluvia (mm)', 'Rain (mm)')}
                </td>
                {monthly.map((m) => (
                  <td key={m.month} className="px-1.5 py-1.5 text-center font-mono tabular-nums text-[#111813]">
                    {Math.round(m.precip_mm_month)}
                  </td>
                ))}
              </tr>
              <tr>
                <td className="px-3 py-1.5 font-medium text-[#111813]">
                  {tr(language, 'Consumo ET (mm)', 'Demand ET (mm)')}
                </td>
                {monthly.map((m) => (
                  <td key={m.month} className="px-1.5 py-1.5 text-center font-mono tabular-nums text-[#475569]">
                    {Math.round(m.evapotranspiration_mm_month)}
                  </td>
                ))}
              </tr>
              <tr>
                <td className="px-3 py-1.5 font-medium text-[#111813]">
                  {tr(language, 'Balance (mm)', 'Balance (mm)')}
                </td>
                {monthly.map((m) => (
                  <td
                    key={m.month}
                    className={`px-1.5 py-1.5 text-center font-mono font-semibold tabular-nums ${
                      m.water_balance_mm >= 0 ? 'text-[#1b4332]' : 'text-[#b45309]'
                    }`}
                  >
                    {m.water_balance_mm > 0 ? '+' : ''}
                    {Math.round(m.water_balance_mm)}
                  </td>
                ))}
              </tr>
              <tr>
                <td className="px-3 py-1.5 font-medium text-[#111813]">
                  {tr(language, 'Humedad Sup/Raíz (%)', 'Moist. Top/Root (%)')}
                </td>
                {monthly.map((m) => (
                  <td key={m.month} className="px-1.5 py-1.5 text-center font-mono tabular-nums text-[#111813]">
                    {Math.round(m.soil_moisture_top * 100)}/{Math.round(m.soil_moisture_root * 100)}
                  </td>
                ))}
              </tr>
              <tr>
                <td className="px-3 py-1.5 font-medium text-[#111813]">
                  {tr(language, 'Temp Mín/Máx (°C)', 'Min/Max Temp (°C)')}
                </td>
                {monthly.map((m) => (
                  <td key={m.month} className="px-1.5 py-1.5 text-center font-mono tabular-nums text-[#111813]">
                    {Math.round(m.temp_min_c)}/{Math.round(m.temp_max_c)}
                  </td>
                ))}
              </tr>
              <tr>
                <td className="px-3 py-1.5 font-medium text-[#111813]">
                  {tr(language, 'Sol / HR%', 'Solar / RH%')}
                </td>
                {monthly.map((m) => (
                  <td key={m.month} className="px-1.5 py-1.5 text-center font-mono tabular-nums text-[#475569]">
                    {m.solar_kwh_m2_day.toFixed(1)}/{Math.round(m.rh_pct)}
                  </td>
                ))}
              </tr>
            </tbody>
          </table>
        </div>
      )}

      {techVisible && (
        <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3.5 text-xs text-[#334155] space-y-2">
          <div className="flex flex-wrap items-center justify-between gap-2 font-mono text-xs text-[#111813]">
            <span>
              {diagnosis.lat}, {diagnosis.lon} · {diagnosis.data_source}
            </span>
            <span>{diagnosis.terrain.terrain_source}</span>
          </div>
          {hc && hc.live_historical && (
            <div className="flex flex-wrap gap-x-5 gap-y-1.5 font-mono text-xs">
              <span>
                {tr(language, 'Tendencia 2005–2024:', '2005–2024 trend:')}{' '}
                <strong className="text-[#111813]">
                  {hc.temp_trend_c_per_decade > 0 ? '+' : ''}
                  {hc.temp_trend_c_per_decade} °C/dec
                </strong>
              </span>
              <span>
                {tr(language, 'Cambio lluvia 5a:', '5y rain change:')}{' '}
                <strong className="text-[#111813]">
                  {hc.precip_change_pct_recent_5y > 0 ? '+' : ''}
                  {hc.precip_change_pct_recent_5y}%
                </strong>
              </span>
              <span>
                {tr(language, 'Años secos:', 'Dry years:')}{' '}
                <strong className="text-[#111813]">
                  {hc.drought_years_last_20}/{hc.years_analyzed || 20}
                </strong>
              </span>
              <span>
                {tr(language, 'Corr. lluvia↔humedad:', 'Rain↔moisture:')}{' '}
                <strong className="text-[#111813]">r = {hc.rain_soil_moisture_corr}</strong>
              </span>
              {hc.latest_ndvi_date && (
                <span>
                  {tr(language, 'Sentinel-2 NDVI:', 'Sentinel-2 NDVI:')}{' '}
                  <strong className="text-[#111813]">
                    {hc.latest_ndvi?.toFixed(2) ?? 'N/D'} ({hc.latest_ndvi_date})
                  </strong>
                </span>
              )}
            </div>
          )}
          <p className="text-xs leading-relaxed text-[#475569]">
            {hc
              ? viewMode === 'agronomist'
                ? hc.correlation_interpretation_agronomist
                : hc.correlation_interpretation_farmer
              : diagnosis.agronomist_summary}
          </p>
        </div>
      )}
    </div>
  );
}

export function DiagnosisSummaryCard({
  language,
  viewMode,
  loading,
  error,
  diagnosis,
  baselineDiagnosis,
  focusedParcel,
  strategies,
  baselineStrategies,
  rainDeltaPct,
  tempDeltaC,
  onChangeScenario,
}: DiagnosisSummaryCardProps) {
  const vegetationLabels: Record<string, string> = {
    suelo_desnudo_o_sin_cobertura: tr(language, 'Poca cobertura vegetal', 'Low vegetative cover'),
    cobertura_baja_o_estres: tr(language, 'Cobertura media / estrés', 'Moderate cover / stress'),
    vegetacion_vigorosa: tr(language, 'Pasto / cultivo vigoroso', 'Healthy green cover'),
  };

  const erosionLabels: Record<'low' | 'moderate' | 'high' | 'unknown', string> = {
    low: tr(language, 'Pendiente baja (terreno firme)', 'Low slope (stable)'),
    moderate: tr(language, 'Pendiente moderada (proteger suelo)', 'Moderate slope (cover needed)'),
    high: tr(language, 'Pendiente alta (riesgo lavado)', 'Steep slope (erosion risk)'),
    unknown: tr(language, 'Relieve sin dato', 'Terrain N/A'),
  };

  const erosionBadgeClass: Record<'low' | 'moderate' | 'high' | 'unknown', string> = {
    low: 'bg-[#dcfce7] text-[#14532d]',
    moderate: 'bg-[#fef3c7] text-[#92400e]',
    high: 'bg-[#fee2e2] text-[#991b1b]',
    unknown: 'bg-[#f1f3f0] text-[#475569]',
  };

  if (loading && !diagnosis) {
    return (
      <section className="flex h-full flex-col justify-center rounded-xl border border-[#e3e6e1] bg-white p-6 text-center">
        <CloudRain className="mx-auto h-8 w-8 text-[#1b4332]" />
        <h2 className="mt-3 text-base font-bold text-[#111813]">
          {tr(language, 'Consultando climatología NASA y satélites...', 'Loading NASA climatology & satellites...')}
        </h2>
        <p className="mt-1.5 text-sm text-[#475569]">
          {tr(
            language,
            'Calculando lluvia anual, humedad en tierra, relieve SRTM y opciones de rotación.',
            'Computing annual rainfall, soil moisture, SRTM terrain, and crop rotation options.',
          )}
        </p>
      </section>
    );
  }

  if (error && !loading) {
    return (
      <section className="flex h-full items-start gap-3 rounded-xl border border-[#fecaca] bg-[#fef2f2] p-5 text-sm text-[#991b1b]">
        <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0" />
        <div>
          <h2 className="text-base font-bold">
            {tr(language, 'No se pudo consultar NASA POWER en vivo', 'Live NASA POWER analysis unavailable')}
          </h2>
          <p className="mt-1 text-xs sm:text-sm">{error}</p>
        </div>
      </section>
    );
  }

  if (!diagnosis) {
    return (
      <section className="flex h-full flex-col justify-center rounded-xl border border-[#e3e6e1] bg-white p-6 text-center">
        <Compass className="mx-auto h-8 w-8 text-[#1b4332]" />
        <h2 className="mt-3 text-base font-bold text-[#111813]">
          {tr(language, '¿Qué conviene sembrar ante un clima cambiante?', 'What should you plant as climate shifts?')}
        </h2>
        <p className="mt-1.5 text-sm text-[#475569]">
          {tr(
            language,
            'Elige una parcela arriba o marca un punto del mapa para ver la comparación entre tu práctica actual y la alternativa recomendada.',
            'Choose a field above or drop a map pin to compare your current practice against a climate-resilient alternative.',
          )}
        </p>
      </section>
    );
  }

  const monthly = diagnosis.monthly ?? [];
  const minAnnualTemp = monthly.length
    ? Math.round(Math.min(...monthly.map((m) => m.temp_min_c)))
    : null;
  const maxAnnualTemp = monthly.length
    ? Math.round(Math.max(...monthly.map((m) => m.temp_max_c)))
    : null;

  const topMoistPct = Math.round(diagnosis.avg_soil_moisture_top * 100);
  const rootMoistPct = Math.round(diagnosis.avg_soil_moisture_root * 100);
  const maxRainEt = Math.max(diagnosis.annual_precip_mm, diagnosis.annual_et_mm, 1);
  const rainBarPct = Math.round((diagnosis.annual_precip_mm / maxRainEt) * 100);
  const etBarPct = Math.round((diagnosis.annual_et_mm / maxRainEt) * 100);

  const hc = diagnosis.historical_correlation ?? null;
  const ndviOk = hc !== null && hc.latest_ndvi !== null && hc.latest_ndvi !== undefined;
  const ndviValue = ndviOk && hc?.latest_ndvi !== null && hc?.latest_ndvi !== undefined ? hc.latest_ndvi : 0;
  const ndviPct = Math.max(0, Math.min(100, Math.round(ndviValue * 100)));

  const optimalMonthsCount = monthly.filter(
    (m) => !m.frost_risk && !m.heat_stress && !m.dry_stress,
  ).length;

  const currentStrategy = strategies.find((s) => s.is_current) ?? null;
  const nonCurrent = strategies.filter((s) => !s.is_current);
  const topStrategy =
    (nonCurrent.length ? [...nonCurrent].sort((a, b) => b.fit_score - a.fit_score)[0] : strategies[0]) ??
    null;

  const currentCoveredCount = currentStrategy
    ? currentStrategy.monthly_calendar.filter((c) => c.status === 'covered').length
    : 0;
  const topCoveredCount = topStrategy
    ? topStrategy.monthly_calendar.filter((c) => c.status === 'covered').length
    : 0;

  const baselineNonCurrent = (baselineStrategies ?? []).filter((s) => !s.is_current);
  const baselineTopStrategy =
    baselineNonCurrent.length > 0
      ? [...baselineNonCurrent].sort((a, b) => b.fit_score - a.fit_score)[0]
      : null;

  const scenarioActive = rainDeltaPct !== 0 || tempDeltaC !== 0;
  const rankChanged =
    Boolean(baselineTopStrategy && topStrategy && baselineTopStrategy.id !== topStrategy.id);
  const scoreDiff =
    baselineTopStrategy && topStrategy
      ? Math.round(topStrategy.fit_score - baselineTopStrategy.fit_score)
      : 0;

  const isDroughtScenario = rainDeltaPct === -25 && tempDeltaC === 1.5;

  return (
    <section
      aria-label={tr(language, 'Centro de decisiones agrícolas', 'Agricultural decision center')}
      className="flex h-full flex-col justify-between gap-4 rounded-xl border border-[#e3e6e1] bg-white p-4 sm:p-5"
    >
      {/* Decision Center Header */}
      <div className="space-y-3">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <h2 className="text-base font-bold leading-snug text-[#111813] sm:text-lg">
              {tr(
                language,
                '¿Qué conviene sembrar ante un clima cambiante?',
                'What should you plant as climate shifts?',
              )}
            </h2>
            <p className="mt-0.5 text-xs text-[#475569] sm:text-sm">
              {focusedParcel ? (
                <>
                  <strong className="text-[#111813]">{focusedParcel.name}</strong> ·{' '}
                </>
              ) : null}
              {tr(
                language,
                'Climatología NASA POWER (30a) + prioridades del productor.',
                'NASA POWER climatology (30y) + farmer field priorities.',
              )}
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {onChangeScenario && (
              <div className="inline-flex rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-0.5">
                <button
                  type="button"
                  onClick={() => onChangeScenario(0, 0)}
                  aria-pressed={!scenarioActive}
                  className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-semibold transition-colors cursor-pointer ${
                    !scenarioActive
                      ? 'bg-[#1b4332] text-white'
                      : 'text-[#334155] hover:text-[#111813]'
                  }`}
                >
                  <Sun className="h-3.5 w-3.5" />
                  <span>{tr(language, 'Clima actual', 'Current climate')}</span>
                </button>
                <button
                  type="button"
                  onClick={() => onChangeScenario(-25, 1.5)}
                  aria-pressed={isDroughtScenario}
                  className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-semibold transition-colors cursor-pointer ${
                    isDroughtScenario
                      ? 'bg-[#b45309] text-white'
                      : 'text-[#334155] hover:text-[#92400e]'
                  }`}
                >
                  <Flame className="h-3.5 w-3.5" />
                  <span>{tr(language, 'Sequía −25%', 'Drought −25%')}</span>
                </button>
              </div>
            )}

            <span className="rounded-md bg-[#dcfce7] px-2.5 py-1 text-xs font-semibold text-[#14532d] tabular-nums">
              {optimalMonthsCount}/12 {tr(language, 'meses favorables', 'favorable mo')}
            </span>
          </div>
        </div>

        {/* Before vs After Impact Box when Drought/Rain Scenario Active */}
        {scenarioActive && baselineDiagnosis && (
          <div className="rounded-lg border border-[#fde68a] bg-[#fffbeb] px-3.5 py-2.5 text-xs text-[#78350f]">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <strong className="font-bold text-[#92400e]">
                {tr(
                  language,
                  '¿Cambia la recomendación? Clima actual frente a escenario simulado:',
                  'Does the recommendation change? Current climate vs. scenario:',
                )}
              </strong>
              <span className="font-mono font-semibold tabular-nums">
                {tr(language, 'Lluvia:', 'Rain:')} {baselineDiagnosis.annual_precip_mm} mm →{' '}
                {diagnosis.annual_precip_mm} mm · {tr(language, 'Meses secos:', 'Dry mo:')}{' '}
                {baselineDiagnosis.dry_months.length} → {diagnosis.dry_months.length}
              </span>
            </div>
            <p className="mt-1 text-xs text-[#78350f]">
              {rankChanged && topStrategy && baselineTopStrategy
                ? tr(
                    language,
                    `Sí cambia la prioridad: bajo este escenario el modelo prioriza "${topStrategy.name}" (${Math.round(topStrategy.fit_score)}/100) en lugar de "${baselineTopStrategy.name}".`,
                    `Yes — under this scenario the model shifts priority to "${topStrategy.name}" (${Math.round(topStrategy.fit_score)}/100) instead of "${baselineTopStrategy.name}".`,
                  )
                : topStrategy
                  ? tr(
                      language,
                      `Se mantiene como líder "${topStrategy.name}" (${Math.round(topStrategy.fit_score)}/100${scoreDiff !== 0 ? `, variación ${scoreDiff > 0 ? `+${scoreDiff}` : scoreDiff} pts respecto al clima normal` : ''}).`,
                      `"${topStrategy.name}" stays top-ranked (${Math.round(topStrategy.fit_score)}/100${scoreDiff !== 0 ? `, ${scoreDiff > 0 ? `+${scoreDiff}` : scoreDiff} pts vs normal climate` : ''}).`,
                    )
                  : null}
            </p>
          </div>
        )}

        {/* Current Situation vs Suggested Model Alternative */}
        {currentStrategy && topStrategy && (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <div className="rounded-lg border border-[#fde68a] bg-[#fffdf7] p-3.5 space-y-2">
              <div className="flex items-center justify-between gap-2">
                <span className="rounded bg-[#fef3c7] px-2 py-0.5 text-xs font-semibold text-[#92400e]">
                  {tr(language, 'Situación actual', 'Current situation')}
                </span>
                <span className="font-mono text-xs font-semibold text-[#78350f] tabular-nums">
                  {currentCoveredCount}/12 {tr(language, 'meses cubierto', 'mo covered')}
                </span>
              </div>
              <h3 className="text-sm font-bold text-[#111813]">{currentStrategy.name}</h3>
              <p className="text-xs leading-relaxed text-[#475569]">
                {viewMode === 'farmer'
                  ? currentStrategy.why_suggested_farmer
                  : currentStrategy.why_suggested_agronomist}
              </p>
              <p className="text-xs font-medium text-[#92400e]">
                {tr(language, 'Riesgos:', 'Risks:')} {diagnosis.dry_months.length}{' '}
                {tr(language, 'meses secos', 'dry mo')} · {diagnosis.frost_risk_months.length}{' '}
                {tr(language, 'meses con helada', 'frost mo')}
              </p>
            </div>

            <div className="rounded-lg border border-[#1b4332]/40 bg-[#f4fbf6] p-3.5 space-y-2">
              <div className="flex items-center justify-between gap-2">
                <span className="inline-flex items-center gap-1 rounded bg-[#1b4332] px-2 py-0.5 text-xs font-semibold text-white">
                  <Award className="h-3.5 w-3.5" />
                  {tr(language, 'Mayor adecuación (modelo)', 'Highest-fit (model)')}
                </span>
                <span className="rounded-full bg-[#dcfce7] px-2.5 py-0.5 font-mono text-xs font-bold text-[#14532d] tabular-nums">
                  {Math.round(topStrategy.fit_score)}/100
                </span>
              </div>

              <h3 className="text-sm font-bold text-[#111813]">{topStrategy.name}</h3>

              <div className="flex flex-wrap items-center gap-1">
                {topStrategy.sequence.map((step, idx) => (
                  <div key={`${step}-${idx}`} className="flex items-center gap-1">
                    <span className="rounded border border-[#bbf7d0] bg-white px-2 py-0.5 text-xs font-semibold text-[#111813]">
                      {step}
                    </span>
                    {idx < topStrategy.sequence.length - 1 && (
                      <ArrowRight className="h-3.5 w-3.5 text-[#1b4332]" />
                    )}
                  </div>
                ))}
              </div>

              <p className="text-xs leading-relaxed text-[#334155]">
                {viewMode === 'farmer'
                  ? topStrategy.why_suggested_farmer
                  : topStrategy.why_suggested_agronomist}
              </p>

              <p className="text-xs font-medium text-[#14532d] tabular-nums">
                {topCoveredCount}/12 {tr(language, 'meses suelo cubierto · Verificar en campo:', 'mo covered · Verify locally:')}{' '}
                <span className="font-normal text-[#334155]">
                  {topStrategy.local_review_needed[0]}
                </span>
              </p>
            </div>
          </div>
        )}
      </div>

      {/* 4 Field Vitals with Immediate Agricultural Explanations */}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {/* 1. Rain vs ET */}
        <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3.5 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813] sm:text-sm">
              <CloudRain className="h-4 w-4 text-[#2563eb]" />
              {tr(language, 'Lluvia vs. Evapotranspiración (ET)', 'Rain vs. ET Demand')}
            </span>
            <span
              className={`rounded-full px-2 py-0.5 text-xs font-semibold ${
                diagnosis.annual_water_balance_mm >= 0
                  ? 'bg-[#dcfce7] text-[#14532d]'
                  : 'bg-[#fef3c7] text-[#92400e]'
              }`}
            >
              {diagnosis.annual_water_balance_mm >= 0
                ? tr(language, 'Superávit', 'Surplus')
                : tr(language, 'Déficit hídrico', 'Water deficit')}
            </span>
          </div>

          <div className="flex items-baseline gap-2">
            <strong className="font-mono text-lg font-bold text-[#111813] tabular-nums">
              {diagnosis.annual_precip_mm} mm
            </strong>
            <span className="font-mono text-xs text-[#475569] tabular-nums">
              ({tr(language, 'Balance:', 'Balance:')}{' '}
              {diagnosis.annual_water_balance_mm > 0 ? '+' : ''}
              {diagnosis.annual_water_balance_mm} mm)
            </span>
          </div>

          <div className="space-y-1.5 text-xs text-[#334155]">
            <div className="flex justify-between">
              <span>{tr(language, 'Lluvia promedio anual', 'Annual rainfall')}</span>
              <span className="font-mono font-semibold text-[#111813]">{diagnosis.annual_precip_mm} mm</span>
            </div>
            <div className="h-2 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
              <div className="h-full rounded-full bg-[#1b4332]" style={{ width: `${rainBarPct}%` }} />
            </div>
            <div className="flex justify-between">
              <span>{tr(language, 'Demanda ET (evapora + planta)', 'ET demand (evap + crop)')}</span>
              <span className="font-mono font-semibold text-[#111813]">{diagnosis.annual_et_mm} mm</span>
            </div>
            <div className="h-2 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
              <div className="h-full rounded-full bg-[#d97706]" style={{ width: `${etBarPct}%` }} />
            </div>
          </div>
        </div>

        {/* 2. Soil Moisture (SMAP) */}
        <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3.5 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813] sm:text-sm">
              <Droplets className="h-4 w-4 text-[#0284c7]" />
              {tr(language, 'Humedad suelo (SMAP)', 'Soil moisture (SMAP)')}
            </span>
            <span
              className={`rounded-full px-2 py-0.5 text-xs font-semibold ${
                rootMoistPct >= 55
                  ? 'bg-[#dcfce7] text-[#14532d]'
                  : rootMoistPct >= 35
                    ? 'bg-[#dbeafe] text-[#1e3a8a]'
                    : 'bg-[#fef3c7] text-[#92400e]'
              }`}
            >
              {rootMoistPct >= 55
                ? tr(language, 'Húmeda', 'Wet')
                : rootMoistPct >= 35
                  ? tr(language, 'Media', 'Moderate')
                  : tr(language, 'Seca', 'Dry')}
            </span>
          </div>

          <strong className="block font-mono text-lg font-bold text-[#111813] tabular-nums">
            {rootMoistPct}%{' '}
            <span className="font-sans text-xs font-normal text-[#475569]">
              {tr(language, 'humedad en zona de raíces', 'root-zone moisture')}
            </span>
          </strong>

          <div className="space-y-1.5 text-xs text-[#334155]">
            <div className="flex justify-between">
              <span>{tr(language, 'En raíz (0–100 cm)', 'Root zone (0–100 cm)')}</span>
              <span className="font-mono font-semibold text-[#111813]">{rootMoistPct}%</span>
            </div>
            <div className="h-2 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
              <div className="h-full rounded-full bg-[#1b4332]" style={{ width: `${rootMoistPct}%` }} />
            </div>
            <div className="flex justify-between">
              <span>{tr(language, 'Superficie (0–5 cm)', 'Topsoil (0–5 cm)')}</span>
              <span className="font-mono font-semibold text-[#111813]">{topMoistPct}%</span>
            </div>
            <div className="h-2 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
              <div className="h-full rounded-full bg-[#0284c7]" style={{ width: `${topMoistPct}%` }} />
            </div>
          </div>
          <p className="text-xs text-[#475569]">
            {tr(
              language,
              'SMAP mide por microondas cuánta agua retiene la tierra.',
              'SMAP measures water retained in topsoil and root zone.',
            )}
          </p>
        </div>

        {/* 3. Climate, Frost, Heat & Terrain */}
        <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3.5 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813] sm:text-sm">
              <Thermometer className="h-4 w-4 text-[#b45309]" />
              {tr(language, 'Clima, altura y relieve', 'Climate & terrain')}
            </span>
            <span
              className={`rounded-full px-2 py-0.5 text-xs font-semibold ${
                erosionBadgeClass[diagnosis.terrain.erosion_risk]
              }`}
            >
              {erosionLabels[diagnosis.terrain.erosion_risk]}
            </span>
          </div>

          <strong className="block font-mono text-lg font-bold text-[#111813] tabular-nums">
            {diagnosis.annual_temp_mean_c} °C{' '}
            <span className="font-sans text-xs font-normal text-[#475569]">
              ({minAnnualTemp ?? '—'}° a {maxAnnualTemp ?? '—'}°)
            </span>
          </strong>

          <div className="flex flex-wrap gap-1.5 text-xs font-medium">
            <span
              className={`inline-flex items-center gap-1 rounded px-2 py-0.5 ${
                diagnosis.frost_risk_months.length > 0
                  ? 'bg-[#dbeafe] text-[#1e3a8a]'
                  : 'bg-[#dcfce7] text-[#14532d]'
              }`}
            >
              <Snowflake className="h-3.5 w-3.5" />
              {diagnosis.frost_risk_months.length} {tr(language, 'meses helada', 'frost mo')}
            </span>
            <span
              className={`inline-flex items-center gap-1 rounded px-2 py-0.5 ${
                diagnosis.heat_stress_months.length > 0
                  ? 'bg-[#ffedd5] text-[#9a3412]'
                  : 'bg-[#dcfce7] text-[#14532d]'
              }`}
            >
              <Sun className="h-3.5 w-3.5" />
              {diagnosis.heat_stress_months.length} {tr(language, 'meses calor', 'heat mo')}
            </span>
            {diagnosis.terrain.live_terrain_data && (
              <span className="inline-flex items-center gap-1 rounded border border-[#e3e6e1] bg-white px-2 py-0.5 font-mono text-[#111813]">
                <Mountain className="h-3.5 w-3.5 text-[#1b4332]" />
                {diagnosis.terrain.elevation_m} m · {diagnosis.terrain.slope_pct}%
              </span>
            )}
          </div>
          {viewMode === 'agronomist' && hc && (
            <p className="font-mono text-xs text-[#475569]">
              {tr(language, 'Tendencia 2005–2024:', '2005–2024 trend:')}{' '}
              {hc.temp_trend_c_per_decade > 0 ? '+' : ''}
              {hc.temp_trend_c_per_decade} °C/dec · {tr(language, 'Lluvia 5a:', '5y rain:')}{' '}
              {hc.precip_change_pct_recent_5y > 0 ? '+' : ''}
              {hc.precip_change_pct_recent_5y}%
            </p>
          )}
        </div>

        {/* 4. Vegetation Vigor (NDVI Sentinel-2) */}
        <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3.5 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813] sm:text-sm">
              <Sprout className="h-4 w-4 text-[#15803d]" />
              {tr(language, 'Verdor del cultivo (NDVI)', 'Crop greenness (NDVI)')}
            </span>
            <span
              className={`rounded-full px-2 py-0.5 text-xs font-semibold ${
                !ndviOk
                  ? 'bg-[#f1f3f0] text-[#475569]'
                  : ndviValue >= 0.45
                    ? 'bg-[#dcfce7] text-[#14532d]'
                    : ndviValue >= 0.2
                      ? 'bg-[#fef3c7] text-[#92400e]'
                      : 'bg-[#fee2e2] text-[#991b1b]'
              }`}
            >
              {ndviOk && hc
                ? vegetationLabels[hc.vegetation_status] || tr(language, 'Vigor medio', 'Moderate')
                : tr(language, 'Con nubes', 'Cloudy pass')}
            </span>
          </div>

          <div className="flex flex-wrap items-baseline justify-between gap-1">
            <strong className="font-mono text-lg font-bold text-[#111813] tabular-nums">
              {ndviOk ? `${ndviPct}% (${ndviValue.toFixed(2)})` : '—'}
            </strong>
            {hc?.latest_ndvi_date && (
              <span className="font-mono text-xs text-[#475569]">
                Sentinel-2: {hc.latest_ndvi_date}
              </span>
            )}
          </div>

          <div className="h-2 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
            <div
              className={`h-full rounded-full ${
                !ndviOk
                  ? 'bg-[#cbd5e1]'
                  : ndviValue < 0.2
                    ? 'bg-[#b45309]'
                    : ndviValue < 0.45
                      ? 'bg-[#d97706]'
                      : 'bg-[#1b4332]'
              }`}
              style={{ width: `${ndviOk ? ndviPct : 0}%` }}
            />
          </div>

          <p className="text-xs leading-snug text-[#475569]">
            {viewMode === 'agronomist' && hc
              ? `${hc.sentinel2_clear_scenes_12m} escenas claras (12m) · r=${hc.rain_soil_moisture_corr} lluvia↔suelo.`
              : tr(
                  language,
                  'NDVI indica qué tan cubierta y activa está la vegetación observada por Sentinel-2.',
                  'NDVI measures green vegetative cover observed by Sentinel-2.',
                )}
          </p>
        </div>
      </div>

      <p className="flex items-start gap-1.5 text-xs text-[#475569]">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-[#1b4332]" />
        <span>
          {tr(
            language,
            'Climatología histórica (NASA POWER 30a) + tendencia 2005–2024 + observación Sentinel-2 fechada + pronóstico 7 días (Open-Meteo).',
            'Historical climatology (30y NASA POWER) + 2005–2024 trend + dated Sentinel-2 pass + 7-day forecast (Open-Meteo).',
          )}
        </span>
      </p>
    </section>
  );
}

export function DiagnosisExplorer({
  language,
  viewMode,
  diagnosis,
  strategies,
  baselineStrategies,
  rainDeltaPct = 0,
  tempDeltaC = 0,
  reminder,
}: DiagnosisExplorerProps) {
  // P0 fix: Rotation comparison is the default opening tab because it directly answers the planting decision
  const [activeTab, setActiveTab] = useState<ExplorerTab>('rotations');

  if (!diagnosis) {
    return null;
  }

  const monthly = diagnosis.monthly ?? [];

  return (
    <section
      aria-label={tr(language, 'Comparación y detalle del campo', 'Field comparison and details')}
      className="rounded-xl border border-[#e3e6e1] bg-white p-4 sm:p-5 space-y-4"
    >
      {/* 3 Big Clear Tabs — Rotations first! */}
      <div
        role="tablist"
        aria-label={tr(language, 'Vistas detalladas del campo', 'Field detail tabs')}
        className="grid grid-cols-1 gap-2 sm:grid-cols-3"
      >
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'rotations'}
          onClick={() => setActiveTab('rotations')}
          className={`flex items-center justify-center gap-2 rounded-lg border px-3 py-2.5 text-xs font-bold transition-colors cursor-pointer sm:text-sm ${
            activeTab === 'rotations'
              ? 'border-[#1b4332] bg-[#1b4332] text-white'
              : 'border-[#e3e6e1] bg-[#fafaf8] text-[#111813] hover:bg-[#e6f4ea]'
          }`}
        >
          <Sprout className="h-4 w-4 shrink-0" />
          <span>
            {tr(
              language,
              `1. Comparar Rotaciones (${strategies.length})`,
              `1. Compare Rotations (${strategies.length})`,
            )}
          </span>
        </button>

        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'calendar'}
          onClick={() => setActiveTab('calendar')}
          className={`flex items-center justify-center gap-2 rounded-lg border px-3 py-2.5 text-xs font-bold transition-colors cursor-pointer sm:text-sm ${
            activeTab === 'calendar'
              ? 'border-[#1b4332] bg-[#1b4332] text-white'
              : 'border-[#e3e6e1] bg-[#fafaf8] text-[#111813] hover:bg-[#e6f4ea]'
          }`}
        >
          <CalendarDays className="h-4 w-4 shrink-0" />
          <span>{tr(language, '2. Pronóstico 7 Días y Año', '2. 7-Day Forecast & Year')}</span>
        </button>

        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'charts'}
          onClick={() => setActiveTab('charts')}
          className={`flex items-center justify-center gap-2 rounded-lg border px-3 py-2.5 text-xs font-bold transition-colors cursor-pointer sm:text-sm ${
            activeTab === 'charts'
              ? 'border-[#1b4332] bg-[#1b4332] text-white'
              : 'border-[#e3e6e1] bg-[#fafaf8] text-[#111813] hover:bg-[#e6f4ea]'
          }`}
        >
          <BarChart3 className="h-4 w-4 shrink-0" />
          <span>{tr(language, '3. Gráficos Agua, Suelo y NASA', '3. Water, Soil & NASA Data')}</span>
        </button>
      </div>

      {/* Tab 1 (Default): Rotation Impact Comparator & Options */}
      {activeTab === 'rotations' && (
        <RotationList
          language={language}
          viewMode={viewMode}
          strategies={strategies}
          baselineStrategies={baselineStrategies}
          rainDeltaPct={rainDeltaPct}
          tempDeltaC={tempDeltaC}
          reminder={reminder}
        />
      )}

      {/* Tab 2: 7-Day Weather + 12-Month Traffic-Light */}
      {activeTab === 'calendar' && (
        <div className="space-y-5">
          {diagnosis.weekly_alerts && (
            <div className="space-y-2.5">
              <div className="flex flex-wrap items-center justify-between gap-2 text-xs sm:text-sm">
                <strong className="text-[#111813]">
                  {tr(language, 'Pronóstico próximos 7 días (Open-Meteo)', 'Next 7 days forecast (Open-Meteo)')}
                </strong>
                <span className="font-medium text-[#1b4332]">
                  {diagnosis.weekly_alerts.farmer_alert_message}
                </span>
              </div>

              <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 md:grid-cols-7">
                {(diagnosis.weekly_alerts.days || []).map((day) => {
                  const WeatherIcon = day.is_storm
                    ? CloudLightning
                    : day.is_frost
                      ? Snowflake
                      : day.is_rain
                        ? CloudRain
                        : Sun;
                  const iconColor = day.is_storm
                    ? 'text-[#b91c1c]'
                    : day.is_frost
                      ? 'text-[#1d4ed8]'
                      : day.is_rain
                        ? 'text-[#0284c7]'
                        : 'text-[#d97706]';

                  return (
                    <div
                      key={day.date}
                      className="flex flex-col items-center rounded-lg border border-[#e3e6e1] bg-[#fafaf8] px-2 py-2.5 text-center tabular-nums"
                    >
                      <span className="text-xs font-bold capitalize text-[#111813]">
                        {formatWeekdayShort(day.date, language)}
                      </span>
                      <span className="font-mono text-[11px] text-[#475569]">{day.date.slice(5)}</span>
                      <WeatherIcon className={`my-1.5 h-5 w-5 ${iconColor}`} />
                      <div className="w-full px-1">
                        <div className="h-1.5 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
                          <div
                            className="h-full rounded-full bg-[#0284c7]"
                            style={{ width: `${Math.min(100, Math.max(0, day.precip_prob_pct))}%` }}
                          />
                        </div>
                        <span className="mt-1 block font-mono text-xs font-semibold text-[#111813]">
                          {day.precip_mm} mm · {day.precip_prob_pct}%
                        </span>
                      </div>
                      <span className="mt-0.5 font-mono text-[11px] text-[#475569]">
                        {Math.round(day.temp_min_c)}° / {Math.round(day.temp_max_c)}°
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {monthly.length > 0 && (
            <div className="space-y-2.5 border-t border-[#e3e6e1] pt-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h3 className="text-sm font-bold text-[#111813]">
                  {tr(language, 'Semáforo climatológico anual (30a NASA POWER)', 'Annual climatology traffic light (30y NASA POWER)')}
                </h3>
                <div className="flex flex-wrap gap-3 text-xs font-medium text-[#334155]">
                  <span className="inline-flex items-center gap-1">
                    <span className="h-2.5 w-2.5 rounded-xs bg-[#86efac]" />
                    {tr(language, 'Bueno', 'Good')}
                  </span>
                  <span className="inline-flex items-center gap-1">
                    <span className="h-2.5 w-2.5 rounded-xs bg-[#fde68a]" />
                    {tr(language, 'Seco', 'Dry')}
                  </span>
                  <span className="inline-flex items-center gap-1">
                    <span className="h-2.5 w-2.5 rounded-xs bg-[#bfdbfe]" />
                    {tr(language, 'Helada', 'Frost')}
                  </span>
                  <span className="inline-flex items-center gap-1">
                    <span className="h-2.5 w-2.5 rounded-xs bg-[#fed7aa]" />
                    {tr(language, 'Calor', 'Heat')}
                  </span>
                </div>
              </div>

              <div className="grid grid-cols-6 gap-1.5 sm:grid-cols-12">
                {monthly.map((m) => {
                  const status = m.frost_risk
                    ? {
                        label: tr(language, 'Helada', 'Frost'),
                        cls: 'border-[#bfdbfe] bg-[#dbeafe] text-[#1e3a8a]',
                        Icon: Snowflake,
                      }
                    : m.heat_stress
                      ? {
                          label: tr(language, 'Calor', 'Heat'),
                          cls: 'border-[#fed7aa] bg-[#ffedd5] text-[#9a3412]',
                          Icon: Thermometer,
                        }
                      : m.dry_stress
                        ? {
                            label: tr(language, 'Seco', 'Dry'),
                            cls: 'border-[#fde68a] bg-[#fef3c7] text-[#92400e]',
                            Icon: AlertTriangle,
                          }
                        : {
                            label: tr(language, 'Bueno', 'Good'),
                            cls: 'border-[#bbf7d0] bg-[#dcfce7] text-[#14532d]',
                            Icon: Sprout,
                          };

                  const StatusIcon = status.Icon;
                  return (
                    <div
                      key={m.month}
                      className={`flex flex-col items-center rounded-lg border px-1 py-2 text-center ${status.cls}`}
                      title={`${m.month_label}: ${m.precip_mm_month} mm · ${Math.round(m.soil_moisture_root * 100)}%`}
                    >
                      <span className="font-mono text-xs font-bold leading-none">
                        {m.month_label}
                      </span>
                      <StatusIcon className="my-1.5 h-3.5 w-3.5" />
                      <span className="text-[11px] font-semibold leading-none">{status.label}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Tab 3: Clean Visual Charts + Optional Table & NASA Technical Specs */}
      {activeTab === 'charts' && (
        <MonthlyAnalytics
          monthly={monthly}
          language={language}
          viewMode={viewMode}
          diagnosis={diagnosis}
        />
      )}
    </section>
  );
}
