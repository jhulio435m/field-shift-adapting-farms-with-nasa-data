import { useCallback, useEffect, useRef, useState } from 'react';
import {
  AlertTriangle,
  Bell,
  LogOut,
  Microscope,
  RefreshCw,
  Smartphone,
  Sprout,
  UserCheck,
} from 'lucide-react';
import {
  analyzeParcel,
  createParcel,
  deleteParcel,
  fetchProfile,
  logoutSession,
  updateBotConfig,
  updateParcel,
} from './api';
import { DiagnosisExplorer, DiagnosisSummaryCard } from './components/DiagnosisPanel';
import { KusiSimulator } from './components/KusiSimulator';
import { OnboardingForm } from './components/OnboardingForm';
import { ParcelMap } from './components/ParcelMap';
import { ParcelPanel } from './components/ParcelPanel';
import { Button } from './components/ui/button';
import { Input } from './components/ui/input';
import { Label } from './components/ui/label';
import { getVerifiedSnapshot, saveVerifiedSnapshot } from './contingencySnapshots';
import { tr } from './i18n';
import type {
  AlertFrequencyType,
  BotChannelType,
  BotConfig,
  FarmerProfile,
  Language,
  Parcel,
  RotationCompareResponse,
  ViewMode,
} from './types';

const selectClass =
  'flex h-9 w-full rounded-lg border border-[#e3e6e1] bg-white px-2.5 py-1.5 text-xs text-[#111813] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#1b4332]';

export default function App() {
  const [language, setLanguage] = useState<Language>(() =>
    localStorage.getItem('fieldShiftLanguage') === 'en' ? 'en' : 'es',
  );
  const [viewMode, setViewMode] = useState<ViewMode>(() =>
    localStorage.getItem('fieldShiftViewMode') === 'agronomist' ? 'agronomist' : 'farmer',
  );
  const [profile, setProfile] = useState<FarmerProfile | null>(null);
  const [checkingSession, setCheckingSession] = useState(true);

  const [selectedLat, setSelectedLat] = useState(-12.065);
  const [selectedLon, setSelectedLon] = useState(-75.2049);
  const [focusZoom, setFocusZoom] = useState(false);

  const [focusedParcelId, setFocusedParcelId] = useState<string | null>(null);
  const [parcelStatus, setParcelStatus] = useState('');
  const [newFormTrigger, setNewFormTrigger] = useState(0);

  const [rainDeltaPct, setRainDeltaPct] = useState(0);
  const [tempDeltaC, setTempDeltaC] = useState(0);

  const [analysisLoading, setAnalysisLoading] = useState(false);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [baselineData, setBaselineData] = useState<RotationCompareResponse | null>(null);
  const [lastData, setLastData] = useState<RotationCompareResponse | null>(null);
  const [isContingencySnapshot, setIsContingencySnapshot] = useState(false);
  const [snapshotTimestamp, setSnapshotTimestamp] = useState<string | null>(null);

  const [showBotSettings, setShowBotSettings] = useState(false);
  const [botDraft, setBotDraft] = useState<BotConfig>({
    preferred_channel: 'whatsapp',
    telegram_handle: '',
    bot_language: 'es',
    bot_audience: 'farmer',
    alert_frequency: 'immediate',
    alerts_enabled: true,
  });
  const [savingBot, setSavingBot] = useState(false);
  const [botStatus, setBotStatus] = useState('');

  const runIdRef = useRef(0);
  const analysisCacheRef = useRef<Map<string, RotationCompareResponse>>(new Map());

  useEffect(() => {
    localStorage.setItem('fieldShiftLanguage', language);
    document.documentElement.lang = language;
    document.title = tr(
      language,
      'Field Shift — Decisiones Agrícolas con Datos NASA',
      'Field Shift — Climate-Aware Crop Decisions with NASA Data',
    );
  }, [language]);

  useEffect(() => {
    localStorage.setItem('fieldShiftViewMode', viewMode);
  }, [viewMode]);

  const loadProfile = useCallback(async () => {
    try {
      const data = await fetchProfile();
      setProfile(data);
      if (data) {
        if (data.bot_config) {
          setBotDraft(data.bot_config);
        }
        if (data.parcels.length > 0) {
          const first = data.parcels[0];
          setFocusedParcelId(first.id);
          setSelectedLat(first.lat);
          setSelectedLon(first.lon);
          setFocusZoom(true);
        } else {
          setFocusedParcelId(null);
        }
      }
    } finally {
      setCheckingSession(false);
    }
  }, []);

  useEffect(() => {
    void loadProfile();
  }, [loadProfile]);

  const runAnalysis = useCallback(
    async (
      parcelId: string | null,
      rain: number,
      temp: number,
      lang: Language,
      forceRefresh = false,
    ) => {
      if (!profile || !parcelId) return;
      const run = ++runIdRef.current;
      const parcelObj = profile.parcels.find((p) => p.id === parcelId) ?? null;

      async function resolveScenario(r: number, t: number): Promise<{
        data: RotationCompareResponse;
        contingency: boolean;
        savedAt: string | null;
      }> {
        const cacheKey = `${parcelId}:${r}:${t}:${lang}`;
        const cached = !forceRefresh ? analysisCacheRef.current.get(cacheKey) : undefined;
        if (cached) {
          return { data: cached, contingency: false, savedAt: null };
        }
        try {
          const liveResult = await analyzeParcel(
            parcelId!,
            { rain_delta_pct: r, temp_delta_c: t, language: lang },
            profile!.csrf,
          );
          analysisCacheRef.current.set(cacheKey, liveResult);
          if (parcelObj) {
            saveVerifiedSnapshot(parcelObj, r, t, lang, liveResult);
          }
          return { data: liveResult, contingency: false, savedAt: null };
        } catch (err) {
          const fallback = getVerifiedSnapshot(parcelObj, r, t, lang);
          if (fallback) {
            return { data: fallback.data, contingency: true, savedAt: fallback.savedAt };
          }
          throw err;
        }
      }

      const activeCached = !forceRefresh
        ? analysisCacheRef.current.get(`${parcelId}:${rain}:${temp}:${lang}`)
        : undefined;
      const baseCached = !forceRefresh
        ? analysisCacheRef.current.get(`${parcelId}:0:0:${lang}`)
        : undefined;

      if (activeCached && (rain === 0 && temp === 0 ? true : baseCached !== undefined)) {
        setLastData(activeCached);
        setBaselineData(rain === 0 && temp === 0 ? activeCached : (baseCached ?? null));
        setAnalysisError(null);
        setAnalysisLoading(false);
        setIsContingencySnapshot(false);
        setSnapshotTimestamp(null);
        return;
      }

      setAnalysisLoading(true);
      setAnalysisError(null);

      try {
        if (rain === 0 && temp === 0) {
          const res = await resolveScenario(0, 0);
          if (run !== runIdRef.current) return;
          setLastData(res.data);
          setBaselineData(res.data);
          setIsContingencySnapshot(res.contingency);
          setSnapshotTimestamp(res.savedAt);
        } else {
          const [activeRes, baseRes] = await Promise.all([
            resolveScenario(rain, temp),
            resolveScenario(0, 0).catch(() => null),
          ]);
          if (run !== runIdRef.current) return;
          setLastData(activeRes.data);
          setBaselineData(baseRes?.data ?? null);
          setIsContingencySnapshot(activeRes.contingency || Boolean(baseRes?.contingency));
          setSnapshotTimestamp(activeRes.savedAt ?? baseRes?.savedAt ?? null);
        }
      } catch (error) {
        if (run !== runIdRef.current) return;
        const message = error instanceof Error ? error.message : String(error);
        setLastData(null);
        setBaselineData(null);
        setIsContingencySnapshot(false);
        setSnapshotTimestamp(null);
        setAnalysisError(message);
      } finally {
        if (run === runIdRef.current) {
          setAnalysisLoading(false);
        }
      }
    },
    [profile],
  );

  useEffect(() => {
    if (!profile || !focusedParcelId) {
      setLastData(null);
      setBaselineData(null);
      setAnalysisError(null);
      setIsContingencySnapshot(false);
      setSnapshotTimestamp(null);
      return;
    }
    const timer = window.setTimeout(() => {
      void runAnalysis(focusedParcelId, rainDeltaPct, tempDeltaC, language);
    }, 180);
    return () => window.clearTimeout(timer);
  }, [profile, focusedParcelId, rainDeltaPct, tempDeltaC, language, runAnalysis]);

  function handleSelectCoords(lat: number, lon: number, zoom = false) {
    setFocusZoom(zoom);
    setSelectedLat(lat);
    setSelectedLon(lon);
    setParcelStatus(
      tr(
        language,
        'Punto marcado. Haz clic en "+ Nuevo campo" para guardarlo.',
        'Pin dropped. Click "+ New field" to save it.',
      ),
    );
  }

  function handleFocusParcel(id: string) {
    if (!profile) return;
    const parcel = profile.parcels.find((p) => p.id === id);
    if (!parcel) return;

    runIdRef.current++;
    setFocusZoom(true);
    setSelectedLat(parcel.lat);
    setSelectedLon(parcel.lon);
    setFocusedParcelId(id);
    setParcelStatus('');
  }

  function handleChangeScenario(rain: number, temp: number) {
    setRainDeltaPct(rain);
    setTempDeltaC(temp);
  }

  function invalidateParcelCache(parcelId: string) {
    for (const key of analysisCacheRef.current.keys()) {
      if (key.startsWith(`${parcelId}:`)) {
        analysisCacheRef.current.delete(key);
      }
    }
  }

  async function handleSaveParcel(payload: Omit<Parcel, 'id'>) {
    if (!profile) return;
    try {
      const created = await createParcel(payload, profile.csrf, language);
      setProfile((prev) => (prev ? { ...prev, parcels: [...prev.parcels, created] } : prev));
      setFocusedParcelId(created.id);
      setSelectedLat(created.lat);
      setSelectedLon(created.lon);
      setFocusZoom(true);
      setParcelStatus(`${tr(language, 'Guardado:', 'Saved:')} ${created.name}`);
    } catch (error) {
      setParcelStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleUpdateParcel(id: string, payload: Omit<Parcel, 'id'>) {
    if (!profile) return;
    try {
      const updated = await updateParcel(id, payload, profile.csrf, language);
      invalidateParcelCache(id);
      setProfile((prev) =>
        prev
          ? {
              ...prev,
              parcels: prev.parcels.map((p) => (p.id === id ? updated : p)),
            }
          : prev,
      );
      setFocusedParcelId(updated.id);
      setSelectedLat(updated.lat);
      setSelectedLon(updated.lon);
      setFocusZoom(true);
      setParcelStatus(`${tr(language, 'Actualizado:', 'Updated:')} ${updated.name}`);
      void runAnalysis(updated.id, rainDeltaPct, tempDeltaC, language, true);
    } catch (error) {
      setParcelStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleDeleteParcel(id: string) {
    if (!profile) return;
    try {
      await deleteParcel(id, profile.csrf, language);
      invalidateParcelCache(id);
      const remaining = profile.parcels.filter((p) => p.id !== id);
      setProfile((prev) => (prev ? { ...prev, parcels: remaining } : prev));

      let nextFocused = focusedParcelId === id ? null : focusedParcelId;
      if (!nextFocused && remaining.length > 0) {
        nextFocused = remaining[0].id;
      }
      const nextFocusParcel = remaining.find((p) => p.id === nextFocused);
      if (nextFocusParcel) {
        setSelectedLat(nextFocusParcel.lat);
        setSelectedLon(nextFocusParcel.lon);
        setFocusZoom(true);
      }
      setFocusedParcelId(nextFocused);
      setParcelStatus(tr(language, 'Campo eliminado.', 'Field deleted.'));
    } catch (error) {
      setParcelStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleSaveBotSettings(e: React.FormEvent) {
    e.preventDefault();
    if (!profile) return;
    setSavingBot(true);
    setBotStatus('');
    try {
      const updated = await updateBotConfig(botDraft, profile.csrf, language);
      setProfile((prev) => (prev ? { ...prev, bot_config: updated } : prev));
      setBotStatus(tr(language, 'Preferencias guardadas.', 'Preferences saved.'));
    } catch (error) {
      setBotStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setSavingBot(false);
    }
  }

  async function handleLogout() {
    try {
      await logoutSession(language);
    } finally {
      runIdRef.current++;
      analysisCacheRef.current.clear();
      setProfile(null);
      setFocusedParcelId(null);
      setLastData(null);
      setBaselineData(null);
      setIsContingencySnapshot(false);
      setSnapshotTimestamp(null);
      setAnalysisError(null);
      setParcelStatus('');
      setShowBotSettings(false);
      setBotStatus('');
    }
  }

  const focusedParcel = profile?.parcels.find((p) => p.id === focusedParcelId) ?? null;

  const channelBadgeText = profile
    ? profile.bot_config.preferred_channel === 'both'
      ? 'WhatsApp & Telegram'
      : profile.bot_config.preferred_channel === 'telegram'
        ? 'Telegram'
        : 'WhatsApp'
    : '';

  const formattedSnapshotDate = snapshotTimestamp
    ? new Date(snapshotTimestamp).toLocaleString(language === 'en' ? 'en-US' : 'es-PE', {
        dateStyle: 'medium',
        timeStyle: 'short',
      })
    : null;

  return (
    <div className="min-h-screen bg-[#fafaf8] text-[#111813]">
      <header className="sticky top-0 z-30 border-b border-[#e3e6e1] bg-white/95 backdrop-blur-sm">
        <div className="mx-auto flex max-w-[1680px] flex-wrap items-center justify-between gap-3 px-4 py-2.5 sm:px-6">
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded-md bg-[#1b4332] text-white">
                <Sprout className="h-4.5 w-4.5" />
              </div>
              <span className="text-base font-bold tracking-tight text-[#111813]">Field Shift</span>
            </div>
            <span className="hidden text-xs font-medium text-[#475569] md:inline">
              {tr(
                language,
                'Centro de Decisiones Agrícolas con Datos NASA y Asistente de Voz',
                'NASA Agro-Climate Decision Center & Voice Assistant',
              )}
            </span>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {profile && (
              <div
                className="inline-flex rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-0.5"
                role="group"
                aria-label={tr(language, 'Nivel de lectura', 'Reading view mode')}
              >
                <button
                  type="button"
                  onClick={() => setViewMode('farmer')}
                  aria-pressed={viewMode === 'farmer'}
                  className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-semibold transition-colors cursor-pointer ${
                    viewMode === 'farmer'
                      ? 'bg-[#1b4332] text-white'
                      : 'text-[#334155] hover:text-[#111813]'
                  }`}
                >
                  <UserCheck className="h-3.5 w-3.5" />
                  <span>{tr(language, 'Vista Agricultor', 'Farmer')}</span>
                </button>
                <button
                  type="button"
                  onClick={() => setViewMode('agronomist')}
                  aria-pressed={viewMode === 'agronomist'}
                  className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-semibold transition-colors cursor-pointer ${
                    viewMode === 'agronomist'
                      ? 'bg-[#1b4332] text-white'
                      : 'text-[#334155] hover:text-[#111813]'
                  }`}
                >
                  <Microscope className="h-3.5 w-3.5" />
                  <span>{tr(language, 'Vista Técnica', 'Technical')}</span>
                </button>
              </div>
            )}

            {profile && (
              <button
                type="button"
                onClick={() => {
                  setBotStatus('');
                  setShowBotSettings((prev) => !prev);
                }}
                aria-expanded={showBotSettings}
                className={`inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-semibold transition-colors cursor-pointer ${
                  showBotSettings
                    ? 'border-[#1b4332] bg-[#e6f4ea] text-[#143326]'
                    : 'border-[#e3e6e1] bg-[#fafaf8] text-[#111813] hover:bg-[#f0f2ee]'
                }`}
              >
                <Bell className="h-3.5 w-3.5 text-[#1b4332]" />
                <span>
                  {tr(language, 'Canal:', 'Channel:')} {channelBadgeText}
                </span>
              </button>
            )}

            <select
              id="languageSelect"
              aria-label="Idioma / Language"
              value={language}
              onChange={(e) => setLanguage(e.target.value === 'en' ? 'en' : 'es')}
              className="h-8 rounded-lg border border-[#e3e6e1] bg-white px-2 text-xs font-semibold text-[#111813] outline-none cursor-pointer"
            >
              <option value="es">ES</option>
              <option value="en">EN</option>
            </select>

            {profile && (
              <button
                type="button"
                onClick={() => void handleLogout()}
                className="inline-flex h-8 items-center gap-1.5 rounded-lg border border-[#e3e6e1] bg-white px-2.5 text-xs font-semibold text-[#111813] hover:bg-[#fafaf8] cursor-pointer"
                title={tr(language, 'Reiniciar demo o cambiar cuenta', 'Reset demo or switch profile')}
              >
                <LogOut className="h-3.5 w-3.5 text-[#475569]" />
                <span className="hidden sm:inline">{tr(language, 'Salir', 'Exit')}</span>
              </button>
            )}
          </div>
        </div>

        {profile && showBotSettings && (
          <div className="border-t border-[#e3e6e1] bg-[#fafaf8]">
            <form
              onSubmit={handleSaveBotSettings}
              className="mx-auto flex max-w-[1680px] flex-wrap items-end gap-3 px-4 py-3 sm:px-6"
            >
              <div className="w-40">
                <Label htmlFor="cfgChannel" className="text-xs">
                  {tr(language, 'Canal Kusi Kusi', 'Kusi Kusi channel')}
                </Label>
                <select
                  id="cfgChannel"
                  className={selectClass}
                  value={botDraft.preferred_channel}
                  onChange={(e) =>
                    setBotDraft((prev) => ({
                      ...prev,
                      preferred_channel: e.target.value as BotChannelType,
                    }))
                  }
                >
                  <option value="whatsapp">WhatsApp</option>
                  <option value="telegram">Telegram</option>
                  <option value="both">{tr(language, 'Ambos', 'Both')}</option>
                </select>
              </div>

              {(botDraft.preferred_channel === 'telegram' ||
                botDraft.preferred_channel === 'both') && (
                <div className="w-44">
                  <Label htmlFor="cfgTelegram" className="text-xs">
                    {tr(language, 'Usuario Telegram', 'Telegram handle')}
                  </Label>
                  <Input
                    id="cfgTelegram"
                    type="text"
                    className="h-9 text-xs"
                    value={botDraft.telegram_handle}
                    onChange={(e) =>
                      setBotDraft((prev) => ({ ...prev, telegram_handle: e.target.value }))
                    }
                    placeholder="@usuario"
                  />
                </div>
              )}

              <div className="w-44">
                <Label htmlFor="cfgFrequency" className="text-xs">
                  {tr(language, 'Frecuencia alertas', 'Alert frequency')}
                </Label>
                <select
                  id="cfgFrequency"
                  className={selectClass}
                  value={botDraft.alert_frequency}
                  onChange={(e) =>
                    setBotDraft((prev) => ({
                      ...prev,
                      alert_frequency: e.target.value as AlertFrequencyType,
                    }))
                  }
                >
                  <option value="immediate">{tr(language, 'Inmediatas', 'Immediate')}</option>
                  <option value="weekly">{tr(language, 'Semanal', 'Weekly')}</option>
                  <option value="on_demand">{tr(language, 'Bajo consulta', 'On demand')}</option>
                </select>
              </div>

              <label
                htmlFor="cfgEnabled"
                className="flex h-9 cursor-pointer items-center gap-2 rounded-lg border border-[#e3e6e1] bg-white px-3 text-xs font-medium text-[#111813]"
              >
                <input
                  id="cfgEnabled"
                  type="checkbox"
                  className="h-3.5 w-3.5 accent-[#1b4332]"
                  checked={botDraft.alerts_enabled}
                  onChange={(e) =>
                    setBotDraft((prev) => ({ ...prev, alerts_enabled: e.target.checked }))
                  }
                />
                <span>{tr(language, 'Alertas activas', 'Alerts enabled')}</span>
              </label>

              <Button type="submit" size="sm" disabled={savingBot}>
                {savingBot
                  ? tr(language, 'Guardando...', 'Saving...')
                  : tr(language, 'Guardar canal', 'Save channel')}
              </Button>

              {botStatus && (
                <span className="text-xs font-medium text-[#1b4332]" role="status">
                  {botStatus}
                </span>
              )}
            </form>
          </div>
        )}
      </header>

      <main className="mx-auto max-w-[1680px] space-y-4 px-4 py-4 sm:px-6">
        {!checkingSession && !profile && (
          <OnboardingForm language={language} onVerified={loadProfile} />
        )}

        {profile && (
          <>
            {/* 3-Moment Decision Journey Strip + Transparent Data Source Legend */}
            <div className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-[#e3e6e1] bg-white px-4 py-2.5 text-xs">
              <div className="flex flex-wrap items-center gap-x-6 gap-y-1.5">
                <span className="inline-flex items-center gap-2 font-semibold text-[#111813]">
                  <span className="flex h-5 w-5 items-center justify-center rounded-full bg-[#1b4332] font-mono text-xs font-bold text-white">
                    1
                  </span>
                  {tr(language, 'Entender mi campo', 'Understand my field')}
                </span>
                <span className="inline-flex items-center gap-2 font-semibold text-[#111813]">
                  <span className="flex h-5 w-5 items-center justify-center rounded-full bg-[#1b4332] font-mono text-xs font-bold text-white">
                    2
                  </span>
                  {tr(language, 'Comparar opciones', 'Compare rotations')}
                </span>
                <span className="inline-flex items-center gap-2 font-semibold text-[#111813]">
                  <span className="flex h-5 w-5 items-center justify-center rounded-full bg-[#1b4332] font-mono text-xs font-bold text-white">
                    3
                  </span>
                  {tr(language, 'Comprender qué hacer', 'Understand what to do')}
                </span>
              </div>
              <span className="font-mono text-xs text-[#475569]">
                {tr(
                  language,
                  'NASA POWER 30a · Serie 2005–2024 · Sentinel-2 NDVI · Pronóstico 7d',
                  'NASA POWER 30y · 2005–2024 Series · Sentinel-2 NDVI · 7d Forecast',
                )}
              </span>
            </div>

            {/* Verified Contingency Snapshot Notice if external NASA POWER failed */}
            {isContingencySnapshot && (
              <div
                role="status"
                className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-[#fde68a] bg-[#fffbeb] px-4 py-3 text-xs text-[#78350f] sm:text-sm"
              >
                <div className="flex items-start gap-2">
                  <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-[#b45309]" />
                  <span>
                    <strong>{tr(language, 'Modo Contingencia:', 'Contingency Mode:')}</strong>{' '}
                    {tr(
                      language,
                      `El endpoint en vivo de NASA POWER no respondió temporalmente; se muestra una captura histórica verificada guardada${formattedSnapshotDate ? ` (${formattedSnapshotDate})` : ''}.`,
                      `Live NASA POWER endpoint timed out; displaying a verified historical capture saved${formattedSnapshotDate ? ` (${formattedSnapshotDate})` : ''}.`,
                    )}
                  </span>
                </div>
                <button
                  type="button"
                  onClick={() =>
                    void runAnalysis(focusedParcelId, rainDeltaPct, tempDeltaC, language, true)
                  }
                  className="inline-flex items-center gap-1 rounded-md border border-[#fde68a] bg-white px-2.5 py-1 text-xs font-semibold text-[#92400e] hover:bg-[#fef3c7] cursor-pointer"
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                  <span>{tr(language, 'Reintentar NASA en vivo', 'Retry live NASA')}</span>
                </button>
              </div>
            )}

            {/* Top Field Selector + Side-by-Side Scenario Stress Comparator */}
            <ParcelPanel
              language={language}
              profileName={profile.name}
              phoneLast4={profile.phone_last4}
              parcels={profile.parcels}
              focusedParcelId={focusedParcelId}
              selectedLat={selectedLat}
              selectedLon={selectedLon}
              parcelStatus={parcelStatus}
              rainDeltaPct={rainDeltaPct}
              tempDeltaC={tempDeltaC}
              baselineData={baselineData}
              activeData={lastData}
              forceOpenNewForm={newFormTrigger}
              onSelectCoords={handleSelectCoords}
              onFocusParcel={handleFocusParcel}
              onSaveParcel={handleSaveParcel}
              onUpdateParcel={handleUpdateParcel}
              onDeleteParcel={handleDeleteParcel}
              onRunAnalysis={() =>
                void runAnalysis(focusedParcelId, rainDeltaPct, tempDeltaC, language, true)
              }
              onChangeScenario={handleChangeScenario}
            />

            {/* Main Grid: Decision Center FIRST + Map | Accessible Voice Simulator */}
            <div className="grid grid-cols-1 gap-5 lg:grid-cols-[minmax(0,1fr)_364px] lg:items-start">
              <section id="workspaceCenter" className="min-w-0 space-y-4">
                <div className="grid grid-cols-1 gap-4 xl:grid-cols-[1.08fr_1fr] xl:items-stretch">
                  <DiagnosisSummaryCard
                    language={language}
                    viewMode={viewMode}
                    loading={analysisLoading}
                    error={analysisError}
                    diagnosis={lastData?.diagnosis ?? null}
                    baselineDiagnosis={baselineData?.diagnosis ?? null}
                    focusedParcel={focusedParcel}
                    strategies={lastData?.strategies ?? []}
                    baselineStrategies={baselineData?.strategies ?? []}
                    rainDeltaPct={rainDeltaPct}
                    tempDeltaC={tempDeltaC}
                    onChangeScenario={handleChangeScenario}
                  />

                  <ParcelMap
                    lat={selectedLat}
                    lon={selectedLon}
                    language={language}
                    focusZoom={focusZoom}
                    parcels={profile.parcels}
                    focusedParcel={focusedParcel}
                    diagnosis={lastData?.diagnosis ?? null}
                    onSelectCoords={(lat, lon) => handleSelectCoords(lat, lon, false)}
                    onSelectParcel={handleFocusParcel}
                    onStartNewParcelAtCoords={() => setNewFormTrigger((n) => n + 1)}
                  />
                </div>

                <DiagnosisExplorer
                  language={language}
                  viewMode={viewMode}
                  diagnosis={lastData?.diagnosis ?? null}
                  strategies={lastData?.strategies ?? []}
                  baselineStrategies={baselineData?.strategies ?? []}
                  rainDeltaPct={rainDeltaPct}
                  tempDeltaC={tempDeltaC}
                  reminder={lastData?.human_judgment_reminder}
                />
              </section>

              <aside id="workspaceWhatsapp" className="min-w-0 space-y-2.5 lg:sticky lg:top-16">
                <div className="rounded-xl border border-[#e3e6e1] bg-white px-3.5 py-2.5">
                  <div className="flex items-center gap-2">
                    <Smartphone className="h-4 w-4 shrink-0 text-[#1b4332]" />
                    <strong className="text-xs font-bold text-[#111813] sm:text-sm">
                      {tr(
                        language,
                        'Simulador WhatsApp · Canal de accesibilidad Kusi Kusi',
                        'WhatsApp Simulator · Kusi Kusi accessibility view',
                      )}
                    </strong>
                  </div>
                  <p className="mt-1 text-xs text-[#475569]">
                    {tr(
                      language,
                      'Simulación local en navegador (Web Speech API) alimentada por los datos NASA calculados para esta parcela.',
                      'Local browser simulation (Web Speech API) driven by the NASA analysis computed for this field.',
                    )}
                  </p>
                </div>

                <KusiSimulator
                  language={language}
                  viewMode={viewMode}
                  diagnosis={lastData?.diagnosis ?? null}
                  loading={analysisLoading}
                  strategies={lastData?.strategies ?? []}
                  parcel={focusedParcel}
                  botConfig={profile.bot_config}
                  rainDeltaPct={rainDeltaPct}
                  tempDeltaC={tempDeltaC}
                />
              </aside>
            </div>
          </>
        )}
      </main>
    </div>
  );
}
