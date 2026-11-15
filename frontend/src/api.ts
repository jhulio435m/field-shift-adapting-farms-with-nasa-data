import { translateApiError } from './i18n';
import type {
  BotConfig,
  FarmerProfile,
  Language,
  Parcel,
  RotationCompareResponse,
} from './types';

async function requestJson<T>(
  method: 'POST' | 'PUT' | 'DELETE',
  path: string,
  body: unknown,
  language: Language,
  csrfToken?: string,
): Promise<T> {
  const response = await fetch(path, {
    method,
    credentials: 'same-origin',
    headers: {
      'Content-Type': 'application/json',
      ...(csrfToken ? { 'X-CSRF-Token': csrfToken } : {}),
    },
    ...(body !== undefined ? { body: JSON.stringify(body) } : {}),
  });

  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    const detail = typeof data.detail === 'string' ? data.detail : `HTTP ${response.status}`;
    throw new Error(translateApiError(detail, language));
  }
  return data as T;
}

export function startOnboarding(
  payload: { name: string; phone: string } & BotConfig,
  language: Language,
) {
  return requestJson<{ challenge: string; demo_mode?: boolean; claim_code?: string }>(
    'POST',
    '/v1/onboarding/start',
    payload,
    language,
  );
}

export function startDemoOnboarding(
  payload: Partial<{ name: string; phone: string; seed_parcels: boolean } & BotConfig>,
  language: Language,
) {
  return requestJson<{ status: string; csrf: string; demo_mode: boolean }>(
    'POST',
    '/v1/onboarding/demo',
    payload,
    language,
  );
}

export function logoutSession(language: Language) {
  return requestJson<{ status: string }>(
    'POST',
    '/v1/onboarding/logout',
    {},
    language,
  );
}

export function claimOnboarding(code: string, language: Language) {
  return requestJson<{ status: string; csrf: string }>(
    'POST',
    '/v1/onboarding/claim',
    { code },
    language,
  );
}

export async function fetchProfile(): Promise<FarmerProfile | null> {
  const response = await fetch('/v1/me', { credentials: 'same-origin' });
  if (!response.ok) return null;
  return (await response.json()) as FarmerProfile;
}

export function updateBotConfig(
  payload: BotConfig,
  csrfToken: string,
  language: Language,
) {
  return requestJson<BotConfig>('PUT', '/v1/bot/config', payload, language, csrfToken);
}

export async function fetchKusiSource(parcelId?: string | null): Promise<Record<string, unknown>> {
  const query = parcelId ? `?parcel_id=${encodeURIComponent(parcelId)}` : '';
  const response = await fetch(`/v1/bot/kusi-source${query}`, { credentials: 'same-origin' });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return (await response.json()) as Record<string, unknown>;
}

export function createParcel(
  payload: Omit<Parcel, 'id'>,
  csrfToken: string,
  language: Language,
) {
  return requestJson<Parcel>('POST', '/v1/parcels', payload, language, csrfToken);
}

export function updateParcel(
  parcelId: string,
  payload: Omit<Parcel, 'id'>,
  csrfToken: string,
  language: Language,
) {
  return requestJson<Parcel>('PUT', `/v1/parcels/${parcelId}`, payload, language, csrfToken);
}

export function deleteParcel(
  parcelId: string,
  csrfToken: string,
  language: Language,
) {
  return requestJson<{ status: string; id: string }>(
    'DELETE',
    `/v1/parcels/${parcelId}`,
    undefined,
    language,
    csrfToken,
  );
}

export function analyzeParcel(
  parcelId: string,
  payload: { rain_delta_pct: number; temp_delta_c: number; language: Language },
  csrfToken: string,
) {
  return requestJson<RotationCompareResponse>(
    'POST',
    `/v1/parcels/${parcelId}/analyze`,
    payload,
    payload.language,
    csrfToken,
  );
}
