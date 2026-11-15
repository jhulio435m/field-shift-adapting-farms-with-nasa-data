import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Box,
  CloudRain,
  Compass,
  Droplets,
  Flame,
  Layers,
  Mountain,
  Pause,
  PenTool,
  Play,
  RotateCcw,
  Snowflake,
  Sprout,
  Sun,
  Thermometer,
  ZoomIn,
  ZoomOut,
} from 'lucide-react';
import { tr } from '../i18n';
import {
  classifyCropVisual,
  computePolygonAreaHa,
  computePolygonPerimeterM,
  computeSeedPlan,
  getEffectivePolygon,
  type CropVisualType,
} from '../lib/parcelGeometry';
import type {
  Language,
  MonthlyClimate,
  Parcel,
  ParcelDiagnosis,
  RotationStrategy,
  ViewMode,
} from '../types';
import { Button } from './ui/button';

interface Parcel3DViewProps {
  language: Language;
  viewMode: ViewMode;
  parcel: Parcel | null;
  lat: number;
  lon: number;
  diagnosis: ParcelDiagnosis | null;
  strategies: RotationStrategy[];
  rainDeltaPct: number;
  tempDeltaC: number;
  onChangeScenario: (rain: number, temp: number) => void;
  onRequestDrawPolygon: () => void;
}

interface Vec3 {
  x: number;
  y: number;
  z: number;
}

interface ProjectedPt {
  sx: number;
  sy: number;
  depth: number;
}

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

function lerpColor(a: [number, number, number], b: [number, number, number], t: number): string {
  const k = clamp(t, 0, 1);
  const r = Math.round(a[0] + (b[0] - a[0]) * k);
  const g = Math.round(a[1] + (b[1] - a[1]) * k);
  const bl = Math.round(a[2] + (b[2] - a[2]) * k);
  return `rgb(${r}, ${g}, ${bl})`;
}

/** Projects polygon geo-coordinates into normalized [-1, 1] XZ world coordinates. */
function normalizePolygonXZ(geoVerts: [number, number][]): Array<{ x: number; z: number }> {
  if (geoVerts.length < 3) {
    return [
      { x: -0.9, z: -0.75 },
      { x: 0.9, z: -0.75 },
      { x: 0.9, z: 0.75 },
      { x: -0.9, z: 0.75 },
    ];
  }
  let cLat = 0;
  let cLon = 0;
  for (const [lt, ln] of geoVerts) {
    cLat += lt;
    cLon += ln;
  }
  cLat /= geoVerts.length;
  cLon /= geoVerts.length;

  const cosLat = Math.max(0.25, Math.cos((cLat * Math.PI) / 180));
  const raw = geoVerts.map(([lt, ln]) => ({
    x: (ln - cLon) * 111320 * cosLat,
    z: -(lt - cLat) * 111132,
  }));

  let maxSpan = 1;
  for (const p of raw) {
    maxSpan = Math.max(maxSpan, Math.abs(p.x), Math.abs(p.z));
  }
  const scale = 0.92 / maxSpan;
  return raw.map((p) => ({ x: p.x * scale, z: p.z * scale }));
}

function pointInPolyXZ(px: number, pz: number, poly: Array<{ x: number; z: number }>): boolean {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const xi = poly[i].x;
    const zi = poly[i].z;
    const xj = poly[j].x;
    const zj = poly[j].z;
    const intersect = zi > pz !== zj > pz && px < ((xj - xi) * (pz - zi)) / (zj - zi + 1e-9) + xi;
    if (intersect) inside = !inside;
  }
  return inside;
}

function getPhenologyState(
  calendar: RotationStrategy['monthly_calendar'],
  monthIdx: number,
  language: Language,
): {
  isBare: boolean;
  stageLabel: string;
  growthFactor: number;
  isSeeding: boolean;
  cropVisual: CropVisualType;
  cropText: string;
} {
  const current = calendar[monthIdx];
  const cropText = current?.crop_or_cover || tr(language, 'Suelo', 'Soil');
  const barePattern = /descanso|barbecho|desnudo|expuesto|sin cultivo|fallow|bare/i;
  if (!current || current.status === 'bare_risk' || barePattern.test(cropText)) {
    return {
      isBare: true,
      stageLabel: tr(language, 'Suelo descubierto / descanso', 'Bare / fallow ground'),
      growthFactor: 0,
      isSeeding: false,
      cropVisual: 'cover',
      cropText,
    };
  }

  const normLabel = cropText.trim().toLowerCase();
  const prevSame =
    calendar[(monthIdx + 11) % 12]?.crop_or_cover?.trim().toLowerCase() === normLabel &&
    calendar[(monthIdx + 11) % 12]?.status !== 'bare_risk';
  const nextSame =
    calendar[(monthIdx + 1) % 12]?.crop_or_cover?.trim().toLowerCase() === normLabel &&
    calendar[(monthIdx + 1) % 12]?.status !== 'bare_risk';

  let growthFactor = 0.72;
  let stageLabel = tr(language, 'Crecimiento y cobertura', 'Vegetative growth & cover');
  let isSeeding = false;

  if (!prevSame && nextSame) {
    growthFactor = 0.28;
    isSeeding = true;
    stageLabel = tr(language, 'Siembra y germinación de semillas', 'Seeding & germination');
  } else if (prevSame && !nextSame) {
    growthFactor = 1.0;
    stageLabel = tr(language, 'Madurez / cosecha o corte', 'Maturity / harvest window');
  } else if (!prevSame && !nextSame) {
    growthFactor = 0.45;
    isSeeding = true;
    stageLabel = tr(language, 'Establecimiento rápido', 'Rapid establishment');
  }

  return {
    isBare: false,
    stageLabel,
    growthFactor,
    isSeeding,
    cropVisual: classifyCropVisual(cropText),
    cropText,
  };
}

export function Parcel3DView({
  language,
  viewMode,
  parcel,
  lat,
  lon,
  diagnosis,
  strategies,
  rainDeltaPct,
  tempDeltaC,
  onChangeScenario,
  onRequestDrawPolygon,
}: Parcel3DViewProps) {
  const [selectedStrategyId, setSelectedStrategyId] = useState<string>('');
  const [monthIdx, setMonthIdx] = useState<number>(() => Math.max(0, (parcel?.planting_month ?? 10) - 1));
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const [yaw, setYaw] = useState<number>(0.68);
  const [pitch, setPitch] = useState<number>(0.58);
  const [zoom, setZoom] = useState<number>(1.0);

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const dragRef = useRef<{ active: boolean; x: number; y: number }>({ active: false, x: 0, y: 0 });
  const animRef = useRef<number>(0);

  const nonCurrentTop = useMemo(() => {
    const nonCurr = strategies.filter((s) => !s.is_current).sort((a, b) => b.fit_score - a.fit_score);
    return nonCurr[0] ?? strategies[0] ?? null;
  }, [strategies]);

  useEffect(() => {
    if (!strategies.length) {
      setSelectedStrategyId('');
      return;
    }
    const exists = strategies.some((s) => s.id === selectedStrategyId);
    if (!exists && nonCurrentTop) {
      setSelectedStrategyId(nonCurrentTop.id);
    }
  }, [strategies, selectedStrategyId, nonCurrentTop]);

  useEffect(() => {
    if (parcel?.planting_month) {
      setMonthIdx(clamp(parcel.planting_month - 1, 0, 11));
    }
  }, [parcel?.id, parcel?.planting_month]);

  useEffect(() => {
    if (!isPlaying) return;
    const id = window.setInterval(() => {
      setMonthIdx((prev) => (prev + 1) % 12);
    }, 1150);
    return () => window.clearInterval(id);
  }, [isPlaying]);

  const activeStrategy =
    strategies.find((s) => s.id === selectedStrategyId) ?? nonCurrentTop ?? null;

  const effectivePolygon = useMemo(
    () => getEffectivePolygon(parcel, lat, lon),
    [parcel, lat, lon],
  );
  const hasCustomPolygon = Boolean(parcel?.polygon && parcel.polygon.length >= 3);
  const areaHa = parcel?.area_ha && parcel.area_ha > 0 ? parcel.area_ha : computePolygonAreaHa(effectivePolygon);
  const perimeterM = computePolygonPerimeterM(effectivePolygon);
  const slopePct = diagnosis?.terrain.slope_pct ?? 4.0;
  const elevationM = diagnosis?.terrain.elevation_m ?? null;

  const monthly = diagnosis?.monthly ?? [];
  const currentMonth: MonthlyClimate | null = monthly[monthIdx] ?? monthly[0] ?? null;
  const calendar = activeStrategy?.monthly_calendar ?? [];
  const phenology = useMemo(
    () => getPhenologyState(calendar, monthIdx, language),
    [calendar, monthIdx, language],
  );

  const seedPlans = useMemo(() => {
    const steps = activeStrategy?.sequence ?? [phenology.cropText];
    return steps.map((step) => computeSeedPlan(step, areaHa, slopePct, language));
  }, [activeStrategy?.sequence, phenology.cropText, areaHa, slopePct, language]);

  const activeSeedPlan = useMemo(
    () =>
      seedPlans.find((sp) => sp.cropType === phenology.cropVisual) ??
      seedPlans[0] ??
      computeSeedPlan(phenology.cropText, areaHa, slopePct, language),
    [seedPlans, phenology.cropVisual, phenology.cropText, areaHa, slopePct, language],
  );

  const polyXZ = useMemo(() => normalizePolygonXZ(effectivePolygon), [effectivePolygon]);

  const sceneStateRef = useRef({
    yaw,
    pitch,
    zoom,
    polyXZ,
    slopePct,
    currentMonth,
    phenology,
    isIrrigated: parcel?.irrigation === 'irrigated',
    rainDeltaPct,
  });
  sceneStateRef.current = {
    yaw,
    pitch,
    zoom,
    polyXZ,
    slopePct,
    currentMonth,
    phenology,
    isIrrigated: parcel?.irrigation === 'irrigated',
    rainDeltaPct,
  };

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const raindrops = Array.from({ length: 56 }, (_, i) => ({
      x: ((i * 73) % 100) / 100,
      y: ((i * 41) % 100) / 100,
      speed: 0.55 + ((i * 19) % 45) / 100,
    }));

    const renderFrame = (timeMs: number) => {
      const rect = canvas.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const cw = Math.max(300, Math.round(rect.width * dpr));
      const ch = Math.max(200, Math.round(rect.height * dpr));
      if (canvas.width !== cw || canvas.height !== ch) {
        canvas.width = cw;
        canvas.height = ch;
      }

      const w = cw / dpr;
      const h = ch / dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

      const {
        yaw: curYaw,
        pitch: curPitch,
        zoom: curZoom,
        polyXZ: vertsXZ,
        slopePct: curSlope,
        currentMonth: m,
        phenology: phen,
        isIrrigated,
      } = sceneStateRef.current;

      const precipMm = m?.precip_mm_month ?? 45;
      const moistTop = m?.soil_moisture_top ?? 0.42;
      const moistRoot = m?.soil_moisture_root ?? 0.48;
      const isFrost = Boolean(m?.frost_risk);
      const isDry = Boolean(m?.dry_stress);
      const isHeat = Boolean(m?.heat_stress);
      const etMm = m?.evapotranspiration_mm_month ?? 80;

      // Sky gradient reacting to climate conditions
      const skyGrad = ctx.createLinearGradient(0, 0, 0, h);
      if (isFrost) {
        skyGrad.addColorStop(0, '#dbeafe');
        skyGrad.addColorStop(0.55, '#eff6ff');
        skyGrad.addColorStop(1, '#e2e8f0');
      } else if (precipMm >= 95) {
        skyGrad.addColorStop(0, '#cbd5e1');
        skyGrad.addColorStop(0.55, '#e2e8f0');
        skyGrad.addColorStop(1, '#f1f5f9');
      } else if (isDry || isHeat) {
        skyGrad.addColorStop(0, '#fef3c7');
        skyGrad.addColorStop(0.55, '#fffbeb');
        skyGrad.addColorStop(1, '#f5f5f4');
      } else {
        skyGrad.addColorStop(0, '#dcfce7');
        skyGrad.addColorStop(0.55, '#f0fdf4');
        skyGrad.addColorStop(1, '#fafaf8');
      }
      ctx.fillStyle = skyGrad;
      ctx.fillRect(0, 0, w, h);

      // Sun or storm indicator in sky corner
      if (precipMm < 95) {
        const sunColor = isFrost ? 'rgba(147, 197, 253, 0.55)' : isDry || isHeat ? 'rgba(245, 158, 11, 0.45)' : 'rgba(250, 204, 21, 0.42)';
        ctx.beginPath();
        ctx.arc(w - 58, 46, isDry || isHeat ? 26 : 20, 0, Math.PI * 2);
        ctx.fillStyle = sunColor;
        ctx.fill();
      }

      const cosY = Math.cos(curYaw);
      const sinY = Math.sin(curYaw);
      const cosP = Math.cos(curPitch);
      const sinP = Math.sin(curPitch);
      const fovScale = Math.min(w, h) * 0.43 * curZoom;
      const centerX = w * 0.5;
      const centerY = h * 0.57;

      const project = (pt: Vec3): ProjectedPt => {
        const rx = pt.x * cosY - pt.z * sinY;
        const rz = pt.x * sinY + pt.z * cosY;
        const ry = pt.y * cosP - rz * sinP;
        const dz = pt.y * sinP + rz * cosP;
        const persp = 4.6 / (4.6 + dz);
        return {
          sx: centerX + rx * fovScale * persp,
          sy: centerY - ry * fovScale * persp,
          depth: dz,
        };
      };

      // Surface elevation from real SRTM slope
      const slopeTilt = clamp(curSlope / 32, 0.02, 0.36);
      const surfaceY = (x: number, z: number): number =>
        0.16 + x * slopeTilt + Math.sin(x * 3.1 + z * 2.4) * 0.015;

      const topVerts: Vec3[] = vertsXZ.map((p) => ({ x: p.x, y: surfaceY(p.x, p.z), z: p.z }));
      const midYOffset = 0.11; // Topsoil 0-5cm slice
      const deepYOffset = 0.44; // Root zone 0-100cm slice
      const midVerts: Vec3[] = topVerts.map((v) => ({ x: v.x, y: v.y - midYOffset, z: v.z }));
      const botVerts: Vec3[] = topVerts.map((v) => ({ x: v.x, y: v.y - deepYOffset, z: v.z }));

      const topProj = topVerts.map(project);
      const midProj = midVerts.map(project);
      const botProj = botVerts.map(project);

      // Soil palette driven by live SMAP moisture
      const topSoilColor = isFrost
        ? lerpColor([180, 175, 165], [148, 163, 184], 0.38)
        : lerpColor([198, 152, 92], [68, 44, 24], clamp((moistTop - 0.15) / 0.55, 0, 1));
      const rootSoilColor = lerpColor(
        [176, 134, 88],
        [45, 74, 88],
        clamp((moistRoot - 0.18) / 0.6, 0, 1) * 0.65,
      );
      const surfaceBaseColor = phen.isBare
        ? isDry
          ? '#d6b482'
          : '#8f653b'
        : phen.cropVisual === 'rice' && (precipMm >= 45 || isIrrigated)
          ? '#3b6e5c'
          : lerpColor([168, 132, 82], [64, 92, 48], phen.growthFactor * 0.62);

      // Subtle ground shadow
      ctx.save();
      ctx.beginPath();
      botProj.forEach((p, i) => {
        if (i === 0) ctx.moveTo(p.sx + 8, p.sy + 14);
        else ctx.lineTo(p.sx + 8, p.sy + 14);
      });
      ctx.closePath();
      ctx.fillStyle = 'rgba(17, 24, 19, 0.14)';
      ctx.fill();
      ctx.restore();

      // Side cross-section walls sorted back-to-front
      const sideWalls = topVerts.map((_, i) => {
        const j = (i + 1) % topVerts.length;
        const avgDepth = (botProj[i].depth + botProj[j].depth + topProj[i].depth + topProj[j].depth) / 4;
        return { i, j, avgDepth };
      });
      sideWalls.sort((a, b) => b.avgDepth - a.avgDepth);

      sideWalls.forEach(({ i, j }, idx) => {
        // Root zone lower slice (0-100cm SMAP)
        ctx.beginPath();
        ctx.moveTo(midProj[i].sx, midProj[i].sy);
        ctx.lineTo(midProj[j].sx, midProj[j].sy);
        ctx.lineTo(botProj[j].sx, botProj[j].sy);
        ctx.lineTo(botProj[i].sx, botProj[i].sy);
        ctx.closePath();
        ctx.fillStyle = rootSoilColor;
        ctx.fill();
        ctx.strokeStyle = 'rgba(17, 24, 19, 0.28)';
        ctx.lineWidth = 1;
        ctx.stroke();

        // Draw subterranean root fibers on exposed front-facing walls
        if (idx >= Math.floor(sideWalls.length / 2) && !phen.isBare) {
          const rootDepthRatio = 0.25 + phen.growthFactor * 0.55;
          for (let r = 1; r <= 4; r++) {
            const t = r / 5;
            const rx = midProj[i].sx + (midProj[j].sx - midProj[i].sx) * t;
            const ryTop = midProj[i].sy + (midProj[j].sy - midProj[i].sy) * t;
            const ryBot = botProj[i].sy + (botProj[j].sy - botProj[i].sy) * t;
            const rLen = (ryBot - ryTop) * rootDepthRatio;
            ctx.beginPath();
            ctx.moveTo(rx, ryTop - 2);
            ctx.quadraticCurveTo(rx + (r % 2 === 0 ? 5 : -5), ryTop + rLen * 0.5, rx, ryTop + rLen);
            ctx.strokeStyle = phen.cropVisual === 'legume' ? 'rgba(234, 214, 150, 0.78)' : 'rgba(218, 198, 158, 0.6)';
            ctx.lineWidth = 1.4;
            ctx.stroke();
            if (phen.cropVisual === 'legume') {
              ctx.beginPath();
              ctx.arc(rx + (r % 2 === 0 ? 2 : -2), ryTop + rLen * 0.65, 2.2, 0, Math.PI * 2);
              ctx.fillStyle = '#bef264';
              ctx.fill();
            }
          }
        }

        // Topsoil slice (0-5cm SMAP)
        ctx.beginPath();
        ctx.moveTo(topProj[i].sx, topProj[i].sy);
        ctx.lineTo(topProj[j].sx, topProj[j].sy);
        ctx.lineTo(midProj[j].sx, midProj[j].sy);
        ctx.lineTo(midProj[i].sx, midProj[i].sy);
        ctx.closePath();
        ctx.fillStyle = topSoilColor;
        ctx.fill();
        ctx.strokeStyle = 'rgba(17, 24, 19, 0.35)';
        ctx.lineWidth = 1;
        ctx.stroke();
      });

      // Top polygon surface
      ctx.save();
      ctx.beginPath();
      topProj.forEach((p, idx) => {
        if (idx === 0) ctx.moveTo(p.sx, p.sy);
        else ctx.lineTo(p.sx, p.sy);
      });
      ctx.closePath();
      ctx.fillStyle = surfaceBaseColor;
      ctx.fill();
      ctx.clip();

      // Contour or straight furrows inside the polygon
      const isContour = curSlope >= 5;
      for (let f = -6; f <= 6; f++) {
        const xVal = f * 0.14;
        ctx.beginPath();
        for (let s = 0; s <= 12; s++) {
          const zVal = -1.0 + (s / 12) * 2.0;
          const curveX = isContour ? xVal + Math.sin(zVal * 2.2) * 0.08 : xVal;
          const p = project({ x: curveX, y: surfaceY(curveX, zVal) + 0.004, z: zVal });
          if (s === 0) ctx.moveTo(p.sx, p.sy);
          else ctx.lineTo(p.sx, p.sy);
        }
        ctx.strokeStyle =
          phen.cropVisual === 'rice' && (precipMm >= 45 || isIrrigated)
            ? 'rgba(125, 211, 252, 0.38)'
            : isFrost
              ? 'rgba(226, 232, 240, 0.55)'
              : 'rgba(56, 38, 20, 0.34)';
        ctx.lineWidth = phen.cropVisual === 'potato' ? 3.5 : 2;
        ctx.stroke();
      }
      ctx.restore();

      // Crisp polygon boundary outline
      ctx.beginPath();
      topProj.forEach((p, idx) => {
        if (idx === 0) ctx.moveTo(p.sx, p.sy);
        else ctx.lineTo(p.sx, p.sy);
      });
      ctx.closePath();
      ctx.strokeStyle = '#14532d';
      ctx.lineWidth = 2;
      ctx.stroke();

      // Generate plant & seed nodes strictly inside the polygon, depth-sorted
      interface PlantNode {
        x: number;
        z: number;
        y: number;
        proj: ProjectedPt;
        jitter: number;
      }
      const plants: PlantNode[] = [];
      const rowStep = phen.cropVisual === 'cover' || phen.cropVisual === 'grain' ? 0.19 : 0.24;
      const colStep = phen.cropVisual === 'cover' ? 0.18 : 0.23;

      for (let gx = -0.82; gx <= 0.82; gx += rowStep) {
        for (let gz = -0.78; gz <= 0.78; gz += colStep) {
          const cx = isContour ? gx + Math.sin(gz * 2.2) * 0.07 : gx;
          if (!pointInPolyXZ(cx, gz, vertsXZ)) continue;
          const y = surfaceY(cx, gz);
          const proj = project({ x: cx, y, z: gz });
          const seedJitter = Math.abs(Math.sin(cx * 31.7 + gz * 17.3));
          plants.push({ x: cx, z: gz, y, proj, jitter: seedJitter });
        }
      }
      plants.sort((a, b) => b.proj.depth - a.proj.depth);

      const scaleByZoom = curZoom * (w / 620);

      plants.forEach((pl) => {
        const px = pl.proj.sx;
        const py = pl.proj.sy;
        const sc = clamp(scaleByZoom * (4.6 / (4.6 + pl.proj.depth)), 0.55, 1.8);

        if (phen.isBare) {
          // Sparse dry residue flecks
          ctx.beginPath();
          ctx.moveTo(px - 3 * sc, py);
          ctx.lineTo(px + 3 * sc, py - 1.5 * sc);
          ctx.strokeStyle = isDry ? '#a16207' : '#78350f';
          ctx.lineWidth = 1.2;
          ctx.stroke();
          return;
        }

        if (phen.isSeeding) {
          // Visible planted seed + tiny green cotyledon sprout
          ctx.beginPath();
          ctx.ellipse(px, py + 1.2 * sc, 2.8 * sc, 1.8 * sc, 0.2, 0, Math.PI * 2);
          ctx.fillStyle = phen.cropVisual === 'potato' ? '#d97706' : '#eab308';
          ctx.fill();
          ctx.beginPath();
          ctx.moveTo(px, py);
          ctx.lineTo(px, py - 8 * sc * phen.growthFactor * 2.2);
          ctx.strokeStyle = isFrost ? '#93c5fd' : '#4d7c0f';
          ctx.lineWidth = 1.8 * sc;
          ctx.stroke();
          ctx.beginPath();
          ctx.arc(px + 2 * sc, py - 8 * sc * phen.growthFactor * 2.2, 2.2 * sc, 0, Math.PI * 2);
          ctx.fillStyle = isDry ? '#ca8a04' : '#65a30d';
          ctx.fill();
          return;
        }

        const heightPx =
          (phen.cropVisual === 'maize' || phen.cropVisual === 'quinoa' ? 30 : phen.cropVisual === 'cover' ? 12 : 20) *
          phen.growthFactor *
          sc;
        const foliage = isFrost
          ? '#64748b'
          : isDry
            ? '#a3a826'
            : phen.cropVisual === 'legume'
              ? '#15803d'
              : '#1b4332';
        const tipColor = isFrost
          ? '#e0f2fe'
          : isDry
            ? '#eab308'
            : phen.cropVisual === 'quinoa'
              ? '#b91c1c'
              : phen.cropVisual === 'maize'
                ? '#facc15'
                : '#4ade80';

        // Stem / canopy silhouette by crop class
        if (phen.cropVisual === 'maize' || phen.cropVisual === 'quinoa') {
          ctx.beginPath();
          ctx.moveTo(px, py);
          ctx.lineTo(px + (pl.jitter - 0.5) * 3, py - heightPx);
          ctx.strokeStyle = foliage;
          ctx.lineWidth = 2.2 * sc;
          ctx.stroke();
          // Leaves
          ctx.beginPath();
          ctx.moveTo(px, py - heightPx * 0.45);
          ctx.quadraticCurveTo(px - 8 * sc, py - heightPx * 0.6, px - 10 * sc, py - heightPx * 0.35);
          ctx.moveTo(px, py - heightPx * 0.55);
          ctx.quadraticCurveTo(px + 8 * sc, py - heightPx * 0.72, px + 10 * sc, py - heightPx * 0.48);
          ctx.strokeStyle = foliage;
          ctx.lineWidth = 1.6 * sc;
          ctx.stroke();
          // Tassel or panicle
          ctx.beginPath();
          ctx.arc(px, py - heightPx - 1.5 * sc, 2.6 * sc, 0, Math.PI * 2);
          ctx.fillStyle = tipColor;
          ctx.fill();
        } else if (phen.cropVisual === 'potato' || phen.cropVisual === 'legume' || phen.cropVisual === 'vegetable') {
          ctx.beginPath();
          ctx.arc(px, py - heightPx * 0.55, 6.2 * sc * phen.growthFactor + 2, 0, Math.PI * 2);
          ctx.fillStyle = foliage;
          ctx.fill();
          ctx.beginPath();
          ctx.arc(px - 2 * sc, py - heightPx * 0.75, 2.8 * sc, 0, Math.PI * 2);
          ctx.fillStyle = tipColor;
          ctx.fill();
        } else {
          // Rice / grain / dense pasture cover
          for (let b = -1; b <= 1; b++) {
            ctx.beginPath();
            ctx.moveTo(px + b * 2 * sc, py);
            ctx.lineTo(px + b * 4.5 * sc, py - heightPx);
            ctx.strokeStyle = b === 0 ? foliage : tipColor;
            ctx.lineWidth = 1.7 * sc;
            ctx.stroke();
          }
        }
      });

      // Dynamic climate particles (Rain / Frost crystals / Heat-ET wisps)
      const sec = timeMs * 0.001;
      if (precipMm >= 55) {
        const dropCount = Math.min(raindrops.length, Math.round(precipMm / 3.5));
        ctx.strokeStyle = 'rgba(37, 99, 235, 0.45)';
        ctx.lineWidth = 1.3;
        for (let i = 0; i < dropCount; i++) {
          const d = raindrops[i];
          const rx = ((d.x + sec * 0.08) % 1) * w;
          const ry = ((d.y + sec * d.speed) % 1) * (h * 0.85);
          ctx.beginPath();
          ctx.moveTo(rx, ry);
          ctx.lineTo(rx - 3, ry + 11);
          ctx.stroke();
        }
      }

      if (isFrost) {
        ctx.fillStyle = 'rgba(219, 234, 254, 0.9)';
        for (let i = 0; i < 24; i++) {
          const d = raindrops[i];
          const fx = ((d.x + Math.sin(sec + i) * 0.02 + 1) % 1) * w;
          const fy = ((d.y + sec * 0.18) % 1) * (h * 0.82);
          ctx.beginPath();
          ctx.arc(fx, fy, 2.2, 0, Math.PI * 2);
          ctx.fill();
        }
      }

      if ((isDry || isHeat) && etMm >= 80) {
        ctx.strokeStyle = 'rgba(217, 119, 6, 0.38)';
        ctx.lineWidth = 1.4;
        for (let i = 0; i < 6; i++) {
          const baseX = w * 0.28 + i * (w * 0.085);
          const phase = (sec * 0.9 + i * 0.4) % 1;
          const baseY = centerY - phase * 55;
          ctx.beginPath();
          ctx.moveTo(baseX, baseY);
          ctx.quadraticCurveTo(baseX + 5, baseY - 9, baseX, baseY - 18);
          ctx.stroke();
        }
      }

      animRef.current = window.requestAnimationFrame(renderFrame);
    };

    animRef.current = window.requestAnimationFrame(renderFrame);
    return () => window.cancelAnimationFrame(animRef.current);
  }, []);

  if (!diagnosis) return null;

  const isDrought = rainDeltaPct === -25 && tempDeltaC === 1.5;
  const isRainy = rainDeltaPct === 20 && tempDeltaC === 0.5;
  const scenarioActive = rainDeltaPct !== 0 || tempDeltaC !== 0;

  return (
    <section
      aria-label={tr(language, 'Visor 3D de parcela, semillas y clima', '3D field, seeds & climate studio')}
      className="rounded-xl border border-[#e3e6e1] bg-white p-4 sm:p-5 space-y-4"
    >
      {/* Unified Header: 3D Simulator + Integrated Climate Scenario Controls */}
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-base font-bold text-[#111813] sm:text-lg">
            <Box className="h-5 w-5 text-[#1b4332]" />
            <span>
              {tr(
                language,
                'Vista 3D de la parcela, semillas y respuesta al clima',
                '3D Field View, Seed Plan & Climate Response',
              )}
            </span>
          </h2>
          <p className="mt-0.5 text-xs text-[#475569] sm:text-sm">
            {tr(
              language,
              'Gira tu parcela en 3D con pendiente real NASA SRTM, humedad SMAP en 2 capas y simula mes a mes cómo responde bajo sequía o lluvias.',
              'Orbit your field in 3D with NASA SRTM slope and 2-layer SMAP moisture across all 12 months and climate shifts.',
            )}
          </p>
        </div>

        {/* Climate scenario controls moved cleanly into the 3D simulator */}
        <div className="flex flex-wrap items-center gap-1.5 rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-1">
          <button
            type="button"
            onClick={() => onChangeScenario(0, 0)}
            aria-pressed={!scenarioActive}
            className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer ${
              !scenarioActive
                ? 'bg-[#1b4332] text-white'
                : 'text-[#334155] hover:bg-white hover:text-[#111813]'
            }`}
          >
            <Sun className="h-3.5 w-3.5" />
            <span>{tr(language, 'Clima Normal', 'Normal')}</span>
          </button>
          <button
            type="button"
            onClick={() => onChangeScenario(-25, 1.5)}
            aria-pressed={isDrought}
            className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer ${
              isDrought
                ? 'bg-[#b45309] text-white'
                : 'text-[#334155] hover:bg-[#fef3c7] hover:text-[#92400e]'
            }`}
          >
            <Flame className="h-3.5 w-3.5" />
            <span>{tr(language, 'Sequía −25%', 'Drought −25%')}</span>
          </button>
          <button
            type="button"
            onClick={() => onChangeScenario(20, 0.5)}
            aria-pressed={isRainy}
            className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer ${
              isRainy
                ? 'bg-[#1d4ed8] text-white'
                : 'text-[#334155] hover:bg-[#dbeafe] hover:text-[#1e3a8a]'
            }`}
          >
            <CloudRain className="h-3.5 w-3.5" />
            <span>{tr(language, 'Lluvioso +20%', 'Rainy +20%')}</span>
          </button>
        </div>
      </div>

      {/* Main 3D Canvas (Left) + Seed & Agronomic Readout (Right) */}
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1.28fr_0.72fr] lg:items-start">
        <div className="space-y-3">
          {/* Strategy Selector pills directly tied to the 3D scene */}
          <div className="flex flex-wrap items-center gap-1.5">
            {strategies.map((strat) => {
              const isSelected = strat.id === activeStrategy?.id;
              return (
                <button
                  key={strat.id}
                  type="button"
                  onClick={() => setSelectedStrategyId(strat.id)}
                  aria-pressed={isSelected}
                  className={`inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs font-semibold transition-colors cursor-pointer ${
                    isSelected
                      ? 'border-[#1b4332] bg-[#e6f4ea] text-[#14532d]'
                      : 'border-[#e3e6e1] bg-[#fafaf8] text-[#334155] hover:bg-[#f0f2ee]'
                  }`}
                >
                  <Sprout className="h-3.5 w-3.5" />
                  <span>{strat.is_current ? tr(language, `Actual: ${strat.name}`, `Current: ${strat.name}`) : strat.name}</span>
                  <span className="rounded bg-white px-1.5 py-0.5 font-mono text-[11px] font-bold text-[#111813] tabular-nums">
                    {strat.fit_score}
                  </span>
                </button>
              );
            })}
          </div>

          {/* 3D Canvas Viewport */}
          <div className="relative h-[320px] w-full overflow-hidden rounded-xl border border-[#e3e6e1] bg-[#fafaf8] sm:h-[360px]">
            <canvas
              ref={canvasRef}
              onPointerDown={(e) => {
                dragRef.current = { active: true, x: e.clientX, y: e.clientY };
                e.currentTarget.setPointerCapture(e.pointerId);
              }}
              onPointerMove={(e) => {
                if (!dragRef.current.active) return;
                const dx = e.clientX - dragRef.current.x;
                const dy = e.clientY - dragRef.current.y;
                dragRef.current = { active: true, x: e.clientX, y: e.clientY };
                setYaw((prev) => prev + dx * 0.01);
                setPitch((prev) => clamp(prev + dy * 0.008, 0.25, 1.12));
              }}
              onPointerUp={(e) => {
                dragRef.current.active = false;
                if (e.currentTarget.hasPointerCapture(e.pointerId)) {
                  e.currentTarget.releasePointerCapture(e.pointerId);
                }
              }}
              className="block h-full w-full touch-none cursor-grab active:cursor-grabbing"
              aria-label={tr(language, 'Escena 3D interactiva del cultivo y suelo', 'Interactive 3D crop and soil scene')}
            />

            {/* Top-left Active Stage & Soil Cut Badge */}
            <div className="pointer-events-none absolute left-3 top-3 flex max-w-[calc(100%-96px)] flex-wrap items-center gap-1.5">
              <span className="rounded-md border border-[#d6dad3] bg-white/95 px-2.5 py-1 text-xs font-bold text-[#111813] shadow-xs">
                {currentMonth?.month_label} · {phenology.cropText}
              </span>
              <span className="rounded-md border border-[#d6dad3] bg-white/95 px-2 py-1 font-mono text-xs font-semibold text-[#1b4332] shadow-xs tabular-nums">
                {areaHa.toFixed(2)} ha · {slopePct.toFixed(1)}% SRTM
              </span>
            </div>

            {/* Top-right Orbit & Zoom Controls */}
            <div className="absolute right-3 top-3 flex items-center gap-1">
              <button
                type="button"
                title={tr(language, 'Acercar', 'Zoom in')}
                onClick={() => setZoom((z) => clamp(z + 0.15, 0.7, 1.45))}
                className="rounded-md border border-[#d6dad3] bg-white/95 p-1.5 text-[#111813] shadow-xs hover:bg-[#fafaf8] cursor-pointer"
              >
                <ZoomIn className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                title={tr(language, 'Alejar', 'Zoom out')}
                onClick={() => setZoom((z) => clamp(z - 0.15, 0.7, 1.45))}
                className="rounded-md border border-[#d6dad3] bg-white/95 p-1.5 text-[#111813] shadow-xs hover:bg-[#fafaf8] cursor-pointer"
              >
                <ZoomOut className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                title={tr(language, 'Reiniciar vista 3D', 'Reset 3D camera')}
                onClick={() => {
                  setYaw(0.68);
                  setPitch(0.58);
                  setZoom(1.0);
                }}
                className="rounded-md border border-[#d6dad3] bg-white/95 p-1.5 text-[#111813] shadow-xs hover:bg-[#fafaf8] cursor-pointer"
              >
                <RotateCcw className="h-3.5 w-3.5" />
              </button>
            </div>

            {/* Bottom-left Subsurface Legend & Orbit Hint */}
            <div className="pointer-events-none absolute bottom-2.5 left-3 right-3 flex flex-wrap items-center justify-between gap-2">
              <div className="inline-flex items-center gap-2 rounded-md border border-[#d6dad3] bg-white/95 px-2.5 py-1 text-[11px] font-medium text-[#111813] shadow-xs">
                <span className="inline-flex items-center gap-1">
                  <span className="h-2.5 w-2.5 rounded-xs bg-[#6b4423]" />
                  {tr(language, 'Sup. 0–5 cm:', 'Top 0–5cm:')}{' '}
                  <strong className="font-mono">{Math.round((currentMonth?.soil_moisture_top ?? 0) * 100)}%</strong>
                </span>
                <span className="inline-flex items-center gap-1">
                  <span className="h-2.5 w-2.5 rounded-xs bg-[#334e58]" />
                  {tr(language, 'Raíz SMAP:', 'Root SMAP:')}{' '}
                  <strong className="font-mono">{Math.round((currentMonth?.soil_moisture_root ?? 0) * 100)}%</strong>
                </span>
              </div>

              <span className="rounded-md bg-white/90 px-2 py-1 text-[11px] font-medium text-[#475569] shadow-xs">
                {tr(language, 'Arrastra para girar 360°', 'Drag to orbit 360°')}
              </span>
            </div>
          </div>

          {/* 12-Month Timeline & Play Year Simulation */}
          <div className="rounded-xl border border-[#e3e6e1] bg-[#fafaf8] p-3 space-y-2.5">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setIsPlaying((p) => !p)}
                  className="inline-flex items-center gap-1.5 rounded-md bg-[#1b4332] px-3 py-1.5 text-xs font-bold text-white hover:bg-[#143326] cursor-pointer"
                >
                  {isPlaying ? <Pause className="h-3.5 w-3.5" /> : <Play className="h-3.5 w-3.5" />}
                  <span>
                    {isPlaying
                      ? tr(language, 'Pausar año', 'Pause year')
                      : tr(language, 'Simular 12 meses', 'Simulate 12 months')}
                  </span>
                </button>
                <span className="text-xs font-semibold text-[#111813]">{phenology.stageLabel}</span>
              </div>

              {currentMonth && (
                <div className="flex flex-wrap items-center gap-2 font-mono text-xs text-[#334155] tabular-nums">
                  <span>{currentMonth.precip_mm_month} mm lluvia</span>
                  <span>· ET {currentMonth.evapotranspiration_mm_month} mm</span>
                  <span>
                    · {Math.round(currentMonth.temp_min_c)}°/{Math.round(currentMonth.temp_max_c)}°C
                  </span>
                </div>
              )}
            </div>

            <div className="grid grid-cols-6 gap-1 sm:grid-cols-12">
              {monthly.map((m, idx) => {
                const isSelected = idx === monthIdx;
                const stressCls = m.frost_risk
                  ? 'border-[#bfdbfe] bg-[#dbeafe] text-[#1e3a8a]'
                  : m.heat_stress
                    ? 'border-[#fed7aa] bg-[#ffedd5] text-[#9a3412]'
                    : m.dry_stress
                      ? 'border-[#fde68a] bg-[#fef3c7] text-[#92400e]'
                      : 'border-[#bbf7d0] bg-[#dcfce7] text-[#14532d]';
                return (
                  <button
                    key={m.month}
                    type="button"
                    onClick={() => {
                      setIsPlaying(false);
                      setMonthIdx(idx);
                    }}
                    className={`rounded-md border px-1 py-1.5 text-center font-mono text-xs font-bold transition-all cursor-pointer ${
                      isSelected
                        ? 'border-[#111813] bg-[#1b4332] text-white ring-2 ring-[#1b4332]/25'
                        : `${stressCls} hover:opacity-90`
                    }`}
                  >
                    {m.month_label}
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        {/* Right Column: Drawn Polygon Metrics + Seed Plan & Furrow Layout */}
        <div className="space-y-3">
          {/* Polygon Identity & Draw Trigger */}
          <div className="rounded-xl border border-[#e3e6e1] bg-[#fafaf8] p-3.5 space-y-2.5">
            <div className="flex items-start justify-between gap-2">
              <div>
                <h3 className="flex items-center gap-1.5 text-sm font-bold text-[#111813]">
                  <Layers className="h-4 w-4 text-[#1b4332]" />
                  <span>{tr(language, 'Geometría y semillas del lote', 'Field geometry & seed plan')}</span>
                </h3>
                <p className="mt-0.5 text-xs text-[#475569]">
                  {hasCustomPolygon
                    ? tr(language, 'Basado en el polígono dibujado en el mapa.', 'Computed from your drawn map polygon.')
                    : tr(language, 'Área base estimada — dibuja tu parcela para precisión exacta.', 'Baseline area — draw polygon on map for exact sizing.')}
                </p>
              </div>
              <Button type="button" size="sm" variant="outline" onClick={onRequestDrawPolygon}>
                <PenTool className="h-3.5 w-3.5" />
                <span>{hasCustomPolygon ? tr(language, 'Redibujar', 'Redraw') : tr(language, 'Dibujar', 'Draw')}</span>
              </Button>
            </div>

            <div className="grid grid-cols-3 gap-2 text-xs">
              <div className="rounded-lg border border-[#e3e6e1] bg-white px-2.5 py-2">
                <span className="block text-[11px] text-[#475569]">{tr(language, 'Área', 'Area')}</span>
                <strong className="font-mono text-sm font-bold text-[#111813] tabular-nums">
                  {areaHa.toFixed(2)} ha
                </strong>
              </div>
              <div className="rounded-lg border border-[#e3e6e1] bg-white px-2.5 py-2">
                <span className="block text-[11px] text-[#475569]">{tr(language, 'Perímetro', 'Perimeter')}</span>
                <strong className="font-mono text-sm font-bold text-[#111813] tabular-nums">
                  {perimeterM} m
                </strong>
              </div>
              <div className="rounded-lg border border-[#e3e6e1] bg-white px-2.5 py-2">
                <span className="block text-[11px] text-[#475569]">{tr(language, 'Pendiente', 'Slope')}</span>
                <strong className="font-mono text-sm font-bold text-[#111813] tabular-nums">
                  {slopePct.toFixed(1)}%
                </strong>
              </div>
            </div>

            {elevationM !== null && (
              <p className="flex items-center gap-1.5 text-xs text-[#475569]">
                <Mountain className="h-3.5 w-3.5 shrink-0 text-[#1b4332]" />
                <span>
                  {tr(language, 'Altitud NASA SRTM:', 'NASA SRTM elevation:')}{' '}
                  <strong className="font-mono text-[#111813]">{elevationM} m s. n. m.</strong>
                </span>
              </p>
            )}
          </div>

          {/* Seed & Furrow Cards for Each Step of Selected Strategy */}
          <div className="rounded-xl border border-[#e3e6e1] bg-white p-3.5 space-y-2.5">
            <div className="flex items-center justify-between gap-2">
              <h3 className="flex items-center gap-1.5 text-sm font-bold text-[#111813]">
                <Sprout className="h-4 w-4 text-[#15803d]" />
                <span>
                  {tr(
                    language,
                    `Semillas necesarias (${areaHa.toFixed(2)} ha)`,
                    `Required seeds (${areaHa.toFixed(2)} ha)`,
                  )}
                </span>
              </h3>
              <span className="rounded bg-[#dcfce7] px-2 py-0.5 font-mono text-[11px] font-semibold text-[#14532d]">
                {seedPlans.length} {tr(language, 'etapas', 'stages')}
              </span>
            </div>

            <div className="space-y-2">
              {seedPlans.map((sp, idx) => {
                const isCurrentVisual = sp.cropTitle === activeSeedPlan.cropTitle;
                return (
                  <div
                    key={`${sp.cropTitle}-${idx}`}
                    className={`rounded-lg border p-2.5 transition-colors ${
                      isCurrentVisual
                        ? 'border-[#1b4332] bg-[#f4fbf6]'
                        : 'border-[#e3e6e1] bg-[#fafaf8]'
                    }`}
                  >
                    <div className="flex flex-wrap items-center justify-between gap-1">
                      <span className="text-xs font-bold text-[#111813]">{sp.cropTitle}</span>
                      <span className="font-mono text-xs font-bold text-[#1b4332] tabular-nums">
                        {sp.totalSeedKg.toLocaleString()} kg ({sp.sackLabel})
                      </span>
                    </div>
                    <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 font-mono text-[11px] text-[#475569] tabular-nums">
                      <span>
                        {tr(language, 'Dosis:', 'Rate:')} {sp.seedRateKgHa} kg/ha
                      </span>
                      <span>
                        {tr(language, 'Surcos:', 'Rows:')} {sp.rowSpacingCm} cm × {sp.plantSpacingCm} cm
                      </span>
                      {viewMode === 'agronomist' && (
                        <span>~{sp.estimatedPlants.toLocaleString()} {tr(language, 'plantas', 'plants')}</span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>

            <p className="flex items-start gap-1.5 rounded-lg border border-[#e3e6e1] bg-[#fafaf8] p-2.5 text-xs leading-snug text-[#334155]">
              <Compass className="mt-0.5 h-3.5 w-3.5 shrink-0 text-[#1b4332]" />
              <span>{activeSeedPlan.furrowAdvice}</span>
            </p>
          </div>

          {/* Active Month Weather Status Pill */}
          {currentMonth && (
            <div className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-[#e3e6e1] bg-[#fafaf8] px-3.5 py-2.5 text-xs">
              <span className="inline-flex items-center gap-1.5 font-semibold text-[#111813]">
                {currentMonth.frost_risk ? (
                  <Snowflake className="h-4 w-4 text-[#1d4ed8]" />
                ) : currentMonth.dry_stress || currentMonth.heat_stress ? (
                  <Thermometer className="h-4 w-4 text-[#b45309]" />
                ) : (
                  <Droplets className="h-4 w-4 text-[#15803d]" />
                )}
                {currentMonth.frost_risk
                  ? tr(language, 'Alerta helada (<2 °C): proteger siembra', 'Frost alert (<2 °C): protect crop')
                  : currentMonth.dry_stress
                    ? tr(language, 'Estrés seco: suelo requiere cobertura', 'Dry stress: cover protects soil moisture')
                    : currentMonth.heat_stress
                      ? tr(language, 'Calor alto: mayor demanda de agua', 'Heat stress: higher ET water demand')
                      : tr(language, 'Mes favorable para crecimiento activo', 'Favorable month for active growth')}
              </span>
              <span className="font-mono font-semibold text-[#475569] tabular-nums">
                {tr(language, 'Bal:', 'Bal:')} {currentMonth.water_balance_mm > 0 ? '+' : ''}
                {Math.round(currentMonth.water_balance_mm)} mm
              </span>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
