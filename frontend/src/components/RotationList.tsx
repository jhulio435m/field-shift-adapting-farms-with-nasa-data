import { useState } from 'react';
import {
  AlertCircle,
  ArrowRight,
  Award,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  Droplets,
  ShieldCheck,
} from 'lucide-react';
import { tr } from '../i18n';
import type { Language, RotationStrategy, ViewMode } from '../types';

interface RotationListProps {
  language: Language;
  viewMode: ViewMode;
  strategies: RotationStrategy[];
  baselineStrategies?: RotationStrategy[];
  rainDeltaPct?: number;
  tempDeltaC?: number;
  reminder?: string;
}

const calendarStatusStyles: Record<string, string> = {
  covered: 'bg-[#dcfce7] text-[#14532d] border border-[#bbf7d0]',
  bare_risk: 'bg-[#fee2e2] text-[#7f1d1d] border border-[#fecaca]',
  frost_risk: 'bg-[#dbeafe] text-[#1e3a8a] border border-[#bfdbfe]',
  water_stress: 'bg-[#fef3c7] text-[#78350f] border border-[#fde68a]',
};

export function RotationList({
  language,
  viewMode,
  strategies,
  baselineStrategies,
  rainDeltaPct = 0,
  tempDeltaC = 0,
  reminder,
}: RotationListProps) {
  const [expandedDetails, setExpandedDetails] = useState<Record<string, boolean>>({});

  const waterLabels: Record<'low' | 'medium' | 'high', { text: string; drops: number; cls: string }> = {
    low: {
      text: tr(language, 'Demanda baja', 'Low demand'),
      drops: 1,
      cls: 'bg-[#dcfce7] text-[#14532d]',
    },
    medium: {
      text: tr(language, 'Demanda media', 'Medium demand'),
      drops: 2,
      cls: 'bg-[#dbeafe] text-[#1e3a8a]',
    },
    high: {
      text: tr(language, 'Demanda alta', 'High demand'),
      drops: 3,
      cls: 'bg-[#fef3c7] text-[#92400e]',
    },
  };

  const currentStrategy = strategies.find((s) => s.is_current) ?? null;
  const nonCurrent = strategies.filter((s) => !s.is_current);
  const topRecommended = nonCurrent.length
    ? [...nonCurrent].sort((a, b) => b.fit_score - a.fit_score)[0]
    : null;
  const topRecommendedId = topRecommended?.id ?? null;

  const baseTop = baselineStrategies
    ? baselineStrategies.find((s) => s.id === topRecommendedId) ??
      baselineStrategies.filter((s) => !s.is_current).sort((a, b) => b.fit_score - a.fit_score)[0] ??
      null
    : null;

  const scenarioActive = rainDeltaPct !== 0 || tempDeltaC !== 0;

  const curCovered = currentStrategy
    ? currentStrategy.monthly_calendar.filter((c) => c.status === 'covered').length
    : 0;
  const topCovered = topRecommended
    ? topRecommended.monthly_calendar.filter((c) => c.status === 'covered').length
    : 0;
  const coverDiff = topCovered - curCovered;

  const curStressMonths = currentStrategy
    ? currentStrategy.monthly_calendar.filter((c) => c.status !== 'covered').length
    : 0;
  const topStressMonths = topRecommended
    ? topRecommended.monthly_calendar.filter((c) => c.status !== 'covered').length
    : 0;

  const toggleDetails = (id: string) => {
    setExpandedDetails((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[#e3e6e1] pb-3">
        <div>
          <h2 className="text-base font-bold text-[#111813] sm:text-lg">
            {tr(
              language,
              `Comparador de rotaciones (${strategies.length} opciones evaluadas)`,
              `Rotation comparator (${strategies.length} options evaluated)`,
            )}
          </h2>
          <p className="mt-0.5 text-xs text-[#475569]">
            {tr(
              language,
              'Compara tu práctica actual frente a las alternativas adaptadas al balance hídrico y clima NASA.',
              'Compare your current practice against alternatives adapted to NASA water balance and climate.',
            )}
          </p>
        </div>

        <div className="flex flex-wrap gap-3 text-xs font-medium text-[#334155]">
          <span className="inline-flex items-center gap-1.5">
            <span className="h-3 w-3 rounded-xs bg-[#86efac]" />
            {tr(language, 'Suelo cubierto', 'Covered')}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <span className="h-3 w-3 rounded-xs bg-[#fde68a]" />
            {tr(language, 'Falta agua', 'Dry stress')}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <span className="h-3 w-3 rounded-xs bg-[#bfdbfe]" />
            {tr(language, 'Helada', 'Frost')}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <span className="h-3 w-3 rounded-xs bg-[#fecaca]" />
            {tr(language, 'Suelo desnudo', 'Bare soil')}
          </span>
        </div>
      </div>

      {/* Consistent Impact Comparison Table: Current vs Model Top Alternative */}
      {currentStrategy && topRecommended && (
        <div className="overflow-hidden rounded-xl border border-[#d6dad3] bg-[#fafaf8]">
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#e3e6e1] bg-white px-4 py-2.5">
            <span className="text-sm font-bold text-[#111813]">
              {tr(
                language,
                'Comparación directa de impacto en tu parcela',
                'Direct field impact comparison',
              )}
            </span>
            {scenarioActive && (
              <span className="rounded-full bg-[#fef3c7] px-2.5 py-0.5 font-mono text-xs font-semibold text-[#92400e] tabular-nums">
                {tr(
                  language,
                  `Escenario simulado: lluvia ${rainDeltaPct > 0 ? `+${rainDeltaPct}` : rainDeltaPct}% · +${tempDeltaC.toFixed(1)} °C`,
                  `Simulated scenario: rain ${rainDeltaPct > 0 ? `+${rainDeltaPct}` : rainDeltaPct}% · +${tempDeltaC.toFixed(1)} °C`,
                )}
              </span>
            )}
          </div>

          <div className="overflow-x-auto">
            <table className="w-full min-w-[580px] border-collapse text-left text-xs sm:text-sm">
              <thead>
                <tr className="border-b border-[#e3e6e1] bg-[#f4f6f3] text-[#334155]">
                  <th className="w-[28%] px-4 py-2.5 font-bold">
                    {tr(language, 'Indicador comparado', 'Compared indicator')}
                  </th>
                  <th className="w-[36%] px-4 py-2.5 font-bold text-[#78350f]">
                    {tr(language, 'Rotación actual:', 'Current practice:')} {currentStrategy.name}
                  </th>
                  <th className="w-[36%] bg-[#eef8f0] px-4 py-2.5 font-bold text-[#14532d]">
                    {tr(language, 'Alternativa con mayor adecuación:', 'Highest-fit alternative:')}{' '}
                    {topRecommended.name}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#e3e6e1] bg-white">
                <tr>
                  <td className="px-4 py-2.5 font-semibold text-[#111813]">
                    {tr(language, 'Demanda relativa de agua', 'Relative water demand')}
                  </td>
                  <td className="px-4 py-2.5 text-[#334155]">
                    <span
                      className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-semibold ${
                        waterLabels[currentStrategy.water_demand_level].cls
                      }`}
                    >
                      <Droplets className="h-3.5 w-3.5" />
                      {waterLabels[currentStrategy.water_demand_level].text}
                    </span>
                  </td>
                  <td className="bg-[#f7fcf8] px-4 py-2.5 text-[#14532d]">
                    <span
                      className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-semibold ${
                        waterLabels[topRecommended.water_demand_level].cls
                      }`}
                    >
                      <Droplets className="h-3.5 w-3.5" />
                      {waterLabels[topRecommended.water_demand_level].text}
                    </span>
                  </td>
                </tr>

                <tr>
                  <td className="px-4 py-2.5 font-semibold text-[#111813]">
                    {tr(language, 'Meses con cobertura del suelo', 'Months with soil cover')}
                  </td>
                  <td className="px-4 py-2.5 font-mono text-xs font-semibold text-[#78350f] tabular-nums sm:text-sm">
                    {curCovered} / 12 {tr(language, 'meses', 'months')}{' '}
                    <span className="font-sans text-xs font-normal text-[#475569]">
                      ({curStressMonths} {tr(language, 'en riesgo', 'at risk')})
                    </span>
                  </td>
                  <td className="bg-[#f7fcf8] px-4 py-2.5 font-mono text-xs font-bold text-[#14532d] tabular-nums sm:text-sm">
                    {topCovered} / 12 {tr(language, 'meses', 'months')}
                    {coverDiff > 0 && (
                      <span className="ml-2 inline-flex rounded-full bg-[#dcfce7] px-2 py-0.5 font-sans text-xs font-semibold text-[#14532d]">
                        +{coverDiff} {tr(language, 'meses cubierto', 'more mo')}
                      </span>
                    )}
                    {topStressMonths > 0 && (
                      <span className="ml-1 font-sans text-xs font-normal text-[#475569]">
                        ({topStressMonths} {tr(language, 'con restricción climática', 'climate-limited')})
                      </span>
                    )}
                  </td>
                </tr>

                <tr>
                  <td className="px-4 py-2.5 font-semibold text-[#111813]">
                    {tr(language, 'Adecuación al escenario (modelo)', 'Model scenario fit')}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-[#475569] sm:text-sm">
                    <span className="font-mono font-semibold text-[#111813] tabular-nums">
                      {Math.round(currentStrategy.fit_score)}/100
                    </span>{' '}
                    · {tr(language, 'Monocultivo expuesto al clima actual', 'Single-crop baseline')}
                  </td>
                  <td className="bg-[#f7fcf8] px-4 py-2.5 text-xs sm:text-sm">
                    <span className="rounded-full bg-[#1b4332] px-2.5 py-0.5 font-mono text-xs font-bold text-white tabular-nums">
                      {Math.round(topRecommended.fit_score)}/100
                    </span>
                    {scenarioActive && baseTop && (
                      <span className="ml-2 font-mono text-xs font-semibold text-[#334155] tabular-nums">
                        ({tr(language, 'Normal:', 'Normal:')} {Math.round(baseTop.fit_score)}/100 →{' '}
                        {topRecommended.fit_score - baseTop.fit_score > 0 ? '+' : ''}
                        {Math.round(topRecommended.fit_score - baseTop.fit_score)})
                      </span>
                    )}
                  </td>
                </tr>

                <tr>
                  <td className="px-4 py-2.5 font-semibold text-[#111813]">
                    {tr(language, 'Riesgos por verificar en campo', 'Field risks to verify')}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-[#475569]">
                    {currentStrategy.local_review_needed[0] || currentStrategy.information_gaps[0]}
                  </td>
                  <td className="bg-[#f7fcf8] px-4 py-2.5 text-xs text-[#1e293b]">
                    <span className="inline-flex items-start gap-1.5">
                      <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-[#b45309]" />
                      <span>
                        {topRecommended.local_review_needed[0] || topRecommended.information_gaps[0]}
                      </span>
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <p className="border-t border-[#e3e6e1] bg-[#fafaf8] px-4 py-2 text-xs text-[#475569]">
            {tr(
              language,
              'El puntaje (0–100) mide compatibilidad agroclimática según reglas de balance hídrico, heladas, pendiente y prioridad seleccionada. No estima ahorro monetario ni rendimiento en toneladas; valida semilla, fechas y suelo con tu agrónomo local.',
              'The score (0–100) estimates agro-climatic compatibility from water balance, frost, slope, and field priority rules. It does not claim financial savings or harvest tonnage; confirm seed, timing, and soil locally.',
            )}
          </p>
        </div>
      )}

      {/* Detailed Option Cards */}
      <div className="grid grid-cols-1 gap-4">
        {strategies.map((s) => {
          const why = viewMode === 'farmer' ? s.why_suggested_farmer : s.why_suggested_agronomist;
          const isTopRecommended = !s.is_current && s.id === topRecommendedId;
          const coveredMonths = s.monthly_calendar.filter((c) => c.status === 'covered').length;
          const waterMeta = waterLabels[s.water_demand_level];
          const isOpen = viewMode === 'agronomist' || Boolean(expandedDetails[s.id]);

          return (
            <article
              key={s.id}
              className={`rounded-xl border p-4 sm:p-5 ${
                isTopRecommended
                  ? 'border-[#1b4332] bg-[#f4fbf6] ring-1 ring-[#1b4332]/20'
                  : s.is_current
                    ? 'border-[#e3e6e1] bg-[#fafaf8]'
                    : 'border-[#e3e6e1] bg-white'
              }`}
            >
              {/* Top status strip */}
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex flex-wrap items-center gap-2">
                  {s.is_current ? (
                    <span className="inline-flex items-center rounded-md border border-[#fde68a] bg-[#fef3c7] px-2.5 py-0.5 text-xs font-semibold text-[#92400e]">
                      {tr(language, 'Situación actual', 'Current practice')}
                    </span>
                  ) : isTopRecommended ? (
                    <span className="inline-flex items-center gap-1.5 rounded-md bg-[#1b4332] px-2.5 py-1 text-xs font-semibold text-white">
                      <Award className="h-3.5 w-3.5" />
                      {tr(
                        language,
                        'Alternativa con mayor adecuación según el modelo',
                        'Highest-fit alternative in model',
                      )}
                    </span>
                  ) : (
                    <span className="inline-flex items-center rounded-md border border-[#d6dad3] bg-[#fafaf8] px-2.5 py-0.5 text-xs font-semibold text-[#1b4332]">
                      {tr(language, 'Otra alternativa viable', 'Viable alternative')}
                    </span>
                  )}

                  <h3 className="text-sm font-bold text-[#111813] sm:text-base">{s.name}</h3>
                </div>

                {/* Quick metrics */}
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-semibold ${waterMeta.cls}`}
                  >
                    <Droplets className="h-3.5 w-3.5" />
                    <span>{waterMeta.text}</span>
                  </span>

                  <span className="inline-flex items-center gap-1 rounded-full bg-[#e6f4ea] px-2.5 py-0.5 text-xs font-semibold text-[#14532d] tabular-nums">
                    <ShieldCheck className="h-3.5 w-3.5" />
                    {coveredMonths}/12 {tr(language, 'meses cubierto', 'mo covered')}
                  </span>

                  {!s.is_current && (
                    <span className="rounded-full bg-[#1b4332] px-2.5 py-0.5 font-mono text-xs font-bold text-white tabular-nums">
                      {Math.round(s.fit_score)}/100
                    </span>
                  )}
                </div>
              </div>

              {/* Fit bar */}
              {!s.is_current && (
                <div className="mt-2.5 h-2 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
                  <div
                    className={`h-full rounded-full ${
                      isTopRecommended ? 'bg-[#1b4332]' : 'bg-[#40916c]'
                    }`}
                    style={{ width: `${Math.min(100, Math.max(0, s.fit_score))}%` }}
                  />
                </div>
              )}

              {/* Visual sequence */}
              <div className="mt-3 flex flex-wrap items-center gap-1.5">
                {s.sequence.map((step, idx) => (
                  <div key={`${step}-${idx}`} className="flex items-center gap-1.5">
                    <span className="rounded-md border border-[#d6dad3] bg-white px-2.5 py-1 text-xs font-semibold text-[#111813] sm:text-sm">
                      {step}
                    </span>
                    {idx < s.sequence.length - 1 && (
                      <ArrowRight className="h-3.5 w-3.5 shrink-0 text-[#475569]" />
                    )}
                  </div>
                ))}
              </div>

              <p className="mt-2.5 text-xs leading-relaxed text-[#334155] sm:text-sm">{why}</p>

              {/* Top 2 benefit chips */}
              {s.benefits.length > 0 && (
                <div className="mt-2.5 flex flex-wrap gap-2">
                  {s.benefits.slice(0, 2).map((b, idx) => (
                    <span
                      key={idx}
                      className="inline-flex items-center gap-1 rounded-md border border-[#d6dad3] bg-white px-2.5 py-1 text-xs font-medium text-[#14532d]"
                    >
                      <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-[#1b4332]" />
                      <span>{b}</span>
                    </span>
                  ))}
                </div>
              )}

              {/* 12-month horizontal calendar bar */}
              <div className="mt-3 grid grid-cols-6 gap-1 sm:grid-cols-12">
                {s.monthly_calendar.map((c) => (
                  <div
                    key={c.month}
                    className={`rounded px-1 py-1.5 text-center ${
                      calendarStatusStyles[c.status] || 'bg-slate-100 text-slate-800'
                    }`}
                    title={`${c.month_label}: ${c.crop_or_cover}`}
                  >
                    <span className="block font-mono text-[11px] font-bold leading-tight">
                      {c.month_label}
                    </span>
                    <span className="mt-0.5 block truncate text-[10.5px] font-medium leading-tight">
                      {c.crop_or_cover}
                    </span>
                  </div>
                ))}
              </div>

              {/* Technical details & local checks */}
              <div className="mt-3 border-t border-[#e3e6e1] pt-2.5">
                {viewMode === 'farmer' && (
                  <button
                    type="button"
                    aria-expanded={isOpen}
                    onClick={() => toggleDetails(s.id)}
                    className="inline-flex items-center gap-1 text-xs font-semibold text-[#1b4332] hover:underline cursor-pointer"
                  >
                    <span>
                      {isOpen
                        ? tr(language, 'Ocultar evidencia y revisiones locales', 'Hide evidence & field checks')
                        : tr(language, 'Ver evidencia NASA y qué validar en campo', 'View NASA evidence & field checks')}
                    </span>
                    {isOpen ? (
                      <ChevronUp className="h-3.5 w-3.5" />
                    ) : (
                      <ChevronDown className="h-3.5 w-3.5" />
                    )}
                  </button>
                )}

                {isOpen && (
                  <div className="mt-2.5 grid grid-cols-1 gap-3 text-xs md:grid-cols-3">
                    <div>
                      <span className="font-semibold text-[#111813]">
                        {tr(language, 'Evidencia satelital y climática', 'Satellite & climate evidence')}:
                      </span>
                      <ul className="mt-1 list-disc space-y-1 pl-4 text-[#475569]">
                        {s.evidence.map((e, idx) => (
                          <li key={idx}>{e}</li>
                        ))}
                      </ul>
                    </div>
                    <div>
                      <span className="font-semibold text-[#111813]">
                        {tr(language, 'Beneficios agronómicos', 'Agronomic benefits')}:
                      </span>
                      <ul className="mt-1 list-disc space-y-1 pl-4 text-[#475569]">
                        {s.benefits.map((b, idx) => (
                          <li key={idx}>{b}</li>
                        ))}
                      </ul>
                    </div>
                    <div>
                      <span className="font-semibold text-[#111813]">
                        {tr(language, 'Pendiente de validar en campo', 'To verify locally')}:
                      </span>
                      <ul className="mt-1 list-disc space-y-1 pl-4 text-[#475569]">
                        {s.information_gaps.concat(s.local_review_needed).map((g, idx) => (
                          <li key={idx}>{g}</li>
                        ))}
                      </ul>
                    </div>
                  </div>
                )}
              </div>
            </article>
          );
        })}
      </div>

      <p className="mt-4 border-t border-[#e3e6e1] pt-3 text-xs text-[#475569]">
        {reminder ||
          tr(
            language,
            'Esta guía usa climatología y terreno NASA como apoyo; no reemplaza la experiencia del productor ni la validación agronómica local.',
            'This guide uses NASA climatology and terrain as support; it does not replace farmer judgment or local agronomic validation.',
          )}
      </p>
    </div>
  );
}
