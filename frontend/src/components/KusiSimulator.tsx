import { useEffect, useMemo, useRef, useState } from 'react';
import {
  BatteryFull,
  Camera,
  CheckCheck,
  ChevronLeft,
  Code2,
  FileText,
  Mic,
  Pause,
  Phone,
  Play,
  Plus,
  Send,
  Signal,
  Smile,
  Sprout,
  User,
  Video,
  Wifi,
} from 'lucide-react';
import { fetchKusiSource } from '../api';
import { tr } from '../i18n';
import type { BotConfig, Language, Parcel, ParcelDiagnosis, RotationStrategy, ViewMode } from '../types';

interface KusiSimulatorProps {
  language: Language;
  viewMode: ViewMode;
  diagnosis: ParcelDiagnosis | null;
  loading?: boolean;
  strategies: RotationStrategy[];
  parcel: Parcel | null;
  botConfig: BotConfig;
  rainDeltaPct: number;
  tempDeltaC: number;
}

interface ChatMessage {
  id: string;
  sender: 'farmer' | 'bot';
  kind: 'voice' | 'text';
  text: string;
  durationLabel?: string;
  time: string;
  waveformSeed?: number;
}

const WAVEFORM_BARS_COUNT = 26;

const BASE_BAR_HEIGHTS = [
  5, 9, 13, 7, 16, 11, 6, 15, 18, 10, 7, 14, 17, 9, 12, 6, 16, 13, 8, 15, 11, 7, 14, 10, 8, 5,
];

function getWaveformHeights(seed = 0): number[] {
  return Array.from({ length: WAVEFORM_BARS_COUNT }, (_, idx) => {
    return BASE_BAR_HEIGHTS[(idx + seed * 3) % BASE_BAR_HEIGHTS.length];
  });
}

function formatCurrentTime(): string {
  const now = new Date();
  return now.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
}

function getBestNonCurrentStrategy(strategies: RotationStrategy[]): RotationStrategy | undefined {
  return strategies.find((s) => !s.is_current) ?? strategies[0];
}

function buildFarmerVoicePrompt(language: Language, parcel: Parcel | null): string {
  const parcelName = parcel?.name ?? tr(language, 'mi parcela', 'my field');
  const cropName = parcel?.current_crop ?? tr(language, 'cultivo actual', 'current crop');
  return tr(
    language,
    `Hola Kusi, estoy revisando ${parcelName} (${cropName}). ¿Qué rotación me recomiendas según los datos NASA de lluvia, calor y humedad del suelo?`,
    `Hi Kusi, I am checking ${parcelName} (${cropName}). What rotation do you recommend based on NASA rainfall, heat, and soil moisture data?`
  );
}

function buildBotVoiceGreeting(
  language: Language,
  viewMode: ViewMode,
  diagnosis: ParcelDiagnosis,
  strategies: RotationStrategy[],
  parcel: Parcel | null,
  rainDeltaPct: number,
  tempDeltaC: number
): string {
  const top = getBestNonCurrentStrategy(strategies);
  const parcelTitle = parcel?.name ?? tr(language, 'tu parcela', 'your field');
  const alertText = diagnosis.weekly_alerts?.farmer_alert_message;
  const corr = diagnosis.historical_correlation;
  const soilRootPct = Math.round(diagnosis.avg_soil_moisture_root * 100);
  const scenarioNote =
    rainDeltaPct !== 0 || tempDeltaC !== 0
      ? tr(
          language,
          ` Escenario simulado: lluvia ${rainDeltaPct > 0 ? `+${rainDeltaPct}` : rainDeltaPct}% y temperatura +${tempDeltaC.toFixed(1)} °C.`,
          ` Simulated scenario: rainfall ${rainDeltaPct > 0 ? `+${rainDeltaPct}` : rainDeltaPct}% and temp +${tempDeltaC.toFixed(1)} °C.`
        )
      : '';

  if (viewMode === 'agronomist') {
    return tr(
      language,
      `Resumen técnico NASA para ${parcelTitle}: lluvia ${diagnosis.annual_precip_mm} mm, ET ${diagnosis.annual_et_mm} mm, balance ${diagnosis.annual_water_balance_mm} mm, humedad radicular ${soilRootPct}%, T media ${diagnosis.annual_temp_mean_c} °C.${corr ? ` Correlación lluvia-humedad r=${corr.rain_soil_moisture_corr.toFixed(2)}.` : ''}${top ? ` Opción líder: ${top.name} (${Math.round(top.fit_score)}/100).` : ''}${scenarioNote}`,
      `Technical NASA brief for ${parcelTitle}: rainfall ${diagnosis.annual_precip_mm} mm, ET ${diagnosis.annual_et_mm} mm, balance ${diagnosis.annual_water_balance_mm} mm, root moisture ${soilRootPct}%, mean T ${diagnosis.annual_temp_mean_c} °C.${corr ? ` Rain-moisture corr r=${corr.rain_soil_moisture_corr.toFixed(2)}.` : ''}${top ? ` Top option: ${top.name} (${Math.round(top.fit_score)}/100).` : ''}${scenarioNote}`
    );
  }

  return tr(
    language,
    `Reporte NASA listo para ${parcelTitle}. Lluvia anual ${diagnosis.annual_precip_mm} mm y humedad radicular ${soilRootPct}%.${top ? ` Mejor rotación: ${top.name} (${top.sequence.join(' -> ')}).` : ''}${alertText ? ` Aviso 7 días: ${alertText}` : ''}${scenarioNote}`,
    `NASA report ready for ${parcelTitle}. Annual rainfall ${diagnosis.annual_precip_mm} mm and root soil moisture ${soilRootPct}%.${top ? ` Best rotation: ${top.name} (${top.sequence.join(' -> ')}).` : ''}${alertText ? ` 7-day notice: ${alertText}` : ''}${scenarioNote}`
  );
}

function buildBotTextSummary(
  language: Language,
  diagnosis: ParcelDiagnosis,
  strategies: RotationStrategy[],
  parcel: Parcel | null
): string {
  const top = getBestNonCurrentStrategy(strategies);
  const soilRootPct = Math.round(diagnosis.avg_soil_moisture_root * 100);
  const ndviLabel =
    diagnosis.historical_correlation?.latest_ndvi !== null &&
    diagnosis.historical_correlation?.latest_ndvi !== undefined
      ? ` · NDVI ${diagnosis.historical_correlation.latest_ndvi.toFixed(2)}`
      : '';

  return tr(
    language,
    `${parcel?.name ?? 'Parcela'}: Lluvia ${diagnosis.annual_precip_mm} mm · ET ${diagnosis.annual_et_mm} mm · Temp ${diagnosis.annual_temp_mean_c} °C · Suelo ${soilRootPct}%${ndviLabel}.${top ? `\nSugerencia principal: ${top.name} (${Math.round(top.fit_score)}/100).` : ''}`,
    `${parcel?.name ?? 'Field'}: Rain ${diagnosis.annual_precip_mm} mm · ET ${diagnosis.annual_et_mm} mm · Temp ${diagnosis.annual_temp_mean_c} °C · Soil ${soilRootPct}%${ndviLabel}.${top ? `\nTop suggestion: ${top.name} (${Math.round(top.fit_score)}/100).` : ''}`
  );
}

function buildBotReply(
  language: Language,
  viewMode: ViewMode,
  prompt: string,
  diagnosis: ParcelDiagnosis | null,
  strategies: RotationStrategy[],
  parcel: Parcel | null,
  rainDeltaPct: number,
  tempDeltaC: number
): string {
  const q = prompt.toLowerCase();
  const nonCurrent = strategies.filter((s) => !s.is_current);
  const top = nonCurrent[0] ?? strategies[0];
  const second = nonCurrent[1] ?? strategies[1];
  const alert = diagnosis?.weekly_alerts;
  const corr = diagnosis?.historical_correlation;

  if (!diagnosis) {
    return tr(
      language,
      'Selecciona una parcela y ejecuta el diagnóstico NASA para recibir recomendaciones reales de lluvia, calor, suelo, vegetación y agua.',
      'Select a field and run the NASA diagnosis first to receive tailored rainfall, heat, soil moisture, vegetation, and water advice.'
    );
  }

  const soilTopPct = Math.round(diagnosis.avg_soil_moisture_top * 100);
  const soilRootPct = Math.round(diagnosis.avg_soil_moisture_root * 100);

  if (q.includes('rotac') || q.includes('siembr') || q.includes('cultiv') || q.includes('sow') || q.includes('crop')) {
    if (!top) {
      return tr(
        language,
        'No hay opciones de rotación cargadas para esta parcela todavía.',
        'No rotation options are loaded for this field yet.'
      );
    }
    if (viewMode === 'agronomist') {
      return tr(
        language,
        `1. ${top.name} (${Math.round(top.fit_score)}/100 · ${top.sequence.join(' -> ')}): ${top.why_suggested_agronomist}${second ? ` Alternativa: ${second.name} (${Math.round(second.fit_score)}/100).` : ''}`,
        `1. ${top.name} (${Math.round(top.fit_score)}/100 · ${top.sequence.join(' -> ')}): ${top.why_suggested_agronomist}${second ? ` Runner-up: ${second.name} (${Math.round(second.fit_score)}/100).` : ''}`
      );
    }
    return tr(
      language,
      `Para ${parcel?.name ?? 'tu parcela'} conviene ${top.name} (${top.sequence.join(' -> ')}): ${top.why_suggested_farmer}`,
      `For ${parcel?.name ?? 'your field'}, ${top.name} (${top.sequence.join(' -> ')}) fits well: ${top.why_suggested_farmer}`
    );
  }

  if (
    q.includes('clim') ||
    q.includes('humed') ||
    q.includes('lluv') ||
    q.includes('agua') ||
    q.includes('veget') ||
    q.includes('ndvi') ||
    q.includes('rain') ||
    q.includes('soil') ||
    q.includes('water')
  ) {
    const scenario =
      rainDeltaPct !== 0 || tempDeltaC !== 0
        ? tr(
            language,
            ` Escenario: lluvia ${rainDeltaPct > 0 ? `+${rainDeltaPct}` : rainDeltaPct}%, +${tempDeltaC.toFixed(1)} °C.`,
            ` Scenario: rain ${rainDeltaPct > 0 ? `+${rainDeltaPct}` : rainDeltaPct}%, +${tempDeltaC.toFixed(1)} °C.`
          )
        : '';
    const ndviPart =
      corr?.latest_ndvi !== null && corr?.latest_ndvi !== undefined
        ? `, NDVI ${corr.latest_ndvi.toFixed(2)} (${corr.vegetation_status})`
        : '';
    return tr(
      language,
      `NASA (${diagnosis.data_source}): lluvia ${diagnosis.annual_precip_mm} mm, ET ${diagnosis.annual_et_mm} mm, balance ${diagnosis.annual_water_balance_mm} mm, humedad superficial ${soilTopPct}% / radicular ${soilRootPct}%${ndviPart}.${scenario}`,
      `NASA (${diagnosis.data_source}): rain ${diagnosis.annual_precip_mm} mm, ET ${diagnosis.annual_et_mm} mm, water balance ${diagnosis.annual_water_balance_mm} mm, top moisture ${soilTopPct}% / root ${soilRootPct}%${ndviPart}.${scenario}`
    );
  }

  if (q.includes('riesg') || q.includes('helad') || q.includes('sequ') || q.includes('calor') || q.includes('alert') || q.includes('risk') || q.includes('frost') || q.includes('heat')) {
    const frostNote =
      diagnosis.frost_risk_months.length > 0
        ? tr(language, ` Heladas: ${diagnosis.frost_risk_months.join(', ')}.`, ` Frost months: ${diagnosis.frost_risk_months.join(', ')}.`)
        : '';
    const heatNote =
      diagnosis.heat_stress_months.length > 0
        ? tr(language, ` Calor extremo: ${diagnosis.heat_stress_months.join(', ')}.`, ` Heat stress: ${diagnosis.heat_stress_months.join(', ')}.`)
        : '';
    return tr(
      language,
      `Alerta 7 días (${alert?.alert_level ?? 'normal'}): ${alert?.farmer_alert_message ?? 'Sin evento crítico inmediato.'}${frostNote}${heatNote}`,
      `7-day alert (${alert?.alert_level ?? 'normal'}): ${alert?.farmer_alert_message ?? 'No immediate critical event.'}${frostNote}${heatNote}`
    );
  }

  if (q.includes('agr') || q.includes('valid') || q.includes('revis') || q.includes('hist') || q.includes('técn') || q.includes('tech')) {
    const elev = diagnosis.terrain.elevation_m !== null ? `${Math.round(diagnosis.terrain.elevation_m)} m s.n.m.` : 'N/D';
    const reviewItem = top?.local_review_needed?.[0];
    return tr(
      language,
      `Parcela ${parcel?.name ?? '-'} (${elev}, pendiente ${diagnosis.terrain.slope_pct ?? '-'}%): ${corr ? `${corr.years_analyzed} años NASA, r=${corr.rain_soil_moisture_corr.toFixed(2)}. ` : ''}${viewMode === 'agronomist' ? diagnosis.agronomist_summary : diagnosis.farmer_summary}${reviewItem ? ` Validar en campo: ${reviewItem}` : ''}`,
      `Field ${parcel?.name ?? '-'} (${elev}, slope ${diagnosis.terrain.slope_pct ?? '-'}%): ${corr ? `${corr.years_analyzed} NASA yrs, r=${corr.rain_soil_moisture_corr.toFixed(2)}. ` : ''}${viewMode === 'agronomist' ? diagnosis.agronomist_summary : diagnosis.farmer_summary}${reviewItem ? ` Field check: ${reviewItem}` : ''}`
    );
  }

  return tr(
    language,
    `NASA activo para ${parcel?.name ?? 'tu parcela'}: lluvia ${diagnosis.annual_precip_mm} mm, T ${diagnosis.annual_temp_mean_c} °C, humedad radicular ${soilRootPct}%${top ? `, top: ${top.name} (${Math.round(top.fit_score)}/100)` : ''}. Pregúntame por rotación, humedad/NDVI o riesgos.`,
    `Active NASA telemetry for ${parcel?.name ?? 'your field'}: rain ${diagnosis.annual_precip_mm} mm, T ${diagnosis.annual_temp_mean_c} °C, root moisture ${soilRootPct}%${top ? `, top pick: ${top.name} (${Math.round(top.fit_score)}/100)` : ''}. Ask about rotation, moisture/NDVI, or alerts.`
  );
}

interface VoiceBubbleProps {
  message: ChatMessage;
  language: Language;
  isPlaying: boolean;
  progress: number;
  showTranscript: boolean;
  onTogglePlay: (message: ChatMessage) => void;
  onToggleTranscript: (id: string) => void;
}

function VoiceBubble({
  message,
  language,
  isPlaying,
  progress,
  showTranscript,
  onTogglePlay,
  onToggleTranscript,
}: VoiceBubbleProps) {
  const isFarmer = message.sender === 'farmer';
  const heights = useMemo(() => getWaveformHeights(message.waveformSeed ?? 0), [message.waveformSeed]);
  const clampedProgress = Math.max(0, Math.min(1, progress));
  const activeBars = Math.round(clampedProgress * heights.length);
  const dotLeftPct = Math.round(clampedProgress * 100);

  const avatarEl = (
    <div className="relative shrink-0 self-center">
      <div
        className={`flex h-9 w-9 items-center justify-center rounded-full ${
          isFarmer ? 'bg-[#94a3b8]/45 text-[#1b4332]' : 'bg-[#cbd5e1]/80 text-[#334155]'
        }`}
      >
        {isFarmer ? <User className="h-4.5 w-4.5" /> : <Sprout className="h-4.5 w-4.5 text-[#1b4332]" />}
      </div>
      <span
        className={`absolute bottom-0 flex h-3.5 w-3.5 items-center justify-center rounded-full ${
          isFarmer ? '-right-0.5 bg-[#c8f7c5] text-[#1b4332]' : '-left-0.5 bg-white text-[#475569]'
        }`}
      >
        <Mic className="h-2.5 w-2.5" />
      </span>
    </div>
  );

  return (
    <div className={`flex ${isFarmer ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`w-[86%] rounded-2xl px-2.5 py-2 shadow-[0_1px_1px_rgba(15,23,42,0.08)] ${
          isFarmer
            ? 'rounded-tr-sm bg-[#c8f7c5] text-[#143322]'
            : 'rounded-tl-sm bg-white text-[#1e293b]'
        }`}
      >
        <div className="flex items-center gap-2">
          {isFarmer && avatarEl}

          <button
            type="button"
            onClick={() => onTogglePlay(message)}
            aria-label={
              isPlaying
                ? tr(language, 'Pausar nota de voz', 'Pause voice note')
                : tr(language, 'Reproducir nota de voz', 'Play voice note')
            }
            className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-full transition-colors ${
              isFarmer
                ? 'text-[#1b4332] hover:bg-[#1b4332]/10'
                : 'text-[#334155] hover:bg-slate-100'
            }`}
          >
            {isPlaying ? <Pause className="h-4 w-4 fill-current" /> : <Play className="h-4 w-4 fill-current translate-x-[1px]" />}
          </button>

          <div className="min-w-0 flex-1 pt-1">
            <div className="relative flex h-5 items-center gap-[2px]">
              <span
                className="pointer-events-none absolute top-1/2 z-10 h-2.5 w-2.5 -translate-y-1/2 rounded-full bg-[#0f172a] transition-[left] duration-100"
                style={{ left: `calc(${dotLeftPct}% - 4px)` }}
              />
              {heights.map((barH, idx) => {
                const filled = idx < activeBars;
                return (
                  <span
                    key={idx}
                    className={`w-[2px] flex-1 rounded-full transition-colors ${
                      filled
                        ? isFarmer
                          ? 'bg-[#1b4332]'
                          : 'bg-[#0f172a]'
                        : isFarmer
                          ? 'bg-[#1b4332]/35'
                          : 'bg-[#94a3b8]/65'
                    }`}
                    style={{ height: `${barH}px` }}
                  />
                );
              })}
            </div>

            <div
              className={`mt-1 flex items-center justify-between text-[10px] leading-none tabular-nums ${
                isFarmer ? 'text-[#1b4332]' : 'text-[#475569]'
              }`}
            >
              <span>{message.durationLabel ?? '0:24'}</span>
              <span className="inline-flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => onToggleTranscript(message.id)}
                  title={tr(language, 'Mostrar texto del audio', 'Show audio transcript')}
                  className="inline-flex items-center gap-0.5 underline decoration-dotted underline-offset-2 hover:opacity-80"
                >
                  <FileText className="h-2.5 w-2.5" />
                  <span>{showTranscript ? tr(language, 'Ocultar', 'Hide') : tr(language, 'Texto', 'Text')}</span>
                </button>
                <span>{message.time}</span>
                {isFarmer && <CheckCheck className="h-3 w-3 text-[#0369a1]" />}
              </span>
            </div>
          </div>

          {!isFarmer && avatarEl}
        </div>

        {showTranscript && (
          <p
            className={`mt-2 border-t pt-1.5 text-[11px] leading-relaxed ${
              isFarmer ? 'border-[#1b4332]/15 text-[#143322]' : 'border-slate-200 text-[#334155]'
            }`}
          >
            {message.text}
          </p>
        )}
      </div>
    </div>
  );
}

export function KusiSimulator({
  language,
  viewMode,
  diagnosis,
  loading = false,
  strategies,
  parcel,
  botConfig,
  rainDeltaPct,
  tempDeltaC,
}: KusiSimulatorProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [draft, setDraft] = useState('');
  const [actionsOpen, setActionsOpen] = useState(true);
  const [showJson, setShowJson] = useState(false);
  const [kusiSource, setKusiSource] = useState<Record<string, unknown> | null>(null);
  const [playingId, setPlayingId] = useState<string | null>(null);
  const [playProgress, setPlayProgress] = useState<Record<string, number>>({});
  const [expandedTranscripts, setExpandedTranscripts] = useState<Record<string, boolean>>({});
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const playTimerRef = useRef<number | null>(null);

  const stopAudioPlayback = () => {
    if (playTimerRef.current !== null) {
      window.clearInterval(playTimerRef.current);
      playTimerRef.current = null;
    }
    if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
      window.speechSynthesis.cancel();
    }
    setPlayingId(null);
  };

  useEffect(() => {
    return () => {
      if (playTimerRef.current !== null) {
        window.clearInterval(playTimerRef.current);
      }
      if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
        window.speechSynthesis.cancel();
      }
    };
  }, []);

  useEffect(() => {
    let active = true;
    fetchKusiSource(parcel?.id)
      .then((data) => {
        if (active) setKusiSource(data);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [parcel?.id, diagnosis?.annual_precip_mm, rainDeltaPct, tempDeltaC]);

  useEffect(() => {
    stopAudioPlayback();
    setPlayProgress({});
    setExpandedTranscripts({});

    if (!diagnosis) {
      setMessages([
        {
          id: 'empty-wait',
          sender: 'bot',
          kind: 'text',
          text: tr(
            language,
            'Listo para recibir tu parcela. Haz clic en "Analizar con NASA" para generar tu nota de voz y recomendaciones.',
            'Ready for your field. Run NASA analysis to receive your voice report and rotation recommendations.'
          ),
          time: '10:30 AM',
        },
      ]);
      return;
    }

    setMessages([
      {
        id: 'farmer-init-voice',
        sender: 'farmer',
        kind: 'voice',
        text: buildFarmerVoicePrompt(language, parcel),
        durationLabel: '0:18',
        time: '10:28 AM',
        waveformSeed: 1,
      },
      {
        id: 'bot-init-voice',
        sender: 'bot',
        kind: 'voice',
        text: buildBotVoiceGreeting(
          language,
          viewMode,
          diagnosis,
          strategies,
          parcel,
          rainDeltaPct,
          tempDeltaC
        ),
        durationLabel: '0:42',
        time: '10:29 AM',
        waveformSeed: 3,
      },
      {
        id: 'bot-init-text',
        sender: 'bot',
        kind: 'text',
        text: buildBotTextSummary(language, diagnosis, strategies, parcel),
        time: '10:30 AM',
      },
    ]);
  }, [
    diagnosis,
    language,
    viewMode,
    parcel?.id,
    parcel?.name,
    parcel?.current_crop,
    rainDeltaPct,
    tempDeltaC,
    strategies,
  ]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }, [messages.length]);

  const handleTogglePlay = (message: ChatMessage) => {
    if (playingId === message.id) {
      stopAudioPlayback();
      return;
    }

    stopAudioPlayback();
    setPlayingId(message.id);
    setPlayProgress((prev) => ({ ...prev, [message.id]: 0.04 }));

    if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
      const utterance = new SpeechSynthesisUtterance(message.text);
      utterance.lang = language === 'es' ? 'es-PE' : 'en-US';
      utterance.rate = 1.0;
      window.speechSynthesis.speak(utterance);
    }

    const previewMs = 6000;
    const stepMs = 100;
    let elapsed = 0;

    playTimerRef.current = window.setInterval(() => {
      elapsed += stepMs;
      const ratio = Math.min(1, elapsed / previewMs);
      setPlayProgress((prev) => ({ ...prev, [message.id]: ratio }));
      if (ratio >= 1) {
        stopAudioPlayback();
        setPlayProgress((prev) => ({ ...prev, [message.id]: 1 }));
      }
    }, stepMs);
  };

  const handleToggleTranscript = (id: string) => {
    setExpandedTranscripts((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const handleSendText = (promptText: string) => {
    const clean = promptText.trim();
    if (!clean) return;
    const nowTime = formatCurrentTime();
    const userMsg: ChatMessage = {
      id: `user-${Date.now()}`,
      sender: 'farmer',
      kind: 'text',
      text: clean,
      time: nowTime,
    };
    const replyMsg: ChatMessage = {
      id: `bot-${Date.now() + 1}`,
      sender: 'bot',
      kind: 'text',
      text: buildBotReply(language, viewMode, clean, diagnosis, strategies, parcel, rainDeltaPct, tempDeltaC),
      time: nowTime,
    };
    setMessages((prev) => [...prev, userMsg, replyMsg]);
    setDraft('');
  };

  const handleSendVoiceSimulation = (promptText?: string) => {
    const spokenQuery =
      promptText?.trim() ||
      tr(
        language,
        'Kusi, envíame la recomendación de rotación según el clima actual de mi parcela.',
        'Kusi, send me the rotation advice for my field current climate.'
      );
    const replyText = buildBotReply(
      language,
      viewMode,
      spokenQuery,
      diagnosis,
      strategies,
      parcel,
      rainDeltaPct,
      tempDeltaC
    );
    const nowTime = formatCurrentTime();

    const userVoice: ChatMessage = {
      id: `farmer-voice-${Date.now()}`,
      sender: 'farmer',
      kind: 'voice',
      text: spokenQuery,
      durationLabel: '0:12',
      time: nowTime,
      waveformSeed: (messages.length % 5) + 1,
    };
    const botVoice: ChatMessage = {
      id: `bot-voice-${Date.now() + 1}`,
      sender: 'bot',
      kind: 'voice',
      text: replyText,
      durationLabel: '0:28',
      time: nowTime,
      waveformSeed: (messages.length % 4) + 2,
    };
    setMessages((prev) => [...prev, userVoice, botVoice]);
    handleTogglePlay(botVoice);
  };

  const channelLabel =
    botConfig.preferred_channel === 'telegram'
      ? 'Telegram'
      : botConfig.preferred_channel === 'both'
        ? 'WhatsApp + Telegram'
        : 'WhatsApp';

  const handleCallTrigger = (mode: 'voice' | 'video') => {
    const firstBotVoice = messages.find((m) => m.sender === 'bot' && m.kind === 'voice');
    if (firstBotVoice && mode === 'voice') {
      handleTogglePlay(firstBotVoice);
      return;
    }
    const top = getBestNonCurrentStrategy(strategies);
    const handleText = botConfig.telegram_handle ? ` (${botConfig.telegram_handle})` : '';
    const callSummary: ChatMessage = {
      id: `call-${Date.now()}`,
      sender: 'bot',
      kind: 'text',
      text: diagnosis
        ? tr(
            language,
            `Canal ${channelLabel}${handleText} activo. ${parcel?.name ?? 'Parcela'}: ${diagnosis.annual_precip_mm} mm lluvia · Top: ${top?.name ?? 'N/D'}.`,
            `${channelLabel}${handleText} channel active. ${parcel?.name ?? 'Field'}: ${diagnosis.annual_precip_mm} mm rain · Top: ${top?.name ?? 'N/A'}.`
          )
        : tr(
            language,
            `Kusi Kusi listo en ${channelLabel}. Ejecuta un análisis NASA primero.`,
            `Kusi Kusi ready on ${channelLabel}. Run a NASA analysis first.`
          ),
      time: formatCurrentTime(),
    };
    setMessages((prev) => [...prev, callSummary]);
  };

  const quickActions = [
    {
      id: 'audio-rotation',
      kind: 'voice' as const,
      label: tr(language, 'Audio: ¿Qué rotación sembrar?', 'Audio: What crop rotation?'),
      prompt: tr(
        language,
        'Audio: ¿Qué rotación sembrar en esta campaña según la humedad NASA?',
        'Audio: What crop rotation should I plant this season?'
      ),
    },
    {
      id: 'nasa-moisture',
      kind: 'text' as const,
      label: tr(language, 'Clima, humedad y NDVI', 'Climate, moisture & NDVI'),
      prompt: tr(language, 'Clima, humedad del suelo y vegetación NDVI NASA', 'NASA climate, soil moisture and NDVI vegetation'),
    },
    {
      id: 'risk-alert',
      kind: 'text' as const,
      label: tr(language, 'Riesgo helada / sequía', 'Frost / drought risk'),
      prompt: tr(language, 'Riesgo de helada, calor o sequía 7 días', '7-day frost, heat, or drought risk'),
    },
    {
      id: 'agronomist-check',
      kind: 'text' as const,
      label: tr(language, 'Revisión con agrónomo', 'Agronomist check'),
      prompt: tr(language, 'Revisión técnica histórica con agrónomo', 'Historical technical check with agronomist'),
    },
  ];

  const botTitle = parcel ? `Kusi Kusi · ${parcel.name}` : 'Kusi Kusi · NASA';
  const statusSubtitle = loading
    ? tr(language, 'Calculando NASA...', 'Updating NASA...')
    : tr(language, `En línea · ${channelLabel}`, `Online · ${channelLabel}`);

  return (
    <section aria-label={tr(language, 'Simulador móvil Kusi Kusi', 'Kusi Kusi mobile simulator')} className="space-y-3">
      {/* Realistic iPhone outer chassis */}
      <div className="relative mx-auto w-full max-w-[360px]">
        {/* Decorative physical side buttons */}
        <span className="pointer-events-none absolute -left-[3px] top-24 h-7 w-[3px] rounded-l bg-[#1f2937]" />
        <span className="pointer-events-none absolute -left-[3px] top-36 h-10 w-[3px] rounded-l bg-[#1f2937]" />
        <span className="pointer-events-none absolute -left-[3px] top-48 h-10 w-[3px] rounded-l bg-[#1f2937]" />
        <span className="pointer-events-none absolute -right-[3px] top-32 h-14 w-[3px] rounded-r bg-[#1f2937]" />

        <div className="overflow-hidden rounded-[44px] border-[10px] border-[#111813] bg-white shadow-[0_18px_40px_rgba(15,23,42,0.18)]">
          {/* iOS status bar + Dynamic Island */}
          <div className="flex items-center justify-between bg-white px-5 pb-1.5 pt-2.5 text-[#111813]">
            <span className="text-[11px] font-semibold tracking-tight tabular-nums">10:30</span>
            <div className="h-[20px] w-[88px] rounded-full bg-[#111813]" aria-hidden="true" />
            <div className="flex items-center gap-1.5">
              <Signal className="h-3 w-3" />
              <Wifi className="h-3 w-3" />
              <BatteryFull className="h-3.5 w-3.5" />
            </div>
          </div>

          {/* WhatsApp-style header bar */}
          <div className="flex items-center justify-between border-b border-slate-200/80 bg-white px-2.5 py-2">
            <div className="flex min-w-0 items-center gap-1.5">
              <button
                type="button"
                onClick={() => setActionsOpen((prev) => !prev)}
                aria-label={tr(language, 'Mostrar u ocultar accesos rápidos', 'Toggle quick prompts')}
                className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[#111813] hover:bg-slate-100"
              >
                <ChevronLeft className="h-5 w-5" />
              </button>

              <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-[#1b4332]/12 text-[#1b4332]">
                <Sprout className="h-4.5 w-4.5" />
              </div>

              <div className="min-w-0">
                <p className="truncate text-[13px] font-semibold leading-tight text-[#111813]">{botTitle}</p>
                <p className="truncate text-[11px] font-medium leading-tight text-[#475569]">{statusSubtitle}</p>
              </div>
            </div>

            <div className="flex shrink-0 items-center gap-0.5 text-[#111813]">
              <button
                type="button"
                onClick={() => handleCallTrigger('video')}
                aria-label={tr(language, 'Resumen de canal', 'Channel summary')}
                className="flex h-7 w-7 items-center justify-center rounded-full hover:bg-slate-100"
              >
                <Video className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={() => handleCallTrigger('voice')}
                aria-label={tr(language, 'Reproducir diagnóstico por voz', 'Play voice diagnosis')}
                className="flex h-7 w-7 items-center justify-center rounded-full hover:bg-slate-100"
              >
                <Phone className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={() => setShowJson((prev) => !prev)}
                aria-label={tr(language, 'Ver contrato JSON', 'Toggle JSON payload')}
                aria-pressed={showJson}
                className={`flex h-7 w-7 items-center justify-center rounded-full transition-colors ${
                  showJson ? 'bg-[#1b4332] text-white' : 'hover:bg-slate-100'
                }`}
              >
                <Code2 className="h-4 w-4" />
              </button>
            </div>
          </div>

          {/* Warm WhatsApp chat canvas */}
          <div className="h-[360px] space-y-2.5 overflow-y-auto bg-[#efeae2] px-3 py-3">
            {messages.map((msg) => {
              if (msg.kind === 'voice') {
                return (
                  <VoiceBubble
                    key={msg.id}
                    message={msg}
                    language={language}
                    isPlaying={playingId === msg.id}
                    progress={playProgress[msg.id] ?? 0}
                    showTranscript={Boolean(expandedTranscripts[msg.id])}
                    onTogglePlay={handleTogglePlay}
                    onToggleTranscript={handleToggleTranscript}
                  />
                );
              }

              const isFarmer = msg.sender === 'farmer';
              return (
                <div key={msg.id} className={`flex ${isFarmer ? 'justify-end' : 'justify-start'}`}>
                  <div
                    className={`max-w-[84%] rounded-2xl px-3 py-2 text-[12px] leading-[1.42] shadow-[0_1px_1px_rgba(15,23,42,0.08)] ${
                      isFarmer
                        ? 'rounded-tr-sm bg-[#c8f7c5] text-[#143322]'
                        : 'rounded-tl-sm bg-white text-[#1e293b]'
                    }`}
                  >
                    <p className="whitespace-pre-line">{msg.text}</p>
                    <div
                      className={`mt-1 flex items-center justify-end gap-1 text-[10px] leading-none tabular-nums ${
                        isFarmer ? 'text-[#1b4332]' : 'text-[#475569]'
                      }`}
                    >
                      <span>{msg.time}</span>
                      {isFarmer && <CheckCheck className="h-3 w-3 text-[#0369a1]" />}
                    </div>
                  </div>
                </div>
              );
            })}
            <div ref={messagesEndRef} />
          </div>

          {/* Collapsible quick-prompt chips inside bottom drawer */}
          {actionsOpen && (
            <div className="flex flex-wrap gap-1.5 border-t border-[#e2ddd3] bg-[#f7f5f0] px-2.5 py-2">
              {quickActions.map((action) => (
                <button
                  key={action.id}
                  type="button"
                  onClick={() =>
                    action.kind === 'voice'
                      ? handleSendVoiceSimulation(action.prompt)
                      : handleSendText(action.prompt)
                  }
                  className="rounded-full border border-[#d5cfc2] bg-white px-2.5 py-1 text-[10.5px] font-medium text-[#1b4332] transition-colors hover:bg-[#c8f7c5]/60"
                >
                  {action.label}
                </button>
              ))}
            </div>
          )}

          {/* Authentic WhatsApp bottom composer */}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              handleSendText(draft);
            }}
            className="flex items-center gap-1.5 border-t border-slate-200/80 bg-[#f6f6f6] px-2.5 py-2"
          >
            <button
              type="button"
              onClick={() => setActionsOpen((prev) => !prev)}
              aria-label={tr(language, 'Alternar preguntas rápidas', 'Toggle quick questions')}
              aria-expanded={actionsOpen}
              className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-[#334155] hover:bg-slate-200/70"
            >
              <Plus className="h-5 w-5" />
            </button>

            <div className="flex min-w-0 flex-1 items-center rounded-full border border-slate-300/80 bg-white px-3 py-1">
              <input
                type="text"
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                placeholder={tr(language, 'Mensaje para Kusi Kusi...', 'Message Kusi Kusi...')}
                className="w-full min-w-0 bg-transparent text-[12px] text-[#111813] placeholder:text-[#64748b] focus:outline-none"
              />
              <Smile className="ml-1.5 h-4 w-4 shrink-0 text-[#475569]" />
            </div>

            <button
              type="button"
              onClick={() =>
                handleSendText(
                  tr(language, '¿Qué muestra la humedad del suelo NASA y NDVI aquí?', 'What does NASA soil moisture and NDVI show here?')
                )
              }
              aria-label={tr(language, 'Enviar consulta de parcela', 'Send field snapshot query')}
              className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-[#334155] hover:bg-slate-200/70"
            >
              <Camera className="h-4.5 w-4.5" />
            </button>

            {draft.trim().length > 0 ? (
              <button
                type="submit"
                aria-label={tr(language, 'Enviar mensaje', 'Send message')}
                className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-[#1b4332] text-white hover:bg-[#143322]"
              >
                <Send className="h-4 w-4" />
              </button>
            ) : (
              <button
                type="button"
                onClick={() => handleSendVoiceSimulation()}
                aria-label={tr(language, 'Enviar nota de voz simulada', 'Send simulated voice note')}
                className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-[#1b4332] hover:bg-[#c8f7c5]"
              >
                <Mic className="h-4.5 w-4.5" />
              </button>
            )}
          </form>

          {/* iPhone bottom home indicator */}
          <div className="flex justify-center bg-white pb-2 pt-1.5">
            <span className="h-1 w-28 rounded-full bg-[#111813]" aria-hidden="true" />
          </div>
        </div>
      </div>

      {/* Collapsible Kusi JSON source inspector below phone */}
      {showJson && (
        <div className="mx-auto max-w-[360px] rounded-2xl border border-slate-200 bg-white p-3 shadow-xs">
          <div className="mb-1.5 flex items-center justify-between">
            <span className="text-[11px] font-semibold text-[#111813]">
              /v1/bot/kusi-source
            </span>
            <button
              type="button"
              onClick={() => setShowJson(false)}
              className="text-[11px] font-medium text-[#475569] hover:text-[#111813]"
            >
              {tr(language, 'Cerrar', 'Close')}
            </button>
          </div>
          <pre className="max-h-56 overflow-auto rounded-xl bg-slate-900 p-2.5 text-[10.5px] leading-relaxed text-emerald-200">
            {JSON.stringify(
              kusiSource ?? {
                parcel_id: parcel?.id ?? null,
                preferred_channel: botConfig.preferred_channel,
              },
              null,
              2
            )}
          </pre>
        </div>
      )}
    </section>
  );
}
