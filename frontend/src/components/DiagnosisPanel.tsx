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
  focusedParcel: Parcel | null;
  strategies: RotationStrategy[];
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

      {/* Technical KPI Strip (ET, Sentinel-2 NDVI, SRTM Terrain) moved cleanly to Tab 3 */}
      {(() => {
        const vegetationLabels: Record<string, string> = {
          suelo_desnudo_o_sin_cobertura: tr(language, 'Poca cobertura vegetal', 'Low vegetative cover'),
          cobertura_baja_o_estres: tr(language, 'Cobertura media / estrés', 'Moderate cover / stress'),
          vegetacion_vigorosa: tr(language, 'Pasto / cultivo vigoroso', 'Healthy green cover'),
        };
        const erosionLabels: Record<'low' | 'moderate' | 'high' | 'unknown', string> = {
          low: tr(language, 'Pendiente baja', 'Low slope'),
          moderate: tr(language, 'Pendiente moderada', 'Moderate slope'),
          high: tr(language, 'Pendiente alta', 'Steep slope'),
          unknown: tr(language, 'Sin dato', 'N/A'),
        };
        const erosionBadgeClass: Record<'low' | 'moderate' | 'high' | 'unknown', string> = {
          low: 'bg-[#dcfce7] text-[#14532d]',
          moderate: 'bg-[#fef3c7] text-[#92400e]',
          high: 'bg-[#fee2e2] text-[#991b1b]',
          unknown: 'bg-[#f1f3f0] text-[#475569]',
        };
        const ndviOk = hc !== null && hc.latest_ndvi !== null && hc.latest_ndvi !== undefined;
        const ndviValue = ndviOk && hc?.latest_ndvi != null ? hc.latest_ndvi : 0;
        const ndviPct = Math.max(0, Math.min(100, Math.round(ndviValue * 100)));

        return (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3 space-y-1">
              <div className="flex items-center justify-between gap-2">
                <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813]">
                  <CloudRain className="h-3.5 w-3.5 text-[#1b4332]" />
                  {tr(language, 'Evapotranspiración (ET)', 'Evapotranspiration (ET)')}
                </span>
                <span className="font-mono text-xs font-bold text-[#111813] tabular-nums">
                  {diagnosis.annual_et_mm} mm
                </span>
              </div>
              <p className="text-xs text-[#475569]">
                {tr(
                  language,
                  `Lluvia ${diagnosis.annual_precip_mm} mm · Balance ${diagnosis.annual_water_balance_mm > 0 ? '+' : ''}${diagnosis.annual_water_balance_mm} mm (agua que evapora y consume el cultivo).`,
                  `Rain ${diagnosis.annual_precip_mm} mm · Balance ${diagnosis.annual_water_balance_mm > 0 ? '+' : ''}${diagnosis.annual_water_balance_mm} mm.`,
                )}
              </p>
            </div>

            <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3 space-y-1">
              <div className="flex items-center justify-between gap-2">
                <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813]">
                  <Sprout className="h-3.5 w-3.5 text-[#15803d]" />
                  {tr(language, 'Verdor Sentinel-2 (NDVI)', 'Sentinel-2 NDVI')}
                </span>
                <span className="font-mono text-xs font-bold text-[#111813] tabular-nums">
                  {ndviOk ? `${ndviPct}% (${ndviValue.toFixed(2)})` : '—'}
                </span>
              </div>
              <p className="text-xs text-[#475569]">
                {ndviOk && hc
                  ? `${vegetationLabels[hc.vegetation_status] || tr(language, 'Vigor medio', 'Moderate')} · ${hc.latest_ndvi_date ?? ''}`
                  : tr(language, 'Sin toma despejada reciente.', 'Recent cloudy pass.')}
              </p>
            </div>

            <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3 space-y-1">
              <div className="flex items-center justify-between gap-2">
                <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813]">
                  <Mountain className="h-3.5 w-3.5 text-[#1b4332]" />
                  {tr(language, 'Relieve SRTM', 'SRTM Terrain')}
                </span>
                <span
                  className={`rounded-full px-2 py-0.5 text-[11px] font-semibold ${
                    erosionBadgeClass[diagnosis.terrain.erosion_risk]
                  }`}
                >
                  {erosionLabels[diagnosis.terrain.erosion_risk]}
                </span>
              </div>
              <p className="font-mono text-xs text-[#111813] tabular-nums">
                {diagnosis.terrain.live_terrain_data
                  ? `${diagnosis.terrain.elevation_m} m · ${diagnosis.terrain.slope_pct}% ${tr(language, 'pendiente', 'slope')}`
                  : tr(language, 'Relieve sin dato puntual', 'Elevation N/A')}
              </p>
            </div>
          </div>
        );
      })()}

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
  focusedParcel,
  strategies,
}: DiagnosisSummaryCardProps) {
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
          {tr(language, 'Diagnóstico agrícola de la parcela', 'Field agricultural diagnosis')}
        </h2>
        <p className="mt-1.5 text-sm text-[#475569]">
          {tr(
            language,
            'Elige una parcela arriba o toca el mapa para ver el resumen de agua, humedad y la alternativa recomendada.',
            'Choose a field above or click the map to view water, soil moisture, and the top suggested alternative.',
          )}
        </p>
      </section>
    );
  }

  const monthly = diagnosis.monthly ?? [];
  const optimalMonthsCount = monthly.filter(
    (m) => !m.frost_risk && !m.heat_stress && !m.dry_stress,
  ).length;
  const topMoistPct = Math.round(diagnosis.avg_soil_moisture_top * 100);
  const rootMoistPct = Math.round(diagnosis.avg_soil_moisture_root * 100);

  const nonCurrent = strategies.filter((s) => !s.is_current);
  const topStrategy =
    (nonCurrent.length ? [...nonCurrent].sort((a, b) => b.fit_score - a.fit_score)[0] : strategies[0]) ??
    null;
  const topCoveredCount = topStrategy
    ? topStrategy.monthly_calendar.filter((c) => c.status === 'covered').length
    : 0;

  return (
    <section
      aria-label={tr(language, 'Diagnóstico resumido de la parcela', 'Compact field diagnosis summary')}
      className="flex h-full flex-col justify-between gap-3.5 rounded-xl border border-[#e3e6e1] bg-white p-4 sm:p-5"
    >
      {/* 1. Compact Header */}
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 className="truncate text-base font-bold leading-snug text-[#111813] sm:text-lg">
            {focusedParcel ? focusedParcel.name : tr(language, 'Punto seleccionado en mapa', 'Selected map location')}
          </h2>
          <p className="mt-0.5 text-xs text-[#475569]">
            {tr(language, 'Resumen agrícola con datos NASA POWER y SMAP.', 'Summary based on NASA POWER & SMAP data.')}
          </p>
        </div>

        <span className="shrink-0 rounded-md bg-[#dcfce7] px-2.5 py-1 text-xs font-semibold text-[#14532d] tabular-nums">
          {optimalMonthsCount}/12 {tr(language, 'meses favorables', 'favorable mo')}
        </span>
      </div>

      {/* 2. Top Suggested Alternative Only */}
      {topStrategy && (
        <div className="rounded-lg border border-[#1b4332]/35 bg-[#f4fbf6] p-3.5 space-y-2">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1 rounded bg-[#1b4332] px-2 py-0.5 text-xs font-semibold text-white">
              <Award className="h-3.5 w-3.5 shrink-0" />
              {tr(language, 'Alternativa con mayor adecuación según el modelo', 'Highest-fit alternative (model)')}
            </span>
            <span className="rounded-full bg-[#dcfce7] px-2.5 py-0.5 font-mono text-xs font-bold text-[#14532d] tabular-nums">
              {Math.round(topStrategy.fit_score)}/100
            </span>
          </div>

          <h3 className="text-sm font-bold text-[#111813] sm:text-base">{topStrategy.name}</h3>

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

          <p className="text-xs leading-relaxed text-[#334155] sm:text-sm">
            {viewMode === 'farmer'
              ? topStrategy.why_suggested_farmer
              : topStrategy.why_suggested_agronomist}
          </p>

          <p className="text-xs font-medium text-[#14532d] tabular-nums">
            {topCoveredCount}/12 {tr(language, 'meses con cobertura · Ver detalle abajo', 'mo covered · Full comparison below')}
          </p>
        </div>
      )}

      {/* 3. Two Essential Indicators Only */}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-1 xl:grid-cols-2">
        {/* Indicator 1: Water & Climate Risk */}
        <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3.5 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813]">
              <CloudRain className="h-4 w-4 text-[#2563eb]" />
              {tr(language, 'Riesgo de agua y clima', 'Water & climate risk')}
            </span>
            <span
              className={`rounded-full px-2 py-0.5 text-[11px] font-semibold ${
                diagnosis.annual_water_balance_mm >= 0
                  ? 'bg-[#dcfce7] text-[#14532d]'
                  : 'bg-[#fef3c7] text-[#92400e]'
              }`}
            >
              {diagnosis.annual_water_balance_mm >= 0
                ? tr(language, 'Superávit', 'Surplus')
                : tr(language, 'Déficit', 'Deficit')}
            </span>
          </div>

          <div className="flex items-baseline gap-2">
            <strong className="font-mono text-lg font-bold text-[#111813] tabular-nums">
              {diagnosis.annual_precip_mm} mm
            </strong>
            <span className="font-mono text-xs text-[#475569] tabular-nums">
              {tr(language, 'Bal:', 'Bal:')} {diagnosis.annual_water_balance_mm > 0 ? '+' : ''}
              {diagnosis.annual_water_balance_mm} mm
            </span>
          </div>

          <div className="flex flex-wrap gap-1.5 text-xs font-medium tabular-nums">
            <span
              className={`rounded px-2 py-0.5 ${
                diagnosis.dry_months.length >= 6
                  ? 'bg-[#fef3c7] text-[#92400e]'
                  : 'bg-[#f1f3f0] text-[#334155]'
              }`}
            >
              {diagnosis.dry_months.length}m {tr(language, 'secos', 'dry')}
            </span>
            <span
              className={`inline-flex items-center gap-1 rounded px-2 py-0.5 ${
                diagnosis.frost_risk_months.length > 0
                  ? 'bg-[#dbeafe] text-[#1e3a8a]'
                  : 'bg-[#dcfce7] text-[#14532d]'
              }`}
            >
              <Snowflake className="h-3 w-3" />
              {diagnosis.frost_risk_months.length}m
            </span>
            <span
              className={`inline-flex items-center gap-1 rounded px-2 py-0.5 ${
                diagnosis.heat_stress_months.length > 0
                  ? 'bg-[#ffedd5] text-[#9a3412]'
                  : 'bg-[#dcfce7] text-[#14532d]'
              }`}
            >
              <Sun className="h-3 w-3" />
              {diagnosis.heat_stress_months.length}m
            </span>
          </div>
        </div>

        {/* Indicator 2: Soil Moisture (SMAP) */}
        <div className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-3.5 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813]">
              <Droplets className="h-4 w-4 text-[#0284c7]" />
              {tr(language, 'Humedad suelo (SMAP)', 'Soil moisture (SMAP)')}
            </span>
            <span
              className={`rounded-full px-2 py-0.5 text-[11px] font-semibold ${
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
              {tr(language, 'en raíz', 'root zone')}
            </span>
          </strong>

          <div className="space-y-1 text-xs text-[#334155]">
            <div className="flex justify-between font-mono tabular-nums">
              <span>{tr(language, 'Raíz (0–100 cm)', 'Root (0–100 cm)')}</span>
              <span className="font-semibold text-[#111813]">{rootMoistPct}%</span>
            </div>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
              <div className="h-full rounded-full bg-[#1b4332]" style={{ width: `${rootMoistPct}%` }} />
            </div>
            <div className="flex justify-between font-mono tabular-nums">
              <span>{tr(language, 'Superficie (0–5 cm)', 'Top (0–5 cm)')}</span>
              <span className="font-semibold text-[#111813]">{topMoistPct}%</span>
            </div>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
              <div className="h-full rounded-full bg-[#0284c7]" style={{ width: `${topMoistPct}%` }} />
            </div>
          </div>
        </div>
      </div>
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
      {/* 3 Clean Tabs — Rotations first */}
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
              `Comparar rotaciones (${strategies.length})`,
              `Compare rotations (${strategies.length})`,
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
          <span>{tr(language, 'Pronóstico y calendario', 'Forecast & calendar')}</span>
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
          <span>{tr(language, 'Datos técnicos y gráficos', 'Technical data & charts')}</span>
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
