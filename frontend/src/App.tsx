import { useCallback, useEffect, useRef, useState } from 'react';
import { Bell, LogOut, Smartphone, Sprout } from 'lucide-react';
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
import { tr } from './i18n';
import type {
  AlertFrequencyType,
  BotChannelType,
  BotConfig,
  FarmerProfile,
  Language,
  Parcel,
  RotationCompareResponse,
} from './types';

const selectClass =
  'flex h-9 w-full rounded-lg border border-[#e3e6e1] bg-white px-2.5 py-1.5 text-xs text-[#111813] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#1b4332]';

export default function App() {
  const [language, setLanguage] = useState<Language>(() =>
    localStorage.getItem('fieldShiftLanguage') === 'en' ? 'en' : 'es',
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
  const [lastData, setLastData] = useState<RotationCompareResponse | null>(null);

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
      'Field Shift — Monitoreo Satelital para el Campo',
      'Field Shift — Satellite Field Monitoring',
    );
  }, [language]);

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
      const cacheKey = `${parcelId}:${rain}:${temp}:${lang}`;
      const cached = !forceRefresh ? analysisCacheRef.current.get(cacheKey) : undefined;
      if (cached) {
        setLastData(cached);
        setAnalysisError(null);
        setAnalysisLoading(false);
        return;
      }

      setAnalysisLoading(true);
      setAnalysisError(null);

      try {
        const result = await analyzeParcel(
          parcelId,
          { rain_delta_pct: rain, temp_delta_c: temp, language: lang },
          profile.csrf,
        );
        if (run !== runIdRef.current) return;
        analysisCacheRef.current.set(cacheKey, result);
        setLastData(result);
      } catch (error) {
        if (run !== runIdRef.current) return;
        const message = error instanceof Error ? error.message : String(error);
        setLastData(null);
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
      setAnalysisError(null);
      return;
    }
    const timer = window.setTimeout(() => {
      void runAnalysis(focusedParcelId, rainDeltaPct, tempDeltaC, language);
    }, 200);
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

  return (
    <div className="min-h-screen bg-[#fafaf8] text-[#111813]">
      <header className="sticky top-0 z-30 border-b border-[#e3e6e1] bg-white/95 backdrop-blur-sm">
        <div className="mx-auto flex max-w-[1680px] flex-wrap items-center justify-between gap-3 px-4 py-2.5 sm:px-6">
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="flex h-7 w-7 items-center justify-center rounded-md bg-[#1b4332] text-white">
                <Sprout className="h-4 w-4" />
              </div>
              <span className="text-sm font-bold tracking-tight text-[#111813]">Field Shift</span>
            </div>
            <span className="hidden text-xs font-medium text-[#475569] md:inline">
              {tr(
                language,
                'Monitoreo Satelital NASA y Asistente de Voz por WhatsApp',
                'NASA Satellite Monitoring & WhatsApp Voice Assistant',
              )}
            </span>
          </div>

          <div className="flex items-center gap-2">
            {profile && (
              <button
                type="button"
                onClick={() => {
                  setBotStatus('');
                  setShowBotSettings((prev) => !prev);
                }}
                aria-expanded={showBotSettings}
                className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-semibold transition-colors cursor-pointer ${
                  showBotSettings
                    ? 'border-[#1b4332] bg-[#e6f4ea] text-[#143326]'
                    : 'border-[#e3e6e1] bg-[#fafaf8] text-[#111813] hover:bg-[#f0f2ee]'
                }`}
              >
                <Bell className="h-3 w-3 text-[#1b4332]" />
                <span>
                  {tr(language, 'Alertas', 'Alerts')}: {channelBadgeText}
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
                <Label htmlFor="cfgChannel" className="text-[11px]">
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
                  <Label htmlFor="cfgTelegram" className="text-[11px]">
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
                <Label htmlFor="cfgFrequency" className="text-[11px]">
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
            {/* Compact 3-Step Visual Guide */}
            <div className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-[#e3e6e1] bg-white px-4 py-2.5 text-xs">
              <div className="flex flex-wrap items-center gap-x-5 gap-y-1.5">
                <span className="inline-flex items-center gap-2 font-medium text-[#111813]">
                  <span className="flex h-5 w-5 items-center justify-center rounded-full bg-[#1b4332] font-mono text-[11px] font-bold text-white">
                    1
                  </span>
                  {tr(language, 'Elige tu campo en el mapa', 'Pick your field on the map')}
                </span>
                <span className="inline-flex items-center gap-2 font-medium text-[#111813]">
                  <span className="flex h-5 w-5 items-center justify-center rounded-full bg-[#1b4332] font-mono text-[11px] font-bold text-white">
                    2
                  </span>
                  {tr(
                    language,
                    'Revisa el agua, suelo, pasto y qué sembrar',
                    'Check water, soil, pasture & crop advice',
                  )}
                </span>
                <span className="inline-flex items-center gap-2 font-medium text-[#111813]">
                  <span className="flex h-5 w-5 items-center justify-center rounded-full bg-[#1b4332] font-mono text-[11px] font-bold text-white">
                    3
                  </span>
                  {tr(
                    language,
                    'Escucha el reporte de voz en WhatsApp',
                    'Listen to the voice report on WhatsApp',
                  )}
                </span>
              </div>
              <span className="font-mono text-[11px] text-[#475569]">NASA POWER · GIBS · SRTM</span>
            </div>

            {/* Top Field & Weather Selector Bar */}
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
              forceOpenNewForm={newFormTrigger}
              onSelectCoords={handleSelectCoords}
              onFocusParcel={handleFocusParcel}
              onSaveParcel={handleSaveParcel}
              onUpdateParcel={handleUpdateParcel}
              onDeleteParcel={handleDeleteParcel}
              onRunAnalysis={() =>
                void runAnalysis(focusedParcelId, rainDeltaPct, tempDeltaC, language, true)
              }
              onChangeRain={setRainDeltaPct}
              onChangeTemp={setTempDeltaC}
            />

            {/* Main Above-the-Fold Grid: (Map + Vitals) + (Tabs below) | Sticky WhatsApp Simulator */}
            <div className="grid grid-cols-1 gap-5 lg:grid-cols-[minmax(0,1fr)_364px] lg:items-start">
              <section id="workspaceCenter" className="min-w-0 space-y-4">
                <div className="grid grid-cols-1 gap-4 xl:grid-cols-[1.15fr_1fr] xl:items-stretch">
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

                  <DiagnosisSummaryCard
                    language={language}
                    loading={analysisLoading}
                    error={analysisError}
                    diagnosis={lastData?.diagnosis ?? null}
                    focusedParcel={focusedParcel}
                    strategies={lastData?.strategies ?? []}
                  />
                </div>

                <DiagnosisExplorer
                  language={language}
                  diagnosis={lastData?.diagnosis ?? null}
                  strategies={lastData?.strategies ?? []}
                  reminder={lastData?.human_judgment_reminder}
                />
              </section>

              <aside id="workspaceWhatsapp" className="min-w-0 space-y-2.5 lg:sticky lg:top-16">
                <div className="rounded-xl border border-[#e3e6e1] bg-white px-3.5 py-2.5">
                  <div className="flex items-center gap-2">
                    <Smartphone className="h-4 w-4 shrink-0 text-[#1b4332]" />
                    <strong className="text-xs font-bold text-[#111813]">
                      {tr(
                        language,
                        'Simulador WhatsApp · Así le llega al productor',
                        'WhatsApp Simulator · Farmer mobile view',
                      )}
                    </strong>
                  </div>
                  <p className="mt-0.5 text-[11px] text-[#475569]">
                    {tr(
                      language,
                      'Toca Play para oír la nota de voz o haz una pregunta abajo.',
                      'Tap Play to hear the voice report or ask a question below.',
                    )}
                  </p>
                </div>

                <KusiSimulator
                  language={language}
                  viewMode="farmer"
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
