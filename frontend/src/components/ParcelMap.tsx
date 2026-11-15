import { useEffect, useRef, useState } from 'react';
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';
import {
  ChevronDown,
  ChevronUp,
  CircleDot,
  CloudRain,
  Droplets,
  Layers,
  Map as MapIcon,
  Mountain,
  Plus,
  Snowflake,
  Sprout,
} from 'lucide-react';
import { tr } from '../i18n';
import type { Language, Parcel, ParcelDiagnosis } from '../types';
import { Button } from './ui/button';

// Fix default marker icons under Vite bundling
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

interface ParcelMapProps {
  lat: number;
  lon: number;
  language: Language;
  focusZoom?: boolean;
  parcels?: Parcel[];
  focusedParcel?: Parcel | null;
  diagnosis?: ParcelDiagnosis | null;
  onSelectCoords: (lat: number, lon: number) => void;
  onSelectParcel?: (id: string) => void;
  onStartNewParcelAtCoords?: () => void;
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

export function ParcelMap({
  lat,
  lon,
  language,
  focusZoom = false,
  parcels = [],
  focusedParcel = null,
  diagnosis = null,
  onSelectCoords,
  onSelectParcel,
  onStartNewParcelAtCoords,
}: ParcelMapProps) {
  const [baseMap, setBaseMap] = useState<BaseMapKey>('osm');
  const [activeFilter, setActiveFilter] = useState<NasaFilterKey>('none');
  const [layerOpacity, setLayerOpacity] = useState<number>(0.7);
  const [showHalo, setShowHalo] = useState<boolean>(true);
  const [controlsOpen, setControlsOpen] = useState<boolean>(true);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const controlsOverlayRef = useRef<HTMLDivElement | null>(null);
  const legendOverlayRef = useRef<HTMLDivElement | null>(null);
  const mapRef = useRef<L.Map | null>(null);
  const pinMarkerRef = useRef<L.Marker | null>(null);
  const parcelGroupRef = useRef<L.LayerGroup | null>(null);
  const thematicGroupRef = useRef<L.LayerGroup | null>(null);
  const baseLayersRef = useRef<Record<BaseMapKey, L.TileLayer> | null>(null);
  const overlayLayersRef = useRef<Record<Exclude<NasaFilterKey, 'none'>, L.TileLayer> | null>(null);

  const onSelectRef = useRef(onSelectCoords);
  onSelectRef.current = onSelectCoords;
  const onSelectParcelRef = useRef(onSelectParcel);
  onSelectParcelRef.current = onSelectParcel;

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

    const gibsPrecip = L.tileLayer(
      'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/IMERG_Precipitation_Rate/default/2024-02-15T00:00:00Z/GoogleMapsCompatible_Level6/{z}/{y}/{x}.png',
      { opacity: 0.7, maxNativeZoom: 6, maxZoom: 18, attribution: 'NASA GPM IMERG' },
    );
    const gibsSoil = L.tileLayer(
      'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/SMAP_L4_Analyzed_Root_Zone_Soil_Moisture/default/2024-07-01/GoogleMapsCompatible_Level6/{z}/{y}/{x}.png',
      { opacity: 0.7, maxNativeZoom: 6, maxZoom: 18, attribution: 'NASA SMAP L4 Soil Moisture' },
    );
    const gibsLst = L.tileLayer(
      'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/MODIS_Terra_Land_Surface_Temp_Night/default/2024-07-01/GoogleMapsCompatible_Level7/{z}/{y}/{x}.png',
      { opacity: 0.7, maxNativeZoom: 7, maxZoom: 18, attribution: 'NASA MODIS LST Night' },
    );
    const gibsNdvi = L.tileLayer(
      'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/MODIS_Terra_L3_NDVI_16Day/default/2024-07-01/GoogleMapsCompatible_Level9/{z}/{y}/{x}.png',
      { opacity: 0.7, maxNativeZoom: 9, maxZoom: 18, attribution: 'NASA MODIS NDVI 16-Day' },
    );
    const gibsWater = L.tileLayer(
      'https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/MODIS_Terra_CorrectedReflectance_Bands721/default/2024-07-01/GoogleMapsCompatible_Level9/{z}/{y}/{x}.jpg',
      { opacity: 0.7, maxNativeZoom: 9, maxZoom: 18, attribution: 'NASA MODIS Bands 7-2-1' },
    );

    const map = L.map(containerRef.current, { layers: [osmLayer] }).setView([lat, lon], 7);
    const thematicGroup = L.layerGroup().addTo(map);
    const parcelGroup = L.layerGroup().addTo(map);
    const pinMarker = L.marker([lat, lon]).addTo(map);

    map.on('click', (e: L.LeafletMouseEvent) => {
      const nextLat = Number(e.latlng.lat.toFixed(4));
      const nextLon = Number(e.latlng.lng.toFixed(4));
      onSelectRef.current(nextLat, nextLon);
    });

    mapRef.current = map;
    pinMarkerRef.current = pinMarker;
    parcelGroupRef.current = parcelGroup;
    thematicGroupRef.current = thematicGroup;
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
      baseLayersRef.current = null;
      overlayLayersRef.current = null;
    };
  }, []);

  // Prevent clicks/scrolls on floating in-map UI panels from panning, zooming, or pinning Leaflet underneath
  useEffect(() => {
    if (controlsOverlayRef.current) {
      L.DomEvent.disableClickPropagation(controlsOverlayRef.current);
      L.DomEvent.disableScrollPropagation(controlsOverlayRef.current);
    }
    if (legendOverlayRef.current) {
      L.DomEvent.disableClickPropagation(legendOverlayRef.current);
      L.DomEvent.disableScrollPropagation(legendOverlayRef.current);
    }
  });

  // Swap base map cleanly instead of stacking layers over OSM
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

  // Apply active NASA overlay filter and opacity
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

  // Render parcel green markers
  useEffect(() => {
    const group = parcelGroupRef.current;
    if (!group) return;
    group.clearLayers();

    const focusedId = focusedParcel?.id ?? null;

    parcels.forEach((p) => {
      const isFocused = p.id === focusedId;
      const circle = L.circleMarker([p.lat, p.lon], {
        radius: isFocused ? 10 : 8,
        color: isFocused ? '#143326' : '#1b4332',
        weight: isFocused ? 3 : 2,
        fillColor: isFocused ? '#1b4332' : '#dcfce7',
        fillOpacity: isFocused ? 0.95 : 0.9,
      });
      circle.bindTooltip(p.name, {
        direction: 'top',
        offset: [0, -8],
      });
      circle.on('click', (e: L.LeafletMouseEvent) => {
        L.DomEvent.stopPropagation(e);
        onSelectParcelRef.current?.(p.id);
      });
      circle.addTo(group);
    });
  }, [parcels, focusedParcel?.id]);

  // Sync pin position & focus zoom
  useEffect(() => {
    const map = mapRef.current;
    const marker = pinMarkerRef.current;
    if (!map || !marker) return;
    marker.setLatLng([lat, lon]);
    if (focusZoom) {
      map.setView([lat, lon], Math.max(map.getZoom(), 9));
    }
  }, [lat, lon, focusZoom]);

  const filterOptions: FilterOptionConfig[] = [
    {
      key: 'none',
      label: tr(language, 'Sin filtro (Solo mapa base)', 'No filter (Base map only)'),
      shortLabel: tr(language, 'Sin filtro', 'None'),
      icon: MapIcon,
      color: '#1b4332',
      hint: tr(
        language,
        'Haz clic en el mapa para fijar coordenadas o activa un filtro NASA.',
        'Click the map to pin coordinates or select a NASA filter.',
      ),
    },
    {
      key: 'precip',
      label: tr(language, 'Lluvia (NASA GPM IMERG)', 'Rainfall (NASA GPM IMERG)'),
      shortLabel: tr(language, 'Lluvia', 'Rainfall'),
      icon: CloudRain,
      color: '#2563eb',
      hint: tr(
        language,
        'Tasa satelital de precipitación GPM IMERG + lluvia anual NASA POWER.',
        'Satellite GPM IMERG precipitation rate + annual NASA POWER rainfall.',
      ),
      scale: {
        gradient: 'linear-gradient(90deg, #e0f2fe 0%, #38bdf8 35%, #2563eb 70%, #7e22ce 100%)',
        labels: ['0 mm/h', '5 mm/h', '15+ mm/h'],
        unit: 'IMERG',
      },
    },
    {
      key: 'soil',
      label: tr(language, 'Humedad suelo (NASA SMAP L4)', 'Soil moisture (NASA SMAP L4)'),
      shortLabel: tr(language, 'Suelo SMAP', 'SMAP Soil'),
      icon: Sprout,
      color: '#0d9488',
      hint: tr(
        language,
        'Humedad radicular satelital SMAP L4 (0-100 cm) y reserva hídrica superficial.',
        'Satellite SMAP L4 root-zone soil moisture (0-100 cm).',
      ),
      scale: {
        gradient: 'linear-gradient(90deg, #fef3c7 0%, #99f6e4 45%, #0d9488 75%, #115e59 100%)',
        labels: [
          tr(language, '15% Seco', '15% Dry'),
          tr(language, '45% Óptimo', '45% Optimal'),
          tr(language, '80%+ Saturado', '80%+ Wet'),
        ],
        unit: 'SMAP L4',
      },
    },
    {
      key: 'lst',
      label: tr(language, 'Calor / Heladas (MODIS LST)', 'Heat / Frost (MODIS LST)'),
      shortLabel: tr(language, 'Calor/LST', 'Heat/LST'),
      icon: Snowflake,
      color: '#ea580c',
      hint: tr(
        language,
        'Temperatura superficial nocturna MODIS LST para riesgo de heladas o estrés térmico.',
        'Nighttime MODIS surface temperature for frost pockets and thermal stress.',
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
      label: tr(language, 'Vegetación (MODIS NDVI)', 'Vegetation (MODIS NDVI)'),
      shortLabel: 'NDVI',
      icon: Sprout,
      color: '#15803d',
      hint: tr(
        language,
        'Verdor satelital MODIS 16 días junto al último NDVI Sentinel-2 de tu parcela.',
        '16-day MODIS greenness paired with your field latest Sentinel-2 NDVI.',
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
      label: tr(language, 'Agua y balance (Bands 7-2-1)', 'Water balance (Bands 7-2-1)'),
      shortLabel: tr(language, 'Agua', 'Water'),
      icon: Droplets,
      color: '#4f46e5',
      hint: tr(
        language,
        'Bandas SWIR-NIR-VIS (7-2-1) para cuerpos de agua, suelos húmedos y balance Lluvia - ET.',
        'Bands 7-2-1 highlighting surface water, wet soil, and Rainfall - ET balance.',
      ),
      scale: {
        gradient: 'linear-gradient(90deg, #dc2626 0%, #f59e0b 40%, #38bdf8 70%, #1e3a8a 100%)',
        labels: [
          tr(language, 'Déficit', 'Deficit'),
          tr(language, 'Equilibrio', 'Balanced'),
          tr(language, 'Superávit', 'Surplus'),
        ],
        unit: 'WATER',
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
          `Lluvia: ${diagnosis.annual_precip_mm} mm/año · ${diagnosis.dry_months.length} meses secos`,
          `Rain: ${diagnosis.annual_precip_mm} mm/yr · ${diagnosis.dry_months.length} dry mo`,
        );
      case 'soil':
        return tr(
          language,
          `Suelo raíz: ${Math.round(diagnosis.avg_soil_moisture_root * 100)}% · Sup: ${Math.round(diagnosis.avg_soil_moisture_top * 100)}%`,
          `Root soil: ${Math.round(diagnosis.avg_soil_moisture_root * 100)}% · Top: ${Math.round(diagnosis.avg_soil_moisture_top * 100)}%`,
        );
      case 'lst':
        return tr(
          language,
          `Temp. media: ${diagnosis.annual_temp_mean_c} °C · Heladas: ${diagnosis.frost_risk_months.length}m · Calor: ${diagnosis.heat_stress_months.length}m`,
          `Mean temp: ${diagnosis.annual_temp_mean_c} °C · Frost: ${diagnosis.frost_risk_months.length}m · Heat: ${diagnosis.heat_stress_months.length}m`,
        );
      case 'ndvi': {
        const ndviVal = diagnosis.historical_correlation?.latest_ndvi;
        const vegStatus = diagnosis.historical_correlation?.vegetation_status;
        return tr(
          language,
          `NDVI: ${ndviVal !== null && ndviVal !== undefined ? ndviVal.toFixed(2) : 'N/D'}${vegStatus ? ` (${vegStatus})` : ''}`,
          `NDVI: ${ndviVal !== null && ndviVal !== undefined ? ndviVal.toFixed(2) : 'N/A'}${vegStatus ? ` (${vegStatus})` : ''}`,
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

  // Draw instant visual halo and readout badge over focused coordinate when a filter is active
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
    'h-8 w-full cursor-pointer rounded-md border border-[#d6dad3] bg-white px-2 text-[11.5px] font-medium text-[#111813] focus:border-[#1b4332] focus:outline-none focus:ring-2 focus:ring-[#1b4332]/15';

  return (
    <section
      aria-label={tr(language, 'Visor geoespacial satelital', 'Satellite geospatial viewer')}
      className="rounded-xl border border-[#e3e6e1] bg-white p-4 sm:p-5"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-bold text-[#111813]">
            {tr(
              language,
              'Visor Geoespacial Satelital (NASA GIBS + SRTM)',
              'Satellite Geospatial Viewer (NASA GIBS + SRTM)',
            )}
          </h2>
          <p className="mt-0.5 text-xs text-[#526056]">
            {focusedParcel && !isCustomPin
              ? `${tr(language, 'Parcela enfocada:', 'Focused field:')} ${focusedParcel.name}`
              : tr(
                  language,
                  'Pin en coordenada personalizada — listo para guardar o analizar.',
                  'Custom coordinate pin — ready to save or analyze.',
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
              {tr(language, 'Alt.', 'Elev.')} {diagnosis.terrain.elevation_m} m ·{' '}
              {tr(language, 'Pend.', 'Slope')} {diagnosis.terrain.slope_pct}%
            </span>
          )}

          {onStartNewParcelAtCoords && (
            <Button
              type="button"
              size="sm"
              variant={isCustomPin ? 'default' : 'outline'}
              onClick={onStartNewParcelAtCoords}
            >
              <Plus className="h-3.5 w-3.5" />
              <span>
                {tr(language, 'Crear parcela en este punto', 'Create field at this point')}
              </span>
            </Button>
          )}
        </div>
      </div>

      {/* Map canvas with in-map floating Layer/Filter selector and live legend */}
      <div className="map-shell mt-3">
        <div id="map" ref={containerRef} />

        {/* In-map top-right collapsible control box */}
        <div
          ref={controlsOverlayRef}
          onClick={stopMapPropagation}
          onMouseDown={stopMapPropagation}
          onDoubleClick={stopMapPropagation}
          className="absolute right-2.5 top-2.5 z-[1000] w-[224px] rounded-lg border border-[#d6dad3] bg-white/95 shadow-md backdrop-blur-xs sm:w-[248px]"
        >
          <button
            type="button"
            aria-expanded={controlsOpen}
            onClick={() => setControlsOpen((prev) => !prev)}
            className="flex w-full cursor-pointer items-center justify-between gap-2 px-3 py-2 text-left text-xs font-semibold text-[#111813] hover:bg-[#fafaf8] rounded-lg"
          >
            <span className="flex min-w-0 items-center gap-1.5">
              <Layers className="h-3.5 w-3.5 shrink-0 text-[#1b4332]" />
              <span className="truncate">
                {controlsOpen
                  ? tr(language, 'Capas y filtros NASA', 'Layers & NASA filters')
                  : `${tr(language, 'Capa:', 'Layer:')} ${activeConfig.shortLabel}`}
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
                  className="mb-1 block text-[11px] font-semibold text-[#475569]"
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
                  <option value="satellite">{tr(language, 'Satélite óptico (HD)', 'Optical satellite (HD)')}</option>
                  <option value="topo">{tr(language, 'Relieve topográfico', 'Topographic relief')}</option>
                </select>
              </div>

              <div>
                <label
                  htmlFor="nasa-filter-select"
                  className="mb-1 block text-[11px] font-semibold text-[#475569]"
                >
                  {tr(language, 'Filtro satelital NASA', 'NASA satellite filter')}
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

              <div className="grid grid-cols-2 gap-1.5 pt-0.5">
                <div>
                  <label
                    htmlFor="layer-opacity-select"
                    className="mb-1 block text-[10.5px] font-semibold text-[#475569]"
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
                    className={`inline-flex h-8 w-full items-center justify-center gap-1 rounded-md border px-2 text-[11px] font-medium transition-colors cursor-pointer disabled:cursor-not-allowed disabled:opacity-50 ${
                      showHalo && activeFilter !== 'none'
                        ? 'border-[#1b4332] bg-[#1b4332] text-white'
                        : 'border-[#d6dad3] bg-white text-[#111813] hover:bg-[#e6f4ea]'
                    }`}
                  >
                    <CircleDot className="h-3 w-3 shrink-0" />
                    <span>{tr(language, 'Halo', 'Halo')}</span>
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* In-map bottom-left readout & chromatic legend */}
        <div
          ref={legendOverlayRef}
          onClick={stopMapPropagation}
          onMouseDown={stopMapPropagation}
          onDoubleClick={stopMapPropagation}
          className="absolute bottom-6 left-2.5 z-[1000] max-w-[calc(100%-20px)] rounded-lg border border-[#d6dad3] bg-white/95 px-3 py-2 shadow-md backdrop-blur-xs sm:bottom-3 sm:max-w-[320px]"
        >
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="inline-flex items-center gap-1 text-[11.5px] font-semibold text-[#111813]">
              <ActiveFilterIcon className="h-3.5 w-3.5 text-[#1b4332]" />
              <span>{activeConfig.shortLabel}</span>
            </span>
            {fieldReadout && (
              <span className="rounded bg-[#e6f4ea] px-1.5 py-0.5 font-mono text-[10.5px] font-semibold text-[#1b4332] tabular-nums">
                {fieldReadout}
              </span>
            )}
          </div>

          <p className="mt-0.5 text-[11px] leading-snug text-[#475569]">{activeConfig.hint}</p>

          {activeConfig.scale && (
            <div className="mt-2">
              <div className="flex items-center justify-between font-mono text-[9.5px] font-semibold text-[#111813]">
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
