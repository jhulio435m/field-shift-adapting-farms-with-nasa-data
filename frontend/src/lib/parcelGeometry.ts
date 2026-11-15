import { tr } from '../i18n';
import type { Language, Parcel } from '../types';

export type CropVisualType =
  | 'potato'
  | 'maize'
  | 'legume'
  | 'rice'
  | 'grain'
  | 'quinoa'
  | 'cover'
  | 'vegetable';

export interface SeedPlan {
  cropTitle: string;
  cropType: CropVisualType;
  seedRateKgHa: number;
  totalSeedKg: number;
  sacks50kg: number;
  sackLabel: string;
  rowSpacingCm: number;
  plantSpacingCm: number;
  estimatedPlants: number;
  furrowAdvice: string;
}

const EARTH_RADIUS_M = 6378137;
const DEG_TO_RAD = Math.PI / 180;

/**
 * Spherical excess approximation for geodesic polygon area in hectares.
 */
export function computePolygonAreaHa(vertices: [number, number][]): number {
  if (vertices.length < 3) return 0;
  let total = 0;
  for (let i = 0; i < vertices.length; i++) {
    const [lat1, lon1] = vertices[i];
    const [lat2, lon2] = vertices[(i + 1) % vertices.length];
    total +=
      (lon2 - lon1) *
      DEG_TO_RAD *
      (2 + Math.sin(lat1 * DEG_TO_RAD) + Math.sin(lat2 * DEG_TO_RAD));
  }
  const areaM2 = Math.abs((total * EARTH_RADIUS_M * EARTH_RADIUS_M) / 2);
  const ha = areaM2 / 10000;
  return Number(Math.max(0.05, ha).toFixed(2));
}

function haversineMeters(a: [number, number], b: [number, number]): number {
  const dLat = (b[0] - a[0]) * DEG_TO_RAD;
  const dLon = (b[1] - a[1]) * DEG_TO_RAD;
  const lat1 = a[0] * DEG_TO_RAD;
  const lat2 = b[0] * DEG_TO_RAD;
  const h =
    Math.sin(dLat / 2) * Math.sin(dLat / 2) +
    Math.cos(lat1) * Math.cos(lat2) * Math.sin(dLon / 2) * Math.sin(dLon / 2);
  return 2 * EARTH_RADIUS_M * Math.asin(Math.min(1, Math.sqrt(h)));
}

export function computePolygonPerimeterM(vertices: [number, number][]): number {
  if (vertices.length < 2) return 0;
  let sum = 0;
  const len = vertices.length;
  for (let i = 0; i < len; i++) {
    if (len === 2 && i === 1) break;
    sum += haversineMeters(vertices[i], vertices[(i + 1) % len]);
  }
  return Math.round(sum);
}

export function computePolygonCentroid(vertices: [number, number][]): [number, number] {
  if (vertices.length === 0) return [0, 0];
  let sumLat = 0;
  let sumLon = 0;
  for (const [lat, lon] of vertices) {
    sumLat += lat;
    sumLon += lon;
  }
  return [
    Number((sumLat / vertices.length).toFixed(4)),
    Number((sumLon / vertices.length).toFixed(4)),
  ];
}

/**
 * Returns saved parcel polygon if valid (>=3 vertices), otherwise builds a synthetic
 * slightly irregular 4-corner agricultural field scaled to area_ha around (lat, lon).
 */
export function getEffectivePolygon(
  parcel: Parcel | null | undefined,
  fallbackLat: number,
  fallbackLon: number,
): [number, number][] {
  if (parcel?.polygon && parcel.polygon.length >= 3) {
    return parcel.polygon;
  }
  const cLat = parcel ? parcel.lat : fallbackLat;
  const cLon = parcel ? parcel.lon : fallbackLon;
  const targetHa = parcel?.area_ha && parcel.area_ha > 0.05 ? parcel.area_ha : 2.0;
  const areaM2 = targetHa * 10000;
  const halfH = Math.sqrt(areaM2 * 0.78) / 2;
  const halfW = (areaM2 / (halfH * 2)) / 2;
  const mPerDegLat = 111132;
  const mPerDegLon = Math.max(20000, 111320 * Math.cos(cLat * DEG_TO_RAD));
  const dLat = halfH / mPerDegLat;
  const dLon = halfW / mPerDegLon;

  return [
    [Number((cLat + dLat * 0.96).toFixed(6)), Number((cLon - dLon * 1.04).toFixed(6))],
    [Number((cLat + dLat * 1.03).toFixed(6)), Number((cLon + dLon * 0.98).toFixed(6))],
    [Number((cLat - dLat * 0.97).toFixed(6)), Number((cLon + dLon * 1.05).toFixed(6))],
    [Number((cLat - dLat * 1.02).toFixed(6)), Number((cLon - dLon * 0.96).toFixed(6))],
  ];
}

export function classifyCropVisual(label: string): CropVisualType {
  const txt = label.toLowerCase();
  if (/papa|potato|tub[eé]rculo/.test(txt)) return 'potato';
  if (/ma[ií]z|maize|corn|choclo/.test(txt)) return 'maize';
  if (/arroz|rice/.test(txt)) return 'rice';
  if (/quinua|quinoa|kiwicha/.test(txt)) return 'quinoa';
  if (/frijol|frejol|fr[eé]jol|arveja|haba|caup[ií]|pallar|tarwi|chocho|pea|bean|lentil|cowpea|legum/.test(txt))
    return 'legume';
  if (/cobertura|pasto|vicia|tr[eé]bol|alfalfa|avena|forraje|ryegrass|cover|pasture|clover|vetch|mulch/.test(txt))
    return 'cover';
  if (/trigo|cebada|sorgo|wheat|barley|sorghum|millet|cereal/.test(txt)) return 'grain';
  return 'vegetable';
}

interface CropAgronomySpec {
  seedRateKgHa: number;
  sackWeightKg: number;
  rowSpacingCm: number;
  plantSpacingCm: number;
}

const AGRONOMY_SPECS: Record<CropVisualType, CropAgronomySpec> = {
  potato: { seedRateKgHa: 1800, sackWeightKg: 50, rowSpacingCm: 90, plantSpacingCm: 30 },
  maize: { seedRateKgHa: 28, sackWeightKg: 25, rowSpacingCm: 80, plantSpacingCm: 25 },
  legume: { seedRateKgHa: 65, sackWeightKg: 50, rowSpacingCm: 50, plantSpacingCm: 15 },
  rice: { seedRateKgHa: 95, sackWeightKg: 50, rowSpacingCm: 25, plantSpacingCm: 15 },
  grain: { seedRateKgHa: 120, sackWeightKg: 50, rowSpacingCm: 20, plantSpacingCm: 5 },
  quinoa: { seedRateKgHa: 12, sackWeightKg: 25, rowSpacingCm: 60, plantSpacingCm: 10 },
  cover: { seedRateKgHa: 55, sackWeightKg: 25, rowSpacingCm: 25, plantSpacingCm: 8 },
  vegetable: { seedRateKgHa: 6, sackWeightKg: 10, rowSpacingCm: 50, plantSpacingCm: 25 },
};

export function computeSeedPlan(
  stepLabel: string,
  areaHa: number,
  slopePct: number | null,
  language: Language,
): SeedPlan {
  const safeArea = areaHa > 0.05 ? areaHa : 1.0;
  const cropType = classifyCropVisual(stepLabel);
  const spec = AGRONOMY_SPECS[cropType];
  const totalSeedKg = Math.round(spec.seedRateKgHa * safeArea);
  const sacksCount = Number((totalSeedKg / spec.sackWeightKg).toFixed(1));
  const areaM2 = safeArea * 10000;
  const plantsPerM2 = 10000 / (spec.rowSpacingCm * spec.plantSpacingCm);
  const estimatedPlants = Math.round(areaM2 * plantsPerM2);

  const highSlope = slopePct !== null && slopePct >= 5;
  const furrowAdvice = highSlope
    ? tr(
        language,
        `Pendiente ${slopePct?.toFixed(1)}% (SRTM): trazar surcos en curvas a nivel para frenar lavado de suelo y retener lluvia.`,
        `Slope ${slopePct?.toFixed(1)}% (SRTM): align furrows along contour lines to slow runoff and retain rainfall.`,
      )
    : tr(
        language,
        slopePct !== null
          ? `Pendiente suave (${slopePct.toFixed(1)}% SRTM): surcos rectos orientados Norte–Sur para captar sol uniforme.`
          : 'Terreno uniforme: surcos rectos espaciados según densidad recomendada.',
        slopePct !== null
          ? `Gentle slope (${slopePct.toFixed(1)}% SRTM): straight North–South rows for even sun capture.`
          : 'Even terrain: straight rows following recommended spacing.',
      );

  const sackLabel = tr(
    language,
    `${sacksCount} sacos de ${spec.sackWeightKg} kg`,
    `${sacksCount} × ${spec.sackWeightKg} kg bags`,
  );

  return {
    cropTitle: stepLabel,
    cropType,
    seedRateKgHa: spec.seedRateKgHa,
    totalSeedKg,
    sacks50kg: sacksCount,
    sackLabel,
    rowSpacingCm: spec.rowSpacingCm,
    plantSpacingCm: spec.plantSpacingCm,
    estimatedPlants,
    furrowAdvice,
  };
}
