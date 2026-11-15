import { useEffect, useRef, useState } from 'react';
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';
import {
  Check,
  ChevronDown,
  ChevronUp,
  CircleDot,
  CloudRain,
  Droplets,
  Layers,
  Map as MapIcon,
  Mountain,
  PenTool,
  Plus,
  RotateCcw,
  Snowflake,
  Sprout,
  Trash2,
  X,
} from 'lucide-react';
import { tr } from '../i18n';
import {
  computePolygonAreaHa,
  computePolygonCentroid,
  computePolygonPerimeterM,
  getEffectivePolygon,
} from '../lib/parcelGeometry';
import type { CropKey, Language, Parcel, ParcelDiagnosis } from '../types';
import { Button } from './ui/button';

import markerIcon2x from 'leaflet/dist/images/marker-icon-2x.png';
import markerIcon from 'leaflet/dist/images/marker-icon.png';
import markerShadow from 'leaflet/dist/images/marker-shadow.png';

L.Icon.Default.mergeOptions({
  iconRetinaUrl: markerIcon2x,
  iconUrl: markerIcon,
  shadowUrl: markerShadow,
});

type BaseMapKey = 'osm' | 'satellite' | 'topo';
type NasaFilterKey = 'none' | 'precip' | 'soil' | 'lst' | 'ndvi' | 'water';
type GibsMosaicDate = '2026-07-01' | '2026-02-15' | '2025-07-01';

interface ParcelMapProps {
  lat: number;
  lon: number;
  language: Language;
  focusZoom?: boolean;
  parcels?: Parcel[];
  focusedParcel?: Parcel | null;
  diagnosis?: ParcelDiagnosis | null;
  pendingPolygon?: [number, number][] | null;
  externalDrawTrigger?: number;
  onSelectCoords: (lat: number, lon: number) => void;
  onSelectParcel?: (id: string) => void;
  onStartNewParcelAtCoords?: () => void;
  onSaveDrawnParcel?: (
    polygon: [number, number][],
    areaHa: number,
    centroidLat: number,
    centroidLon: number,
    mode: 'create' | 'update',
    customName?: string,
    crop?: CropKey,
  ) => Promise<void> | void;
}

interface FilterOptionConfig {
  key: NasaFilterKey;
  label: string;
  shortLabel: string;
  icon: typeof MapIcon;
  color: string;
  hint: string;
  scale?: {
    gradient: string;
    labels: [string, string, string];
    unit: string;
  };
}

function getGibsLayerUrl(key: Exclude<NasaFilterKey, 'none'>, date: GibsMosaicDate): string {
  switch (key) {
    case 'precip':
      return `https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/IMERG_Precipitation_Rate/default/${date}T00:00:00Z/GoogleMapsCompatible_Level6/{z}/{y}/{x}.png`;
    case 'soil':
      return `https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/SMAP_L4_Analyzed_Root_Zone_Soil_Moisture/default/${date}/GoogleMapsCompatible_Level6/{z}/{y}/{x}.png`;
    case 'lst':
      return `https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/MODIS_Terra_Land_Surface_Temp_Night/default/${date}/GoogleMapsCompatible_Level7/{z}/{y}/{x}.png`;
    case 'ndvi':
      return `https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/MODIS_Terra_L3_NDVI_16Day/default/${date}/GoogleMapsCompatible_Level9/{z}/{y}/{x}.png`;
    case 'water':
      return `https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/MODIS_Terra_CorrectedReflectance_Bands721/default/${date}/GoogleMapsCompatible_Level9/{z}/{y}/{x}.jpg`;
  }
}

export function ParcelMap({
  lat,
  lon,
  language,
  focusZoom = false,
  parcels = [],
  focusedParcel = null,
  diagnosis = null,
  pendingPolygon = null,
  externalDrawTrigger = 0,
  onSelectCoords,
  onSelectParcel,
  onStartNewParcelAtCoords,
  onSaveDrawnParcel,
}: ParcelMapProps) {
  const [baseMap, setBaseMap] = useState<BaseMapKey>('osm');
  const [activeFilter, setActiveFilter] = useState<NasaFilterKey>('none');
  const [mosaicDate, setMosaicDate] = useState<GibsMosaicDate>('2026-07-01');
  const [layerOpacity, setLayerOpacity] = useState<number>(0.7);
  const [showHalo, setShowHalo] = useState<boolean>(true);
  const [controlsOpen, setControlsOpen] = useState<boolean>(false);
  const [isDrawing, setIsDrawing] = useState<boolean>(false);
  const [draftVertices, setDraftVertices] = useState<[number, number][]>([]);
  const [drawName, setDrawName] = useState<string>('');
  const [drawCrop, setDrawCrop] = useState<CropKey>('potato_continuous');
  const [savingPolygon, setSavingPolygon] = useState<boolean>(false);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const controlsOverlayRef = useRef<HTMLDivElement | null>(null);
  const legendOverlayRef = useRef<HTMLDivElement | null>(null);
  const drawOverlayRef = useRef<HTMLDivElement | null>(null);
  const mapRef = useRef<L.Map | null>(null);
  const pinMarkerRef = useRef<L.Marker | null>(null);
  const parcelGroupRef = useRef<L.LayerGroup | null>(null);
  const thematicGroupRef = useRef<L.LayerGroup | null>(null);
  const drawGroupRef = useRef<L.LayerGroup | null>(null);
  const baseLayersRef = useRef<Record<BaseMapKey, L.TileLayer> | null>(null);
  const overlayLayersRef = useRef<Record<Exclude<NasaFilterKey, 'none'>, L.TileLayer> | null>(null);

  const onSelectRef = useRef(onSelectCoords);
  onSelectRef.current = onSelectCoords;
  const onSelectParcelRef = useRef(onSelectParcel);
  onSelectParcelRef.current = onSelectParcel;
  const isDrawingRef = useRef(isDrawing);
  isDrawingRef.current = isDrawing;
  const lastDrawTriggerRef = useRef(externalDrawTrigger);

  useEffect(() => {
    if (!containerRef.current || mapRef.current) return;

    const osmLayer = L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
      attribution: '&copy; OpenStreetMap',
      maxZoom: 18,
    });
    const satelliteLayer = L.tileLayer(
      'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
      {
        attribution: 'Esri World Imagery',
        maxZoom: 18,
      },
    );
    const topoLayer = L.tileLayer('https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png', {
      attribution: '&copy; OpenTopoMap',
      maxZoom: 17,
    });

    const gibsPrecip = L.tileLayer(getGibsLayerUrl('precip', '2026-07-01'), {
      opacity: 0.7,
      maxNativeZoom: 6,
      maxZoom: 18,
      attribution: 'NASA GPM IMERG',
    });
    const gibsSoil = L.tileLayer(getGibsLayerUrl('soil', '2026-07-01'), {
      opacity: 0.7,
      maxNativeZoom: 6,
      maxZoom: 18,
      attribution: 'NASA SMAP L4 Soil Moisture',
    });
    const gibsLst = L.tileLayer(getGibsLayerUrl('lst', '2026-07-01'), {
      opacity: 0.7,
      maxNativeZoom: 7,
      maxZoom: 18,
      attribution: 'NASA MODIS LST Night',
    });
    const gibsNdvi = L.tileLayer(getGibsLayerUrl('ndvi', '2026-07-01'), {
      opacity: 0.7,
      maxNativeZoom: 9,
      maxZoom: 18,
      attribution: 'NASA MODIS NDVI 16-Day',
    });
    const gibsWater = L.tileLayer(getGibsLayerUrl('water', '2026-07-01'), {
      opacity: 0.7,
      maxNativeZoom: 9,
      maxZoom: 18,
      attribution: 'NASA MODIS Bands 7-2-1',
    });

    const map = L.map(containerRef.current, { layers: [osmLayer] }).setView([lat, lon], 7);
    const thematicGroup = L.layerGroup().addTo(map);
    const parcelGroup = L.layerGroup().addTo(map);
    const drawGroup = L.layerGroup().addTo(map);
    const pinMarker = L.marker([lat, lon]).addTo(map);

    map.on('click', (e: L.LeafletMouseEvent) => {
      if (isDrawingRef.current) {
        const vLat = Number(e.latlng.lat.toFixed(6));
        const vLon = Number(e.latlng.lng.toFixed(6));
        setDraftVertices((prev) => [...prev, [vLat, vLon]]);
        return;
      }
      const nextLat = Number(e.latlng.lat.toFixed(4));
      const nextLon = Number(e.latlng.lng.toFixed(4));
      onSelectRef.current(nextLat, nextLon);
    });

    mapRef.current = map;
    pinMarkerRef.current = pinMarker;
    parcelGroupRef.current = parcelGroup;
    thematicGroupRef.current = thematicGroup;
    drawGroupRef.current = drawGroup;
    baseLayersRef.current = {
      osm: osmLayer,
      satellite: satelliteLayer,
      topo: topoLayer,
    };
    overlayLayersRef.current = {
      precip: gibsPrecip,
      soil: gibsSoil,
      lst: gibsLst,
      ndvi: gibsNdvi,
      water: gibsWater,
    };

    const resizeTimer = window.setTimeout(() => {
      map.invalidateSize();
    }, 80);

    return () => {
      window.clearTimeout(resizeTimer);
      map.remove();
      mapRef.current = null;
      pinMarkerRef.current = null;
      parcelGroupRef.current = null;
      thematicGroupRef.current = null;
      drawGroupRef.current = null;
      baseLayersRef.current = null;
      overlayLayersRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (controlsOverlayRef.current) {
      L.DomEvent.disableClickPropagation(controlsOverlayRef.current);
      L.DomEvent.disableScrollPropagation(controlsOverlayRef.current);
    }
    if (legendOverlayRef.current) {
      L.DomEvent.disableClickPropagation(legendOverlayRef.current);
      L.DomEvent.disableScrollPropagation(legendOverlayRef.current);
    }
    if (drawOverlayRef.current) {
      L.DomEvent.disableClickPropagation(drawOverlayRef.current);
      L.DomEvent.disableScrollPropagation(drawOverlayRef.current);
    }
  });

  useEffect(() => {
    const map = mapRef.current;
    const bases = baseLayersRef.current;
    if (!map || !bases) return;

    (Object.keys(bases) as BaseMapKey[]).forEach((key) => {
      const layer = bases[key];
      if (key === baseMap) {
        if (!map.hasLayer(layer)) {
          layer.addTo(map);
          layer.bringToBack();
        }
      } else if (map.hasLayer(layer)) {
        map.removeLayer(layer);
      }
    });
  }, [baseMap]);

  useEffect(() => {
    const overlays = overlayLayersRef.current;
    if (!overlays) return;
    (Object.keys(overlays) as Array<Exclude<NasaFilterKey, 'none'>>).forEach((key) => {
      overlays[key].setUrl(getGibsLayerUrl(key, mosaicDate));
    });
  }, [mosaicDate]);

  useEffect(() => {
    const map = mapRef.current;
    const overlays = overlayLayersRef.current;
    if (!map || !overlays) return;

    (Object.keys(overlays) as Array<Exclude<NasaFilterKey, 'none'>>).forEach((key) => {
      const layer = overlays[key];
      if (key === activeFilter) {
        layer.setOpacity(layerOpacity);
        if (!map.hasLayer(layer)) layer.addTo(map);
      } else if (map.hasLayer(layer)) {
        map.removeLayer(layer);
      }
    });
  }, [activeFilter, layerOpacity]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.style.cursor = isDrawing ? 'crosshair' : '';
  }, [isDrawing]);

  function startDrawingPolygon() {
    setIsDrawing(true);
    setDraftVertices([]);
    setDrawName(tr(language, `Parcela ${parcels.length + 1}`, `Field ${parcels.length + 1}`));
    setDrawCrop(focusedParcel?.current_crop ?? 'potato_continuous');
    setControlsOpen(false);
    setBaseMap('satellite');
    const map = mapRef.current;
    if (map) {
      map.setView([lat, lon], Math.max(map.getZoom(), 15));
    }
  }

  function cancelDrawingPolygon() {
    setIsDrawing(false);
    setDraftVertices([]);
  }

  async function handleSaveDrawn(mode: 'create' | 'update') {
    if (draftVertices.length < 3 || savingPolygon) return;
    setSavingPolygon(true);
    try {
      const areaHa = computePolygonAreaHa(draftVertices);
      const [cLat, cLon] = computePolygonCentroid(draftVertices);
      await onSaveDrawnParcel?.(
        draftVertices,
        areaHa,
        cLat,
        cLon,
        mode,
        drawName.trim() || undefined,
        drawCrop,
      );
      setIsDrawing(false);
      setDraftVertices([]);
    } finally {
      setSavingPolygon(false);
    }
  }

  useEffect(() => {
    if (externalDrawTrigger > 0 && externalDrawTrigger !== lastDrawTriggerRef.current) {
      lastDrawTriggerRef.current = externalDrawTrigger;
      startDrawingPolygon();
    }
  }, [externalDrawTrigger]);

  useEffect(() => {
    const group = drawGroupRef.current;
    if (!group) return;
    group.clearLayers();
    if (!isDrawing || draftVertices.length === 0) return;

    if (draftVertices.length >= 3) {
      L.polygon(draftVertices, {
        color: '#d97706',
        weight: 2.5,
        fillColor: '#fde047',
        fillOpacity: 0.28,
        dashArray: '6 4',
        interactive: false,
      }).addTo(group);
    } else if (draftVertices.length === 2) {
      L.polyline(draftVertices, {
        color: '#d97706',
        weight: 2.5,
        dashArray: '6 4',
        interactive: false,
      }).addTo(group);
    }

    draftVertices.forEach((pt, idx) => {
      L.circleMarker(pt, {
        radius: idx === 0 ? 7 : 5,
        color: '#92400e',
        weight: 2,
        fillColor: idx === 0 ? '#f59e0b' : '#fef3c7',
        fillOpacity: 1,
        interactive: false,
      })
        .bindTooltip(`${idx + 1}`, { permanent: true, direction: 'top', offset: [0, -6] })
        .addTo(group);
    });
  }, [draftVertices, isDrawing]);

  useEffect(() => {
    const group = parcelGroupRef.current;
    if (!group) return;
    group.clearLayers();

    const focusedId = focusedParcel?.id ?? null;

    parcels.forEach((p) => {
      const isFocused = p.id === focusedId;
      const polyVerts = getEffectivePolygon(p, p.lat, p.lon);
      const areaHa = p.area_ha ?? computePolygonAreaHa(polyVerts);

      const poly = L.polygon(polyVerts, {
        color: isFocused ? '#14532d' : '#1b4332',
        weight: isFocused ? 2.5 : 1.5,
        fillColor: isFocused ? '#22c55e' : '#86efac',
        fillOpacity: isFocused ? 0.28 : 0.14,
        dashArray: p.polygon && p.polygon.length >= 3 ? undefined : '5 4',
        interactive: !isDrawing,
      });
      poly.bindTooltip(`${p.name} · ${areaHa} ha`, {
        direction: 'top',
        sticky: true,
      });
      poly.on('click', (e: L.LeafletMouseEvent) => {
        if (isDrawingRef.current) return;
        L.DomEvent.stopPropagation(e);
        onSelectParcelRef.current?.(p.id);
      });
      poly.addTo(group);

      const circle = L.circleMarker([p.lat, p.lon], {
        radius: isFocused ? 8 : 6,
        color: isFocused ? '#143326' : '#1b4332',
        weight: isFocused ? 3 : 2,
        fillColor: isFocused ? '#1b4332' : '#dcfce7',
        fillOpacity: isFocused ? 0.95 : 0.9,
        interactive: !isDrawing,
      });
      circle.bindTooltip(`${p.name} · ${areaHa} ha`, {
        direction: 'top',
        offset: [0, -8],
      });
      circle.on('click', (e: L.LeafletMouseEvent) => {
        if (isDrawingRef.current) return;
        L.DomEvent.stopPropagation(e);
        onSelectParcelRef.current?.(p.id);
      });
      circle.addTo(group);
    });

    if (pendingPolygon && pendingPolygon.length >= 3) {
      L.polygon(pendingPolygon, {
        color: '#d97706',
        weight: 2.5,
        fillColor: '#fde047',
        fillOpacity: 0.24,
        dashArray: '5 4',
        interactive: false,
      }).addTo(group);
    }
  }, [parcels, focusedParcel?.id, isDrawing, pendingPolygon]);

  useEffect(() => {
    const map = mapRef.current;
    const marker = pinMarkerRef.current;
    if (!map || !marker) return;
    marker.setLatLng([lat, lon]);
    if (focusZoom && !isDrawingRef.current) {
      map.setView([lat, lon], Math.max(map.getZoom(), 14));
    }
  }, [lat, lon, focusZoom]);

  const filterOptions: FilterOptionConfig[] = [
    {
      key: 'none',
      label: tr(language, 'Sin capa NASA (Solo mapa base)', 'No NASA layer (Base map only)'),
      shortLabel: tr(language, 'Solo mapa', 'Base map'),
      icon: MapIcon,
      color: '#1b4332',
      hint: tr(
        language,
        'Haz clic en el mapa para fijar coordenadas o elige una capa satelital NASA arriba.',
        'Click the map to set coordinates or pick a NASA satellite overlay above.',
      ),
    },
    {
      key: 'precip',
      label: tr(language, 'Lluvia satelital (GPM IMERG)', 'Satellite rain (GPM IMERG)'),
      shortLabel: tr(language, 'Lluvia GPM', 'Rain GPM'),
      icon: CloudRain,
      color: '#2563eb',
      hint: tr(
        language,
        'IMERG: tasa de lluvia medida por constelación satelital GPM en la fecha del mosaico.',
        'IMERG: GPM satellite constellation rainfall rate for the selected mosaic date.',
      ),
      scale: {
        gradient: 'linear-gradient(90deg, #e0f2fe 0%, #38bdf8 35%, #2563eb 70%, #7e22ce 100%)',
        labels: ['0 mm/h', '5 mm/h', '15+ mm/h'],
        unit: 'IMERG',
      },
    },
    {
      key: 'soil',
      label: tr(language, 'Humedad del suelo (NASA SMAP L4)', 'Soil moisture (NASA SMAP L4)'),
      shortLabel: tr(language, 'Humedad SMAP', 'SMAP Moisture'),
      icon: Sprout,
      color: '#0d9488',
      hint: tr(
        language,
        'SMAP L4: cuánta agua retiene la tierra en zona de raíces (0–100 cm) según radar NASA.',
        'SMAP L4: root-zone water retention (0–100 cm) estimated by NASA radar.',
      ),
      scale: {
        gradient: 'linear-gradient(90deg, #fef3c7 0%, #99f6e4 45%, #0d9488 75%, #115e59 100%)',
        labels: [
          tr(language, '15% Seco', '15% Dry'),
          tr(language, '45% Óptimo', '45% Optimal'),
          tr(language, '80%+ Húmedo', '80%+ Wet'),
        ],
        unit: 'SMAP L4',
      },
    },
    {
      key: 'lst',
      label: tr(language, 'Heladas / Calor nocturno (MODIS LST)', 'Night Temp / Frost (MODIS LST)'),
      shortLabel: tr(language, 'Temp. MODIS', 'Temp. MODIS'),
      icon: Snowflake,
      color: '#ea580c',
      hint: tr(
        language,
        'LST: temperatura superficial nocturna útil para detectar bolsillos de helada.',
        'LST: nighttime land surface temperature indicating frost pockets or heat.',
      ),
      scale: {
        gradient: 'linear-gradient(90deg, #1d4ed8 0%, #06b6d4 35%, #f59e0b 70%, #dc2626 100%)',
        labels: [
          tr(language, '-10 °C Helada', '-10 °C Frost'),
          '+10 °C',
          tr(language, '+25 °C Cálido', '+25 °C Warm'),
        ],
        unit: 'LST',
      },
    },
    {
      key: 'ndvi',
      label: tr(language, 'Verdor vegetal (MODIS NDVI 16d)', 'Vegetation greenness (MODIS NDVI)'),
      shortLabel: tr(language, 'Verdor NDVI', 'Greenness NDVI'),
      icon: Sprout,
      color: '#15803d',
      hint: tr(
        language,
        'NDVI: qué tan cubierto y verde está el cultivo/pasto (mosaico 16 días) frente a suelo desnudo.',
        'NDVI: crop/pasture vegetative cover (16-day mosaic) versus bare ground.',
      ),
      scale: {
        gradient: 'linear-gradient(90deg, #b45309 0%, #eab308 35%, #84cc16 65%, #15803d 100%)',
        labels: [
          tr(language, '0.0 Desnudo', '0.0 Bare'),
          tr(language, '0.4 Medio', '0.4 Moderate'),
          tr(language, '0.8+ Denso', '0.8+ Dense'),
        ],
        unit: 'NDVI',
      },
    },
    {
      key: 'water',
      label: tr(language, 'Agua y suelos húmedos (Bandas 7-2-1)', 'Water & wet soils (Bands 7-2-1)'),
      shortLabel: tr(language, 'Agua 7-2-1', 'Water 7-2-1'),
      icon: Droplets,
      color: '#4f46e5',
      hint: tr(
        language,
        'Combinación infrarroja 7-2-1 para distinguir agua superficial, humedad y suelo seco.',
        'SWIR-NIR Bands 7-2-1 highlighting surface water, moist soil, and dry land.',
      ),
      scale: {
        gradient: 'linear-gradient(90deg, #dc2626 0%, #f59e0b 40%, #38bdf8 70%, #1e3a8a 100%)',
        labels: [
          tr(language, 'Seco', 'Dry'),
          tr(language, 'Equilibrio', 'Balanced'),
          tr(language, 'Húmedo', 'Wet'),
        ],
        unit: '7-2-1',
      },
    },
  ];

  const activeConfig = filterOptions.find((opt) => opt.key === activeFilter) ?? filterOptions[0];
  const ActiveFilterIcon = activeConfig.icon;

  const fieldReadout = (() => {
    if (!diagnosis || activeFilter === 'none') return null;
    switch (activeFilter) {
      case 'precip':
        return tr(
          language,
          `Lluvia 30a: ${diagnosis.annual_precip_mm} mm/año · ${diagnosis.dry_months.length}m secos`,
          `30y Rain: ${diagnosis.annual_precip_mm} mm/yr · ${diagnosis.dry_months.length} dry mo`,
        );
      case 'soil':
        return tr(
          language,
          `Raíz: ${Math.round(diagnosis.avg_soil_moisture_root * 100)}% · Sup: ${Math.round(diagnosis.avg_soil_moisture_top * 100)}%`,
          `Root: ${Math.round(diagnosis.avg_soil_moisture_root * 100)}% · Top: ${Math.round(diagnosis.avg_soil_moisture_top * 100)}%`,
        );
      case 'lst':
        return tr(
          language,
          `T media: ${diagnosis.annual_temp_mean_c} °C · Heladas: ${diagnosis.frost_risk_months.length}m`,
          `Mean T: ${diagnosis.annual_temp_mean_c} °C · Frost: ${diagnosis.frost_risk_months.length}m`,
        );
      case 'ndvi': {
        const ndviVal = diagnosis.historical_correlation?.latest_ndvi;
        const ndviDate = diagnosis.historical_correlation?.latest_ndvi_date;
        return tr(
          language,
          `Sentinel-2 NDVI: ${ndviVal !== null && ndviVal !== undefined ? ndviVal.toFixed(2) : 'N/D'}${ndviDate ? ` (${ndviDate})` : ''}`,
          `Sentinel-2 NDVI: ${ndviVal !== null && ndviVal !== undefined ? ndviVal.toFixed(2) : 'N/A'}${ndviDate ? ` (${ndviDate})` : ''}`,
        );
      }
      case 'water': {
        const bal = diagnosis.annual_water_balance_mm;
        const balText = bal > 0 ? `+${bal}` : `${bal}`;
        return tr(
          language,
          `Balance hídrico: ${balText} mm · ET: ${diagnosis.annual_et_mm} mm`,
          `Water balance: ${balText} mm · ET: ${diagnosis.annual_et_mm} mm`,
        );
      }
      default:
        return null;
    }
  })();

  useEffect(() => {
    const group = thematicGroupRef.current;
    if (!group) return;
    group.clearLayers();

    if (!showHalo || activeFilter === 'none') return;

    const ring = L.circle([lat, lon], {
      radius: 9000,
      color: activeConfig.color,
      weight: 2.5,
      fillColor: activeConfig.color,
      fillOpacity: 0.16,
      dashArray: activeFilter === 'water' ? '6 4' : undefined,
      interactive: false,
    });

    if (fieldReadout) {
      ring.bindTooltip(fieldReadout, {
        permanent: true,
        direction: 'right',
        offset: [14, 0],
      });
    }

    ring.addTo(group);
  }, [lat, lon, activeFilter, activeConfig.color, showHalo, fieldReadout]);

  const isCustomPin =
    !focusedParcel ||
    Math.abs(focusedParcel.lat - lat) > 0.0005 ||
    Math.abs(focusedParcel.lon - lon) > 0.0005;

  const stopMapPropagation = (e: React.SyntheticEvent) => {
    e.stopPropagation();
  };

  const selectClassName =
    'h-8 w-full cursor-pointer rounded-md border border-[#d6dad3] bg-white px-2 text-xs font-medium text-[#111813] focus:border-[#1b4332] focus:outline-none focus:ring-2 focus:ring-[#1b4332]/15';

  return (
    <section
      aria-label={tr(language, 'Mapa satelital de la parcela', 'Satellite parcel map')}
      className="flex h-full flex-col justify-between rounded-xl border border-[#e3e6e1] bg-white p-4 sm:p-5"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-base font-bold text-[#111813] sm:text-lg">
            {tr(
              language,
              'Mapa Satelital de la Parcela (NASA GIBS + SRTM)',
              'Parcel Satellite Map (NASA GIBS + SRTM)',
            )}
          </h2>
          <p className="mt-0.5 text-xs text-[#475569]">
            {focusedParcel && !isCustomPin
              ? `${tr(language, 'Parcela:', 'Field:')} ${focusedParcel.name}`
              : tr(
                  language,
                  'Pin personalizado — haz clic en "Crear parcela" para guardarlo.',
                  'Custom pin — click "Create field" to save it.',
                )}
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <span className="rounded-md border border-[#e3e6e1] bg-[#fafaf8] px-2.5 py-1 font-mono text-xs font-medium text-[#111813] tabular-nums">
            {lat.toFixed(4)}, {lon.toFixed(4)}
          </span>

          {diagnosis?.terrain.live_terrain_data && (
            <span className="inline-flex items-center gap-1 rounded-md border border-[#e3e6e1] bg-[#fafaf8] px-2.5 py-1 font-mono text-xs text-[#111813] tabular-nums">
              <Mountain className="h-3.5 w-3.5 text-[#1b4332]" />
              {diagnosis.terrain.elevation_m} m · {diagnosis.terrain.slope_pct}%
            </span>
          )}

          {onSaveDrawnParcel && (
            <Button
              type="button"
              size="sm"
              variant={isDrawing ? 'default' : 'outline'}
              onClick={() => (isDrawing ? cancelDrawingPolygon() : startDrawingPolygon())}
            >
              <PenTool className="h-3.5 w-3.5" />
              <span>
                {isDrawing
                  ? tr(language, 'Dibujando...', 'Drawing...')
                  : tr(language, 'Dibujar linderos', 'Draw boundary')}
              </span>
            </Button>
          )}

          {onStartNewParcelAtCoords && (
            <Button
              type="button"
              size="sm"
              variant={isCustomPin ? 'default' : 'outline'}
              onClick={onStartNewParcelAtCoords}
            >
              <Plus className="h-3.5 w-3.5" />
              <span>{tr(language, 'Crear parcela', 'Create field')}</span>
            </Button>
          )}
        </div>
      </div>

      {/* Map canvas with in-map floating Layer/Filter selector, Polygon Draw bar & explicit date legend */}
      <div className="map-shell mt-3 flex-1">
        <div id="map" ref={containerRef} />

        {/* In-map top-left drawing bar when tracing parcel boundaries */}
        {isDrawing && (
          <div
            ref={drawOverlayRef}
            onClick={stopMapPropagation}
            onMouseDown={stopMapPropagation}
            onDoubleClick={stopMapPropagation}
            className="absolute left-12 top-2.5 right-2.5 z-[1001] max-w-[540px] rounded-lg border border-[#fde68a] bg-white/95 p-2.5 shadow-md backdrop-blur-xs space-y-2"
          >
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="inline-flex items-center gap-1.5 text-xs font-bold text-[#111813]">
                <PenTool className="h-3.5 w-3.5 text-[#b45309]" />
                {tr(
                  language,
                  'Toca las esquinas de tu parcela en el satélite',
                  'Click your field corners on satellite view',
                )}
              </span>
              <span className="rounded bg-[#fffbeb] px-2 py-0.5 font-mono text-xs font-semibold text-[#92400e] tabular-nums">
                {draftVertices.length} pts ·{' '}
                {draftVertices.length >= 3
                  ? `${computePolygonAreaHa(draftVertices).toFixed(2)} ha`
                  : '0.00 ha'}{' '}
                · {computePolygonPerimeterM(draftVertices)} m
              </span>
            </div>

            <div className="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
              <input
                id="draw-parcel-name"
                type="text"
                maxLength={80}
                value={drawName}
                onChange={(e) => setDrawName(e.target.value)}
                placeholder={tr(language, 'Nombre de la parcela', 'Field name')}
                className="h-8 rounded-md border border-[#d6dad3] bg-white px-2.5 text-xs font-medium text-[#111813] focus:border-[#1b4332] focus:outline-none focus:ring-2 focus:ring-[#1b4332]/15"
              />
              <select
                id="draw-parcel-crop"
                value={drawCrop}
                onChange={(e) => setDrawCrop(e.target.value as CropKey)}
                aria-label={tr(language, 'Cultivo inicial', 'Current crop')}
                className={selectClassName}
              >
                <option value="potato_continuous">{tr(language, 'Cultivo: Papa', 'Crop: Potato')}</option>
                <option value="maize_continuous">{tr(language, 'Cultivo: Maíz', 'Crop: Maize')}</option>
                <option value="rice_continuous">{tr(language, 'Cultivo: Arroz', 'Crop: Rice')}</option>
                <option value="wheat_continuous">{tr(language, 'Cultivo: Trigo / cereal', 'Crop: Wheat')}</option>
                <option value="mixed_vegetables">{tr(language, 'Cultivo: Hortalizas', 'Crop: Vegetables')}</option>
              </select>
            </div>

            <div className="flex flex-wrap items-center gap-1.5">
              <button
                type="button"
                disabled={draftVertices.length === 0 || savingPolygon}
                onClick={() => setDraftVertices((prev) => prev.slice(0, -1))}
                className="inline-flex items-center gap-1 rounded border border-[#e3e6e1] bg-white px-2 py-1 text-xs font-semibold text-[#111813] hover:bg-[#fafaf8] disabled:opacity-40 cursor-pointer"
              >
                <RotateCcw className="h-3 w-3" />
                {tr(language, 'Deshacer', 'Undo')}
              </button>
              <button
                type="button"
                disabled={draftVertices.length === 0 || savingPolygon}
                onClick={() => setDraftVertices([])}
                className="inline-flex items-center gap-1 rounded border border-[#e3e6e1] bg-white px-2 py-1 text-xs font-semibold text-[#111813] hover:bg-[#fafaf8] disabled:opacity-40 cursor-pointer"
              >
                <Trash2 className="h-3 w-3" />
                {tr(language, 'Limpiar', 'Clear')}
              </button>
              <button
                type="button"
                disabled={savingPolygon}
                onClick={cancelDrawingPolygon}
                className="inline-flex items-center gap-1 rounded border border-[#e3e6e1] bg-white px-2 py-1 text-xs font-semibold text-[#475569] hover:bg-[#fee2e2] hover:text-[#991b1b] disabled:opacity-40 cursor-pointer"
              >
                <X className="h-3 w-3" />
                {tr(language, 'Cancelar', 'Cancel')}
              </button>

              <div className="ml-auto flex flex-wrap items-center gap-1.5">
                {focusedParcel && (
                  <button
                    type="button"
                    disabled={draftVertices.length < 3 || savingPolygon}
                    onClick={() => void handleSaveDrawn('update')}
                    className="inline-flex items-center gap-1 rounded border border-[#1b4332] bg-white px-2.5 py-1 text-xs font-bold text-[#1b4332] hover:bg-[#e6f4ea] disabled:opacity-40 cursor-pointer"
                  >
                    <Check className="h-3.5 w-3.5" />
                    <span className="max-w-[140px] truncate">
                      {tr(language, `Actualizar "${focusedParcel.name}"`, `Update "${focusedParcel.name}"`)}
                    </span>
                  </button>
                )}

                <button
                  type="button"
                  disabled={draftVertices.length < 3 || savingPolygon}
                  onClick={() => void handleSaveDrawn('create')}
                  className="inline-flex items-center gap-1 rounded bg-[#1b4332] px-2.5 py-1 text-xs font-bold text-white hover:bg-[#143326] disabled:opacity-40 cursor-pointer"
                >
                  <Check className="h-3.5 w-3.5" />
                  {savingPolygon
                    ? tr(language, 'Guardando...', 'Saving...')
                    : tr(
                        language,
                        `Guardar nueva parcela (${draftVertices.length >= 3 ? computePolygonAreaHa(draftVertices).toFixed(2) : '0.00'} ha)`,
                        `Save new field (${draftVertices.length >= 3 ? computePolygonAreaHa(draftVertices).toFixed(2) : '0.00'} ha)`,
                      )}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* In-map top-right collapsible control box */}
        <div
          ref={controlsOverlayRef}
          onClick={stopMapPropagation}
          onMouseDown={stopMapPropagation}
          onDoubleClick={stopMapPropagation}
          className={`absolute right-2.5 top-2.5 z-[1000] rounded-lg border border-[#d6dad3] bg-white/95 shadow-md backdrop-blur-xs ${
            controlsOpen ? 'w-[236px] sm:w-[258px]' : 'w-auto max-w-[calc(100%-20px)]'
          }`}
        >
          <button
            type="button"
            aria-expanded={controlsOpen}
            onClick={() => setControlsOpen((prev) => !prev)}
            className="flex w-full cursor-pointer items-center justify-between gap-2 rounded-lg px-3 py-2 text-left text-xs font-semibold text-[#111813] hover:bg-[#fafaf8]"
          >
            <span className="flex min-w-0 items-center gap-1.5">
              <Layers className="h-3.5 w-3.5 shrink-0 text-[#1b4332]" />
              <span className="truncate">
                {controlsOpen
                  ? tr(language, 'Capas y fecha NASA GIBS', 'NASA GIBS layers & date')
                  : activeFilter === 'none'
                    ? tr(language, 'Capas y filtros NASA', 'NASA layers & filters')
                    : `${activeConfig.shortLabel} · ${mosaicDate}`}
              </span>
            </span>
            {controlsOpen ? (
              <ChevronUp className="h-3.5 w-3.5 shrink-0 text-[#526056]" />
            ) : (
              <ChevronDown className="h-3.5 w-3.5 shrink-0 text-[#526056]" />
            )}
          </button>

          {controlsOpen && (
            <div className="space-y-2 border-t border-[#e3e6e1] px-3 pb-3 pt-2">
              <div>
                <label
                  htmlFor="basemap-select"
                  className="mb-1 block text-xs font-semibold text-[#334155]"
                >
                  {tr(language, 'Mapa base', 'Base map')}
                </label>
                <select
                  id="basemap-select"
                  value={baseMap}
                  onChange={(e) => setBaseMap(e.target.value as BaseMapKey)}
                  className={selectClassName}
                >
                  <option value="osm">{tr(language, 'Callejero (OpenStreetMap)', 'Street map (OSM)')}</option>
                  <option value="satellite">{tr(language, 'Satélite óptico (Esri HD)', 'Optical satellite (Esri)')}</option>
                  <option value="topo">{tr(language, 'Relieve topográfico', 'Topographic relief')}</option>
                </select>
              </div>

              <div>
                <label
                  htmlFor="nasa-filter-select"
                  className="mb-1 block text-xs font-semibold text-[#334155]"
                >
                  {tr(language, 'Capa satelital NASA GIBS', 'NASA GIBS satellite layer')}
                </label>
                <select
                  id="nasa-filter-select"
                  value={activeFilter}
                  onChange={(e) => setActiveFilter(e.target.value as NasaFilterKey)}
                  className={selectClassName}
                >
                  {filterOptions.map((opt) => (
                    <option key={opt.key} value={opt.key}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label
                  htmlFor="gibs-date-select"
                  className="mb-1 block text-xs font-semibold text-[#334155]"
                >
                  {tr(language, 'Fecha de imagen GIBS', 'GIBS mosaic date')}
                </label>
                <select
                  id="gibs-date-select"
                  value={mosaicDate}
                  disabled={activeFilter === 'none'}
                  onChange={(e) => setMosaicDate(e.target.value as GibsMosaicDate)}
                  className={`${selectClassName} disabled:cursor-not-allowed disabled:opacity-50`}
                >
                  <option value="2026-07-01">
                    {tr(language, '2026-07-01 · Estiaje 2026 (reciente)', '2026-07-01 · Dry season 2026')}
                  </option>
                  <option value="2026-02-15">
                    {tr(language, '2026-02-15 · Lluvias 2026', '2026-02-15 · Rainy season 2026')}
                  </option>
                  <option value="2025-07-01">
                    {tr(language, '2025-07-01 · Referencia 2025', '2025-07-01 · Reference 2025')}
                  </option>
                </select>
              </div>

              <div className="grid grid-cols-2 gap-1.5 pt-0.5">
                <div>
                  <label
                    htmlFor="layer-opacity-select"
                    className="mb-1 block text-xs font-semibold text-[#334155]"
                  >
                    {tr(language, 'Opacidad', 'Opacity')}
                  </label>
                  <select
                    id="layer-opacity-select"
                    value={String(layerOpacity)}
                    disabled={activeFilter === 'none'}
                    onChange={(e) => setLayerOpacity(Number(e.target.value))}
                    className={`${selectClassName} h-8 disabled:cursor-not-allowed disabled:opacity-50`}
                  >
                    <option value="0.45">45%</option>
                    <option value="0.7">70%</option>
                    <option value="0.9">90%</option>
                  </select>
                </div>

                <div className="flex flex-col justify-end">
                  <button
                    type="button"
                    aria-pressed={showHalo}
                    disabled={activeFilter === 'none'}
                    onClick={() => setShowHalo((prev) => !prev)}
                    className={`inline-flex h-8 w-full items-center justify-center gap-1 rounded-md border px-2 text-xs font-medium transition-colors cursor-pointer disabled:cursor-not-allowed disabled:opacity-50 ${
                      showHalo && activeFilter !== 'none'
                        ? 'border-[#1b4332] bg-[#1b4332] text-white'
                        : 'border-[#d6dad3] bg-white text-[#111813] hover:bg-[#e6f4ea]'
                    }`}
                  >
                    <CircleDot className="h-3.5 w-3.5 shrink-0" />
                    <span>{tr(language, 'Halo', 'Halo')}</span>
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* In-map bottom-left readout & chromatic legend with explicit GIBS image date */}
        <div
          ref={legendOverlayRef}
          onClick={stopMapPropagation}
          onMouseDown={stopMapPropagation}
          onDoubleClick={stopMapPropagation}
          className={`absolute bottom-6 left-2.5 z-[1000] max-w-[calc(100%-20px)] rounded-lg border border-[#d6dad3] bg-white/95 shadow-md backdrop-blur-xs sm:bottom-3 ${
            activeFilter === 'none' ? 'px-2.5 py-1.5' : 'px-3 py-2 sm:max-w-[340px]'
          }`}
        >
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="inline-flex items-center gap-1 text-xs font-bold text-[#111813]">
              <ActiveFilterIcon className="h-3.5 w-3.5 text-[#1b4332]" />
              <span>{activeConfig.shortLabel}</span>
            </span>
            {activeFilter !== 'none' && (
              <span className="rounded bg-[#fafaf8] px-1.5 py-0.5 font-mono text-[11px] font-semibold text-[#334155]">
                {tr(language, 'Mosaico:', 'Mosaic:')} {mosaicDate}
              </span>
            )}
            {fieldReadout && (
              <span className="rounded bg-[#e6f4ea] px-1.5 py-0.5 font-mono text-[11px] font-semibold text-[#1b4332] tabular-nums">
                {fieldReadout}
              </span>
            )}
          </div>

          {activeFilter !== 'none' && (
            <p className="mt-1 text-xs leading-snug text-[#475569]">{activeConfig.hint}</p>
          )}

          {activeConfig.scale && (
            <div className="mt-2">
              <div className="flex items-center justify-between font-mono text-[11px] font-semibold text-[#111813]">
                <span>{activeConfig.scale.labels[0]}</span>
                <span>{activeConfig.scale.labels[1]}</span>
                <span>{activeConfig.scale.labels[2]}</span>
              </div>
              <div
                className="mt-1 h-2 w-full rounded-full border border-[#d6dad3]"
                style={{ background: activeConfig.scale.gradient }}
                aria-label={`${activeConfig.scale.unit} scale`}
              />
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
