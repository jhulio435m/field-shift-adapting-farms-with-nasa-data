import { useState } from 'react';
import { isValidPhoneNumber } from 'libphonenumber-js';
import { ArrowRight, CheckCircle2, Compass, Play } from 'lucide-react';
import { claimOnboarding, startDemoOnboarding, startOnboarding } from '../api';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { PhoneInput } from './ui/phone-input';
import { tr } from '../i18n';
import type { AlertFrequencyType, BotChannelType, Language, ViewMode } from '../types';

interface OnboardingFormProps {
  language: Language;
  onVerified: () => Promise<void>;
}

const selectClass =
  'flex h-10 w-full rounded-lg border border-[#e3e6e1] bg-white px-3 py-2 text-sm text-[#111813] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#1b4332] focus-visible:ring-offset-1';

export function OnboardingForm({ language, onVerified }: OnboardingFormProps) {
  const [name, setName] = useState('');
  const [preferredChannel, setPreferredChannel] = useState<BotChannelType>('whatsapp');
  const [phone, setPhone] = useState('');
  const [telegramHandle, setTelegramHandle] = useState('');
  const [botAudience, setBotAudience] = useState<ViewMode>('farmer');
  const [alertFrequency, setAlertFrequency] = useState<AlertFrequencyType>('immediate');

  const [challenge, setChallenge] = useState<string | null>(null);
  const [code, setCode] = useState('');
  const [status, setStatus] = useState('');
  const [isError, setIsError] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [demoLoading, setDemoLoading] = useState(false);

  const needsWhatsapp = preferredChannel === 'whatsapp' || preferredChannel === 'both';
  const needsTelegram = preferredChannel === 'telegram' || preferredChannel === 'both';

  function fillDemoProfile() {
    setName('María Quispe');
    setPreferredChannel('whatsapp');
    setPhone('+51987654321');
    setTelegramHandle('');
    setBotAudience('farmer');
    setAlertFrequency('immediate');
    setStatus('');
    setIsError(false);
  }

  async function handleQuickDemo(useCustomFields = false) {
    setDemoLoading(true);
    setIsError(false);
    setStatus('');
    try {
      const payload = useCustomFields
        ? {
            name: name.trim() || 'María Quispe',
            phone: needsWhatsapp && isValidPhoneNumber(phone) ? phone : '+51987654321',
            preferred_channel: preferredChannel,
            telegram_handle: needsTelegram ? telegramHandle.trim() : '@maria_mantaro',
            bot_language: language,
            bot_audience: botAudience,
            alert_frequency: alertFrequency,
            alerts_enabled: true,
            seed_parcels: true,
          }
        : {
            bot_language: language,
            seed_parcels: true,
          };
      await startDemoOnboarding(payload, language);
      await onVerified();
    } catch (error) {
      setIsError(true);
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setDemoLoading(false);
    }
  }

  async function handleStart(e: React.FormEvent) {
    e.preventDefault();
    if (needsWhatsapp && !isValidPhoneNumber(phone)) {
      setIsError(true);
      setStatus(
        tr(
          language,
          'Ingresa un número de WhatsApp válido para el país seleccionado.',
          'Enter a valid WhatsApp number for the selected country.',
        ),
      );
      return;
    }
    if (needsTelegram && telegramHandle.trim().length < 2) {
      setIsError(true);
      setStatus(
        tr(
          language,
          'Ingresa tu usuario o ID de Telegram.',
          'Enter your Telegram username or ID.',
        ),
      );
      return;
    }

    setSubmitting(true);
    setIsError(false);
    setStatus('');
    try {
      const result = await startOnboarding(
        {
          name: name.trim(),
          phone: needsWhatsapp ? phone : '',
          preferred_channel: preferredChannel,
          telegram_handle: needsTelegram ? telegramHandle.trim() : '',
          bot_language: language,
          bot_audience: botAudience,
          alert_frequency: alertFrequency,
          alerts_enabled: true,
        },
        language,
      );
      if (result.demo_mode && result.claim_code) {
        await claimOnboarding(result.claim_code, language);
        await onVerified();
        return;
      }
      setChallenge(result.challenge);
      setStatus(
        tr(
          language,
          'Solicitud creada. El código vence en 10 minutos o usa acceso directo de demo.',
          'Request created. The code expires in 10 minutes or use instant demo access.',
        ),
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const isUnconfigured =
        message.includes('no configurada') ||
        message.includes('not configured') ||
        message.includes('503');
      if (isUnconfigured) {
        try {
          await startDemoOnboarding(
            {
              name: name.trim() || 'María Quispe',
              phone: needsWhatsapp && isValidPhoneNumber(phone) ? phone : '+51987654321',
              preferred_channel: preferredChannel,
              telegram_handle: needsTelegram ? telegramHandle.trim() : '@maria_mantaro',
              bot_language: language,
              bot_audience: botAudience,
              alert_frequency: alertFrequency,
              alerts_enabled: true,
              seed_parcels: true,
            },
            language,
          );
          await onVerified();
          return;
        } catch (demoErr) {
          setIsError(true);
          setStatus(demoErr instanceof Error ? demoErr.message : String(demoErr));
        }
      } else {
        setIsError(true);
        setStatus(message);
      }
    } finally {
      setSubmitting(false);
    }
  }

  async function handleClaim(e: React.FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setIsError(false);
    setStatus('');
    try {
      await claimOnboarding(code.trim(), language);
      await onVerified();
    } catch (error) {
      setIsError(true);
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="mx-auto mt-4 w-full max-w-2xl space-y-5">
      <section
        className="rounded-xl border border-[#143326] bg-[#1b4332] p-6 text-white shadow-sm sm:p-7"
        id="hackathonDemoBanner"
      >
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <Compass className="h-5 w-5 text-[#bbf7d0]" />
              <h1 className="text-xl font-bold tracking-tight text-white">
                {tr(
                  language,
                  'Demo Interactiva — NASA Space Apps',
                  'Interactive Demo — NASA Space Apps',
                )}
              </h1>
            </div>
            <p className="max-w-xl text-sm leading-relaxed text-[#dcfce7]">
              {tr(
                language,
                'Explora al instante sin esperar códigos por teléfono. Incluye dos parcelas reales en Perú (Lote Mantaro — Papa y Valle Piura — Arroz) conectadas en vivo a NASA POWER, SRTM, Sentinel-2 NDVI y el simulador de alertas Kusi Kusi.',
                'Explore immediately with no phone verification wait. Includes two Peruvian fields (Mantaro — Potato and Piura Valley — Rice) connected live to NASA POWER, SRTM, Sentinel-2 NDVI, and the Kusi Kusi bot simulator.',
              )}
            </p>
          </div>
        </div>

        <div className="mt-5 flex flex-wrap items-center gap-3">
          <Button
            type="button"
            onClick={() => void handleQuickDemo(false)}
            disabled={demoLoading || submitting}
            className="bg-white text-[#143326] hover:bg-[#f0fdf4] font-bold shadow-xs"
          >
            <Play className="h-4 w-4 fill-current" />
            <span>
              {demoLoading
                ? tr(language, 'Cargando parcelas NASA...', 'Loading NASA fields...')
                : tr(language, 'Entrar a la Demo en 1 Clic', 'Launch 1-Click Live Demo')}
            </span>
          </Button>
          <span className="text-xs text-[#bbf7d0]">
            {tr(
              language,
              'Sin registro previo · Datos satelitales en tiempo real',
              'No signup required · Real-time satellite data',
            )}
          </span>
        </div>
      </section>

      <section
        className="rounded-xl border border-[#e3e6e1] bg-white p-6 sm:p-7"
        id="onboardingCard"
      >
        <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#e3e6e1] pb-4">
          <div>
            <h2 className="text-lg font-bold text-[#111813]">
              {tr(
                language,
                'O personaliza un perfil agrícola',
                'Or customize a farmer profile',
              )}
            </h2>
            <p className="mt-0.5 text-sm text-[#526056]">
              {tr(
                language,
                'Configura tu canal preferido (WhatsApp o Telegram) y nivel de detalle para el simulador.',
                'Configure your preferred channel (WhatsApp or Telegram) and detail level for the simulator.',
              )}
            </p>
          </div>
          <button
            type="button"
            onClick={fillDemoProfile}
            className="rounded-lg border border-[#e3e6e1] bg-[#fafaf8] px-3 py-1.5 text-xs font-semibold text-[#1b4332] hover:bg-[#e6f4ea] cursor-pointer"
          >
            {tr(language, 'Autocompletar ejemplo', 'Fill example')}
          </button>
        </div>

        <form onSubmit={handleStart} className="mt-5 space-y-4">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div>
              <Label htmlFor="farmerName">{tr(language, 'Tu nombre', 'Your name')}</Label>
              <Input
                id="farmerName"
                type="text"
                autoComplete="name"
                maxLength={100}
                required
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={tr(language, 'Nombre y apellido', 'Full name')}
              />
            </div>

            <div>
              <Label htmlFor="preferredChannel">
                {tr(language, 'Canal de contacto', 'Contact channel')}
              </Label>
              <select
                id="preferredChannel"
                className={selectClass}
                value={preferredChannel}
                onChange={(e) => setPreferredChannel(e.target.value as BotChannelType)}
              >
                <option value="whatsapp">WhatsApp</option>
                <option value="telegram">Telegram</option>
                <option value="both">{tr(language, 'WhatsApp y Telegram', 'Both')}</option>
              </select>
            </div>
          </div>

          {needsWhatsapp && (
            <div>
              <Label htmlFor="farmerPhone">
                {tr(language, 'Número de WhatsApp (demo o prueba)', 'WhatsApp number (demo or test)')}
              </Label>
              <PhoneInput
                id="farmerPhone"
                language={language}
                defaultCountry="PE"
                value={phone}
                onChange={setPhone}
                required
              />
            </div>
          )}

          {needsTelegram && (
            <div>
              <Label htmlFor="telegramHandle">
                {tr(language, 'Usuario o ID de Telegram', 'Telegram username or ID')}
              </Label>
              <Input
                id="telegramHandle"
                type="text"
                maxLength={64}
                required={preferredChannel === 'telegram'}
                value={telegramHandle}
                onChange={(e) => setTelegramHandle(e.target.value)}
                placeholder="@usuario"
              />
            </div>
          )}

          <div className="grid grid-cols-1 gap-4 border-t border-[#e3e6e1] pt-4 sm:grid-cols-2">
            <div>
              <Label htmlFor="botAudience">
                {tr(language, 'Nivel de detalle', 'Detail level')}
              </Label>
              <select
                id="botAudience"
                className={selectClass}
                value={botAudience}
                onChange={(e) => setBotAudience(e.target.value as ViewMode)}
              >
                <option value="farmer">
                  {tr(language, 'Agricultor (directo)', 'Farmer (plain)')}
                </option>
                <option value="agronomist">
                  {tr(language, 'Agrónomo (técnico)', 'Agronomist (technical)')}
                </option>
              </select>
            </div>

            <div>
              <Label htmlFor="alertFrequency">
                {tr(language, 'Frecuencia de alertas', 'Alert frequency')}
              </Label>
              <select
                id="alertFrequency"
                className={selectClass}
                value={alertFrequency}
                onChange={(e) => setAlertFrequency(e.target.value as AlertFrequencyType)}
              >
                <option value="immediate">
                  {tr(language, 'Inmediatas (helada / tormenta)', 'Immediate (frost / storm)')}
                </option>
                <option value="weekly">
                  {tr(language, 'Resumen semanal', 'Weekly digest')}
                </option>
                <option value="on_demand">
                  {tr(language, 'Solo bajo consulta', 'On demand')}
                </option>
              </select>
            </div>
          </div>

          <Button type="submit" className="w-full" disabled={submitting || demoLoading}>
            <span>{tr(language, 'Continuar al mapa', 'Continue to map')}</span>
            <ArrowRight className="h-4 w-4" />
          </Button>
        </form>

        {challenge && (
          <form
            onSubmit={handleClaim}
            className="mt-5 space-y-3 border-t border-[#e3e6e1] pt-5"
          >
            <p className="rounded-lg border border-[#bbf7d0] bg-[#f0fdf4] p-3 text-xs leading-relaxed text-[#111813]">
              {tr(
                language,
                'Envía este identificador desde tu WhatsApp vinculado a Kusi Kusi:',
                'Send this identifier from your WhatsApp linked to Kusi Kusi:',
              )}{' '}
              <strong className="font-mono font-semibold">{challenge}</strong>
            </p>
            <div>
              <Label htmlFor="claimCode">
                {tr(language, 'Código recibido por WhatsApp', 'Code received on WhatsApp')}
              </Label>
              <Input
                id="claimCode"
                type="text"
                autoComplete="one-time-code"
                required
                value={code}
                onChange={(e) => setCode(e.target.value)}
                placeholder="ABCD-1234"
              />
            </div>
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
              <Button type="submit" disabled={submitting || demoLoading}>
                <CheckCircle2 className="h-4 w-4" />
                {tr(language, 'Validar código', 'Verify code')}
              </Button>
              <Button
                type="button"
                variant="outline"
                disabled={submitting || demoLoading}
                onClick={() => void handleQuickDemo(true)}
              >
                <Play className="h-4 w-4" />
                {tr(
                  language,
                  'Acceso directo de demostración con estos datos',
                  'Direct demo access with these details',
                )}
              </Button>
            </div>
          </form>
        )}

        {status && (
          <p
            className={`mt-3 text-xs font-medium ${
              isError ? 'text-[#b91c1c]' : 'text-[#526056]'
            }`}
            role="status"
          >
            {status}
          </p>
        )}
      </section>
    </div>
  );
}
