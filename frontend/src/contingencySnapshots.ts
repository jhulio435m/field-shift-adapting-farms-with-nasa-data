import verifiedSnapshotsRaw from './data/verifiedSnapshots.json';
import type { Language, Parcel, RotationCompareResponse } from './types';

const STORAGE_KEY = 'fieldShiftVerifiedSnapshots';
const BUNDLED_CAPTURE_TIMESTAMP = '2026-11-14T22:42:00-05:00';
const MAX_LOCAL_SNAPSHOTS = 12;

export interface VerifiedSnapshotResult {
  data: RotationCompareResponse;
  savedAt: string;
  origin: 'local_cache' | 'verified_demo_capture';
}

interface StoredEntry {
  data: RotationCompareResponse;
  savedAt: string;
}

const bundledSnapshots = verifiedSnapshotsRaw as unknown as Record<string, RotationCompareResponse>;

function buildSnapshotKey(
  parcel: Parcel,
  rainDeltaPct: number,
  tempDeltaC: number,
  language: Language,
): string {
  return [
    parcel.lat.toFixed(4),
    parcel.lon.toFixed(4),
    parcel.current_crop,
    parcel.irrigation,
    parcel.priority,
    parcel.soil_texture,
    Boolean(parcel.has_livestock) ? '1' : '0',
    rainDeltaPct,
    tempDeltaC,
    language,
  ].join(':');
}

function readStorageMap(): Record<string, StoredEntry> {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? (parsed as Record<string, StoredEntry>) : {};
  } catch {
    return {};
  }
}

export function saveVerifiedSnapshot(
  parcel: Parcel,
  rainDeltaPct: number,
  tempDeltaC: number,
  language: Language,
  data: RotationCompareResponse,
): void {
  try {
    const map = readStorageMap();
    const key = buildSnapshotKey(parcel, rainDeltaPct, tempDeltaC, language);
    map[key] = {
      data,
      savedAt: new Date().toISOString(),
    };
    const keys = Object.keys(map);
    if (keys.length > MAX_LOCAL_SNAPSHOTS) {
      keys
        .sort((a, b) => map[a].savedAt.localeCompare(map[b].savedAt))
        .slice(0, keys.length - MAX_LOCAL_SNAPSHOTS)
        .forEach((oldestKey) => {
          delete map[oldestKey];
        });
    }
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(map));
  } catch {
    // Ignore storage quota / privacy mode errors cleanly
  }
}

function resolveBundledKey(
  parcel: Parcel,
  rainDeltaPct: number,
  tempDeltaC: number,
  language: Language,
): string | null {
  const isMantaro =
    Math.abs(parcel.lat - -12.065) < 0.05 &&
    Math.abs(parcel.lon - -75.2049) < 0.05 &&
    parcel.current_crop === 'potato_continuous' &&
    parcel.irrigation === 'rainfed' &&
    parcel.priority === 'soil_health' &&
    parcel.soil_texture === 'loam' &&
    Boolean(parcel.has_livestock) === true;

  const isPiura =
    Math.abs(parcel.lat - -5.1945) < 0.05 &&
    Math.abs(parcel.lon - -80.6328) < 0.05 &&
    parcel.current_crop === 'rice_continuous' &&
    parcel.irrigation === 'irrigated' &&
    parcel.priority === 'water' &&
    parcel.soil_texture === 'clay' &&
    Boolean(parcel.has_livestock) === false;

  const region = isMantaro ? 'mantaro' : isPiura ? 'piura' : null;
  if (!region) return null;

  let scenario: string | null = null;
  if (rainDeltaPct === 0 && tempDeltaC === 0) scenario = 'normal';
  else if (rainDeltaPct === -25 && tempDeltaC === 1.5) scenario = 'drought';
  else if (rainDeltaPct === 20 && tempDeltaC === 0.5) scenario = 'rainy';

  if (!scenario) return null;
  const key = `${region}_${scenario}_${language}`;
  return key in bundledSnapshots ? key : null;
}

export function getVerifiedSnapshot(
  parcel: Parcel | null,
  rainDeltaPct: number,
  tempDeltaC: number,
  language: Language,
): VerifiedSnapshotResult | null {
  if (!parcel) return null;

  const map = readStorageMap();
  const key = buildSnapshotKey(parcel, rainDeltaPct, tempDeltaC, language);
  const localEntry = map[key];
  if (localEntry?.data) {
    return {
      data: localEntry.data,
      savedAt: localEntry.savedAt,
      origin: 'local_cache',
    };
  }

  const bundledKey = resolveBundledKey(parcel, rainDeltaPct, tempDeltaC, language);
  if (bundledKey) {
    return {
      data: bundledSnapshots[bundledKey],
      savedAt: BUNDLED_CAPTURE_TIMESTAMP,
      origin: 'verified_demo_capture',
    };
  }

  return null;
}
