import { useState } from 'react';
import {
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
  reminder,
}: RotationListProps) {
  const [expandedDetails, setExpandedDetails] = useState<Record<string, boolean>>({});

  const waterLabels: Record<'low' | 'medium' | 'high', { text: string; drops: number; cls: string }> = {
    low: {
      text: tr(language, 'Poca agua', 'Low water'),
      drops: 1,
      cls: 'bg-[#dcfce7] text-[#14532d]',
    },
    medium: {
      text: tr(language, 'Agua media', 'Medium water'),
      drops: 2,
      cls: 'bg-[#dbeafe] text-[#1e3a8a]',
    },
    high: {
      text: tr(language, 'Mucha agua', 'High water'),
      drops: 3,
      cls: 'bg-[#fef3c7] text-[#92400e]',
    },
  };

  const nonCurrent = strategies.filter((s) => !s.is_current);
  const topRecommendedId = nonCurrent.length
    ? [...nonCurrent].sort((a, b) => b.fit_score - a.fit_score)[0].id
    : null;

  const toggleDetails = (id: string) => {
    setExpandedDetails((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#e3e6e1] pb-3">
        <h2 className="text-base font-bold text-[#111813]">
          {tr(
            language,
            `Qué conviene sembrar (${strategies.length} opciones)`,
            `Best planting & pasture options (${strategies.length})`,
          )}
        </h2>
        <div className="flex flex-wrap gap-3 text-[11px] font-medium text-[#475569]">
          <span className="inline-flex items-center gap-1">
            <span className="h-2.5 w-2.5 rounded-xs bg-[#86efac]" />
            {tr(language, 'Suelo cubierto', 'Covered')}
          </span>
          <span className="inline-flex items-center gap-1">
            <span className="h-2.5 w-2.5 rounded-xs bg-[#fde68a]" />
            {tr(language, 'Falta agua', 'Dry stress')}
          </span>
          <span className="inline-flex items-center gap-1">
            <span className="h-2.5 w-2.5 rounded-xs bg-[#bfdbfe]" />
            {tr(language, 'Helada', 'Frost')}
          </span>
          <span className="inline-flex items-center gap-1">
            <span className="h-2.5 w-2.5 rounded-xs bg-[#fecaca]" />
            {tr(language, 'Suelo desnudo', 'Bare soil')}
          </span>
        </div>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4">
        {strategies.map((s) => {
          const why = viewMode === 'farmer' ? s.why_suggested_farmer : s.why_suggested_agronomist;
          const isTopRecommended = !s.is_current && s.id === topRecommendedId;
          const coveredMonths = s.monthly_calendar.filter((c) => c.status === 'covered').length;
          const waterMeta = waterLabels[s.water_demand_level];
          const isOpen = Boolean(expandedDetails[s.id]);

          return (
            <article
              key={s.id}
              className={`rounded-xl border p-4 ${
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
                    <span className="inline-flex items-center rounded-md border border-[#fde68a] bg-[#fef3c7] px-2 py-0.5 text-[11px] font-semibold text-[#92400e]">
                      {tr(language, 'Lo que usas hoy', 'Current practice')}
                    </span>
                  ) : isTopRecommended ? (
                    <span className="inline-flex items-center gap-1 rounded-md bg-[#1b4332] px-2.5 py-0.5 text-[11px] font-semibold text-white">
                      <Award className="h-3.5 w-3.5" />
                      {tr(language, 'Mejor opción recomendada', 'Top recommended pick')}
                    </span>
                  ) : (
                    <span className="inline-flex items-center rounded-md border border-[#d6dad3] bg-[#fafaf8] px-2 py-0.5 text-[11px] font-semibold text-[#1b4332]">
                      {tr(language, 'Otra alternativa', 'Alternative')}
                    </span>
                  )}

                  <h3 className="text-sm font-bold text-[#111813]">{s.name}</h3>
                </div>

                {/* 3 quick visual metrics */}
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-semibold ${waterMeta.cls}`}
                  >
                    <Droplets className="h-3 w-3" />
                    <span>{waterMeta.text}</span>
                    <span className="font-mono text-[10px]">
                      ({'•'.repeat(waterMeta.drops)})
                    </span>
                  </span>

                  <span className="inline-flex items-center gap-1 rounded-full bg-[#e6f4ea] px-2 py-0.5 text-[11px] font-semibold text-[#14532d] tabular-nums">
                    <ShieldCheck className="h-3 w-3" />
                    {coveredMonths}/12 {tr(language, 'meses cubierto', 'mo covered')}
                  </span>

                  {!s.is_current && (
                    <span className="rounded-full bg-[#1b4332] px-2.5 py-0.5 font-mono text-[11px] font-bold text-white tabular-nums">
                      {Math.round(s.fit_score)}/100
                    </span>
                  )}
                </div>
              </div>

              {/* Fit bar for recommendations */}
              {!s.is_current && (
                <div className="mt-2 h-2 w-full overflow-hidden rounded-full bg-[#e3e6e1]">
                  <div
                    className={`h-full rounded-full ${
                      isTopRecommended ? 'bg-[#1b4332]' : 'bg-[#40916c]'
                    }`}
                    style={{ width: `${Math.min(100, Math.max(0, s.fit_score))}%` }}
                  />
                </div>
              )}

              {/* Visual sequence of crops/forages */}
              <div className="mt-3 flex flex-wrap items-center gap-1.5">
                {s.sequence.map((step, idx) => (
                  <div key={`${step}-${idx}`} className="flex items-center gap-1.5">
                    <span className="rounded-md border border-[#d6dad3] bg-white px-2.5 py-1 text-xs font-semibold text-[#111813]">
                      {step}
                    </span>
                    {idx < s.sequence.length - 1 && (
                      <ArrowRight className="h-3.5 w-3.5 shrink-0 text-[#475569]" />
                    )}
                  </div>
                ))}
              </div>

              <p className="mt-2.5 text-xs leading-relaxed text-[#475569]">{why}</p>

              {/* Top 2 quick benefit chips */}
              {s.benefits.length > 0 && (
                <div className="mt-2.5 flex flex-wrap gap-1.5">
                  {s.benefits.slice(0, 2).map((b, idx) => (
                    <span
                      key={idx}
                      className="inline-flex items-center gap-1 rounded-md border border-[#d6dad3] bg-white px-2 py-1 text-[11px] font-medium text-[#14532d]"
                    >
                      <CheckCircle2 className="h-3 w-3 shrink-0 text-[#1b4332]" />
                      <span>{b}</span>
                    </span>
                  ))}
                </div>
              )}

              {/* Compact 12-month horizontal visual bar */}
              <div className="mt-3 grid grid-cols-6 gap-1 sm:grid-cols-12">
                {s.monthly_calendar.map((c) => (
                  <div
                    key={c.month}
                    className={`rounded px-1 py-1 text-center ${
                      calendarStatusStyles[c.status] || 'bg-slate-100 text-slate-800'
                    }`}
                    title={`${c.month_label}: ${c.crop_or_cover}`}
                  >
                    <span className="block font-mono text-[10px] font-bold leading-tight">
                      {c.month_label}
                    </span>
                    <span className="mt-0.5 block truncate text-[9px] font-medium leading-tight">
                      {c.crop_or_cover}
                    </span>
                  </div>
                ))}
              </div>

              {/* Optional expandable technical details */}
              <div className="mt-3 border-t border-[#e3e6e1] pt-2.5">
                <button
                  type="button"
                  aria-expanded={isOpen}
                  onClick={() => toggleDetails(s.id)}
                  className="inline-flex items-center gap-1 text-[11.5px] font-semibold text-[#1b4332] hover:underline cursor-pointer"
                >
                  <span>
                    {isOpen
                      ? tr(language, 'Ocultar detalle técnico', 'Hide technical details')
                      : tr(language, 'Ver detalle técnico', 'View technical details')}
                  </span>
                  {isOpen ? (
                    <ChevronUp className="h-3.5 w-3.5" />
                  ) : (
                    <ChevronDown className="h-3.5 w-3.5" />
                  )}
                </button>

                {isOpen && (
                  <div className="mt-2.5 grid grid-cols-1 gap-3 text-xs md:grid-cols-3">
                    <div>
                      <span className="font-semibold text-[#111813]">
                        {tr(language, 'Evidencia', 'Evidence')}:
                      </span>
                      <ul className="mt-1 list-disc space-y-0.5 pl-4 text-[#475569]">
                        {s.evidence.map((e, idx) => (
                          <li key={idx}>{e}</li>
                        ))}
                      </ul>
                    </div>
                    <div>
                      <span className="font-semibold text-[#111813]">
                        {tr(language, 'Beneficios', 'Benefits')}:
                      </span>
                      <ul className="mt-1 list-disc space-y-0.5 pl-4 text-[#475569]">
                        {s.benefits.map((b, idx) => (
                          <li key={idx}>{b}</li>
                        ))}
                      </ul>
                    </div>
                    <div>
                      <span className="font-semibold text-[#111813]">
                        {tr(language, 'Revisar en campo', 'Field checks')}:
                      </span>
                      <ul className="mt-1 list-disc space-y-0.5 pl-4 text-[#475569]">
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
            'Esta guía usa datos satelitales NASA como apoyo y se complementa con tu experiencia en campo.',
            'This guide uses NASA satellite data as support alongside your local field judgment.',
          )}
      </p>
    </div>
  );
}
