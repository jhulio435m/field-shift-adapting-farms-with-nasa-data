import type { Language } from './types';

export function tr(language: Language, es: string, en: string): string {
  return language === 'en' ? en : es;
}

const ENGLISH_ERRORS: Record<string, string> = {
  'Verificación Kusi Kusi no configurada': 'Kusi Kusi verification is not configured.',
  'Demasiados intentos; espera diez minutos': 'Too many attempts; wait ten minutes.',
  'Prueba de canal inválida': 'Invalid channel proof.',
  'Código inválido': 'Invalid code.',
  'Código vencido o ya utilizado': 'Code expired or already used.',
  'Parcela no encontrada': 'Field not found.',
  'Selecciona al menos una parcela': 'Select at least one field.',
  'Máximo 8 parcelas por cuenta': 'Maximum of 8 fields per account.',
  'Ponle un nombre a la parcela': 'Give the field a name.',
  'NASA POWER no disponible; no se generan recomendaciones con datos sinteticos':
    'NASA POWER is unavailable; no recommendation is generated from synthetic data.',
};

export function translateApiError(detail: string, language: Language): string {
  return language === 'en' ? ENGLISH_ERRORS[detail] || detail : detail;
}
