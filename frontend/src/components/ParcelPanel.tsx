import { useEffect, useRef, useState } from 'react';
import {
  CloudRain,
  Flame,
  MapPin,
  Pencil,
  Plus,
  RefreshCw,
  Sun,
  Trash2,
  X,
} from 'lucide-react';
import { tr } from '../i18n';
import type {
  CropKey,
  IrrigationType,
  Language,
  Parcel,
  PriorityType,
  SoilTextureType,
} from '../types';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';

interface ParcelPanelProps {
  language: Language;
  profileName: string;
  phoneLast4: string;
  parcels: Parcel[];
  focusedParcelId: string | null;
  selectedLat: number;
  selectedLon: number;
  parcelStatus: string;
  rainDeltaPct: number;
  tempDeltaC: number;
  forceOpenNewForm?: number;
  onSelectCoords: (lat: number, lon: number, zoom?: boolean) => void;
  onFocusParcel: (id: string) => void;
  onSaveParcel: (payload: Omit<Parcel, 'id'>) => Promise<void>;
  onUpdateParcel: (id: string, payload: Omit<Parcel, 'id'>) => Promise<void>;
  onDeleteParcel: (id: string) => Promise<void>;
  onRunAnalysis: () => void;
  onChangeRain: (val: number) => void;
  onChangeTemp: (val: number) => void;
}

const selectClass =
  'flex h-9 w-full rounded-lg border border-[#e3e6e1] bg-white px-2.5 py-1.5 text-xs text-[#111813] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#1b4332]';

export function ParcelPanel({
  language,
  profileName,
  phoneLast4,
  parcels,
  focusedParcelId,
  selectedLat,
  selectedLon,
  parcelStatus,
  rainDeltaPct,
  tempDeltaC,
  forceOpenNewForm = 0,
  onSelectCoords,
  onFocusParcel,
  onSaveParcel,
  onUpdateParcel,
  onDeleteParcel,
  onRunAnalysis,
  onChangeRain,
  onChangeTemp,
}: ParcelPanelProps) {
  const [showNewForm, setShowNewForm] = useState(() => parcels.length === 0);
  const [editingParcelId, setEditingParcelId] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  const [parcelName, setParcelName] = useState('');
  const [currentCrop, setCurrentCrop] = useState<CropKey>('maize_continuous');
  const [irrigation, setIrrigation] = useState<IrrigationType>('rainfed');
  const [soilTexture, setSoilTexture] = useState<SoilTextureType>('unknown');
  const [priority, setPriority] = useState<PriorityType>('water');
  const [hasLivestock, setHasLivestock] = useState(false);
  const [areaHa, setAreaHa] = useState('');
  const [plantingMonth, setPlantingMonth] = useState('');
  const [saving, setSaving] = useState(false);

  const nameInputRef = useRef<HTMLInputElement | null>(null);

  const cropShortLabels: Record<CropKey, string> = {
    maize_continuous: tr(language, 'Maíz', 'Maize'),
    potato_continuous: tr(language, 'Papa', 'Potato'),
    wheat_continuous: tr(language, 'Trigo', 'Wheat'),
    rice_continuous: tr(language, 'Arroz', 'Rice'),
    mixed_vegetables: tr(language, 'Hortalizas', 'Vegetables'),
  };

  function resetFormFields() {
    setParcelName('');
    setCurrentCrop('maize_continuous');
    setIrrigation('rainfed');
    setSoilTexture('unknown');
    setPriority('water');
    setHasLivestock(false);
    setAreaHa('');
    setPlantingMonth('');
  }

  useEffect(() => {
    if (forceOpenNewForm > 0) {
      setEditingParcelId(null);
      resetFormFields();
      setShowNewForm(true);
      window.setTimeout(() => {
        nameInputRef.current?.focus();
      }, 60);
    }
  }, [forceOpenNewForm]);

  function startNewParcel() {
    if (showNewForm && !editingParcelId) {
      setShowNewForm(false);
      return;
    }
    setEditingParcelId(null);
    resetFormFields();
    setShowNewForm(true);
  }

  function startEditParcel(parcel: Parcel) {
    setEditingParcelId(parcel.id);
    setParcelName(parcel.name);
    setCurrentCrop(parcel.current_crop);
    setIrrigation(parcel.irrigation);
    setSoilTexture(parcel.soil_texture);
    setPriority(parcel.priority);
    setHasLivestock(Boolean(parcel.has_livestock));
    setAreaHa(parcel.area_ha ? String(parcel.area_ha) : '');
    setPlantingMonth(parcel.planting_month ? String(parcel.planting_month) : '');
    onSelectCoords(parcel.lat, parcel.lon, true);
    setShowNewForm(true);
  }

  function cancelForm() {
    setEditingParcelId(null);
    resetFormFields();
    setShowNewForm(false);
  }

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    try {
      const parsedArea = areaHa.trim() ? Number(areaHa) : null;
      const parsedMonth = plantingMonth ? Number(plantingMonth) : null;
      const payload: Omit<Parcel, 'id'> = {
        name: parcelName.trim(),
        lat: selectedLat,
        lon: selectedLon,
        current_crop: currentCrop,
        irrigation,
        soil_texture: soilTexture,
        priority,
        has_livestock: hasLivestock,
        area_ha: parsedArea && Number.isFinite(parsedArea) ? parsedArea : null,
        planting_month: parsedMonth && Number.isInteger(parsedMonth) ? parsedMonth : null,
      };
      if (editingParcelId) {
        await onUpdateParcel(editingParcelId, payload);
      } else {
        await onSaveParcel(payload);
      }
      setEditingParcelId(null);
      resetFormFields();
      setShowNewForm(false);
    } finally {
      setSaving(false);
    }
  }

  const formOpen = showNewForm || parcels.length === 0;

  return (
    <section
      id="workspaceControls"
      aria-label={tr(language, 'Selector de campo y clima', 'Field & climate selector')}
      className="rounded-xl border border-[#e3e6e1] bg-white p-3.5 sm:p-4"
    >
      <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
        {/* 1. Field pills */}
        <div className="min-w-0 flex-1 space-y-1.5">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-xs font-bold text-[#111813]">
              {tr(language, '1. Elige tu campo', '1. Pick your field')}
            </span>
            <span className="font-mono text-[11px] text-[#475569]">
              {profileName} · •••• {phoneLast4} ({parcels.length}/8)
            </span>
            {parcelStatus && (
              <span className="rounded bg-[#fafaf8] px-2 py-0.5 text-[11px] font-medium text-[#1b4332]" role="status">
                {parcelStatus}
              </span>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-2" aria-label={tr(language, 'Tus campos', 'Your fields')}>
            {parcels.map((parcel) => {
              const isFocused = parcel.id === focusedParcelId;
              const confirmingDelete = confirmDeleteId === parcel.id;

              return (
                <div
                  key={parcel.id}
                  className={`inline-flex items-center gap-1 rounded-lg border px-2.5 py-1.5 transition-colors ${
                    isFocused
                      ? 'border-[#1b4332] bg-[#e6f4ea]'
                      : 'border-[#e3e6e1] bg-[#fafaf8] hover:bg-[#f0f2ee]'
                  }`}
                >
                  <button
                    type="button"
                    onClick={() => {
                      setConfirmDeleteId(null);
                      onFocusParcel(parcel.id);
                    }}
                    aria-pressed={isFocused}
                    className="flex items-center gap-1.5 text-left cursor-pointer"
                  >
                    <MapPin
                      className={`h-3.5 w-3.5 shrink-0 ${
                        isFocused ? 'text-[#1b4332]' : 'text-[#475569]'
                      }`}
                    />
                    <span
                      className={`text-xs ${
                        isFocused ? 'font-bold text-[#143326]' : 'font-semibold text-[#111813]'
                      }`}
                    >
                      {parcel.name}
                    </span>
                    <span className="font-mono text-[10.5px] text-[#475569]">
                      · {cropShortLabels[parcel.current_crop]}
                      {parcel.has_livestock ? tr(language, ' + ganado', ' + livestock') : ''}
                      {parcel.area_ha ? ` · ${parcel.area_ha} ha` : ''}
                    </span>
                  </button>

                  {confirmingDelete ? (
                    <div className="ml-1 flex items-center gap-1">
                      <button
                        type="button"
                        onClick={() => {
                          setConfirmDeleteId(null);
                          void onDeleteParcel(parcel.id);
                        }}
                        className="rounded bg-[#b91c1c] px-1.5 py-0.5 text-[10px] font-semibold text-white hover:bg-[#991b1b] cursor-pointer"
                      >
                        {tr(language, 'Borrar', 'Delete')}
                      </button>
                      <button
                        type="button"
                        onClick={() => setConfirmDeleteId(null)}
                        aria-label={tr(language, 'Cancelar borrado', 'Cancel delete')}
                        className="rounded border border-[#e3e6e1] bg-white p-0.5 text-[#475569] hover:text-[#111813] cursor-pointer"
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </div>
                  ) : (
                    <div className="ml-1 flex items-center gap-0.5">
                      <button
                        type="button"
                        title={tr(language, 'Editar campo', 'Edit field')}
                        aria-label={tr(language, `Editar ${parcel.name}`, `Edit ${parcel.name}`)}
                        onClick={() => startEditParcel(parcel)}
                        className="rounded p-1 text-[#475569] hover:bg-white/80 hover:text-[#111813] cursor-pointer"
                      >
                        <Pencil className="h-3 w-3" />
                      </button>
                      <button
                        type="button"
                        title={tr(language, 'Eliminar campo', 'Delete field')}
                        aria-label={tr(language, `Eliminar ${parcel.name}`, `Delete ${parcel.name}`)}
                        onClick={() => setConfirmDeleteId(parcel.id)}
                        className="rounded p-1 text-[#475569] hover:bg-[#fee2e2] hover:text-[#b91c1c] cursor-pointer"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                  )}
                </div>
              );
            })}

            <button
              type="button"
              onClick={startNewParcel}
              disabled={parcels.length >= 8 && !editingParcelId}
              className="inline-flex items-center gap-1 rounded-lg border border-dashed border-[#1b4332]/45 bg-white px-2.5 py-1.5 text-xs font-semibold text-[#1b4332] hover:bg-[#e6f4ea] disabled:opacity-50 cursor-pointer"
            >
              <Plus className="h-3.5 w-3.5" />
              <span>{tr(language, 'Nuevo campo', 'New field')}</span>
            </button>
          </div>
        </div>

        {/* 2. Weather scenario buttons */}
        <div className="shrink-0 space-y-1.5 border-t border-[#e3e6e1] pt-2.5 xl:border-l xl:border-t-0 xl:pl-4 xl:pt-0">
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs font-bold text-[#111813]">
              {tr(language, '2. Simula el clima', '2. Simulate weather')}
            </span>
            <button
              type="button"
              onClick={onRunAnalysis}
              disabled={!focusedParcelId}
              className="inline-flex items-center gap-1 text-[11px] font-semibold text-[#1b4332] hover:underline disabled:opacity-40 cursor-pointer"
            >
              <RefreshCw className="h-3 w-3" />
              <span>{tr(language, 'Actualizar', 'Refresh')}</span>
            </button>
          </div>

          <div className="grid grid-cols-3 gap-1.5">
            <button
              type="button"
              onClick={() => {
                onChangeRain(0);
                onChangeTemp(0);
              }}
              aria-pressed={rainDeltaPct === 0 && tempDeltaC === 0}
              className={`inline-flex items-center justify-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs transition-colors cursor-pointer ${
                rainDeltaPct === 0 && tempDeltaC === 0
                  ? 'border-[#1b4332] bg-[#e6f4ea] text-[#14532d] font-bold'
                  : 'border-[#e3e6e1] bg-[#fafaf8] text-[#111813] font-medium hover:bg-[#f0f2ee]'
              }`}
            >
              <Sun className="h-3.5 w-3.5 text-[#1b4332]" />
              <span>{tr(language, 'Normal', 'Normal')}</span>
            </button>

            <button
              type="button"
              onClick={() => {
                onChangeRain(-25);
                onChangeTemp(1.5);
              }}
              aria-pressed={rainDeltaPct === -25 && tempDeltaC === 1.5}
              className={`inline-flex items-center justify-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs transition-colors cursor-pointer ${
                rainDeltaPct === -25 && tempDeltaC === 1.5
                  ? 'border-[#b45309] bg-[#fef3c7] text-[#92400e] font-bold'
                  : 'border-[#e3e6e1] bg-[#fafaf8] text-[#111813] font-medium hover:bg-[#fef3c7]/60'
              }`}
            >
              <Flame className="h-3.5 w-3.5 text-[#b45309]" />
              <span>{tr(language, 'Sequía -25%', 'Drought -25%')}</span>
            </button>

            <button
              type="button"
              onClick={() => {
                onChangeRain(20);
                onChangeTemp(0.5);
              }}
              aria-pressed={rainDeltaPct === 20 && tempDeltaC === 0.5}
              className={`inline-flex items-center justify-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs transition-colors cursor-pointer ${
                rainDeltaPct === 20 && tempDeltaC === 0.5
                  ? 'border-[#1d4ed8] bg-[#dbeafe] text-[#1e3a8a] font-bold'
                  : 'border-[#e3e6e1] bg-[#fafaf8] text-[#111813] font-medium hover:bg-[#dbeafe]/60'
              }`}
            >
              <CloudRain className="h-3.5 w-3.5 text-[#1d4ed8]" />
              <span>{tr(language, 'Lluvioso +20%', 'Rainy +20%')}</span>
            </button>
          </div>
        </div>
      </div>

      {/* Expandable drawer for creating / editing a field */}
      {formOpen && (
        <form onSubmit={handleSave} className="mt-4 space-y-3 border-t border-[#e3e6e1] pt-3.5">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-xs font-bold text-[#111813]">
                {editingParcelId
                  ? tr(language, 'Editar campo seleccionado', 'Edit selected field')
                  : tr(language, 'Registrar nuevo campo', 'Register new field')}
              </h3>
              <span className="rounded border border-[#e3e6e1] bg-[#fafaf8] px-2 py-0.5 font-mono text-[11px] text-[#1b4332] tabular-nums">
                {selectedLat.toFixed(4)}, {selectedLon.toFixed(4)}
              </span>
            </div>

            <div className="flex flex-wrap items-center gap-1.5">
              {!editingParcelId && (
                <>
                  <span className="text-[11px] text-[#475569]">
                    {tr(language, 'Ejemplos:', 'Examples:')}
                  </span>
                  <button
                    type="button"
                    onClick={() => {
                      onSelectCoords(-12.065, -75.2049, true);
                      setParcelName('Lote Mantaro — Papa');
                      setCurrentCrop('potato_continuous');
                      setIrrigation('rainfed');
                      setSoilTexture('loam');
                      setPriority('soil_health');
                      setHasLivestock(true);
                      setAreaHa('2.5');
                      setPlantingMonth('10');
                    }}
                    className="rounded border border-[#e3e6e1] bg-[#fafaf8] px-2 py-0.5 text-[11px] font-medium text-[#111813] hover:bg-[#e6f4ea] cursor-pointer"
                  >
                    Mantaro
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      onSelectCoords(-5.1945, -80.6328, true);
                      setParcelName('Valle Piura — Arroz');
                      setCurrentCrop('rice_continuous');
                      setIrrigation('irrigated');
                      setSoilTexture('clay');
                      setPriority('water');
                      setHasLivestock(false);
                      setAreaHa('4.0');
                      setPlantingMonth('1');
                    }}
                    className="rounded border border-[#e3e6e1] bg-[#fafaf8] px-2 py-0.5 text-[11px] font-medium text-[#111813] hover:bg-[#e6f4ea] cursor-pointer"
                  >
                    Piura
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      onSelectCoords(-13.3144, -72.0947, true);
                      setParcelName('Urubamba — Maíz');
                      setCurrentCrop('maize_continuous');
                      setIrrigation('limited');
                      setSoilTexture('loam');
                      setPriority('water');
                      setHasLivestock(true);
                      setAreaHa('1.8');
                      setPlantingMonth('9');
                    }}
                    className="rounded border border-[#e3e6e1] bg-[#fafaf8] px-2 py-0.5 text-[11px] font-medium text-[#111813] hover:bg-[#e6f4ea] cursor-pointer"
                  >
                    Cusco
                  </button>
                </>
              )}

              {parcels.length > 0 && (
                <button
                  type="button"
                  onClick={cancelForm}
                  className="ml-2 text-[11px] font-semibold text-[#475569] hover:text-[#111813] cursor-pointer"
                >
                  {tr(language, 'Cerrar', 'Close')}
                </button>
              )}
            </div>
          </div>

          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 md:grid-cols-4 lg:grid-cols-7">
            <div className="sm:col-span-2">
              <Label htmlFor="parcelName" className="text-[11px]">
                {tr(language, 'Nombre del campo', 'Field name')}
              </Label>
              <Input
                ref={nameInputRef}
                id="parcelName"
                type="text"
                maxLength={80}
                required
                className="h-9 text-xs"
                value={parcelName}
                onChange={(e) => setParcelName(e.target.value)}
                placeholder={tr(language, 'Ej. Potrero Norte', 'E.g. North pasture')}
              />
            </div>

            <div>
              <Label htmlFor="currentCrop" className="text-[11px]">
                {tr(language, 'Qué tiene ahora', 'Current crop')}
              </Label>
              <select
                id="currentCrop"
                className={selectClass}
                value={currentCrop}
                onChange={(e) => setCurrentCrop(e.target.value as CropKey)}
              >
                <option value="maize_continuous">{tr(language, 'Maíz continuo', 'Maize')}</option>
                <option value="potato_continuous">{tr(language, 'Papa continua', 'Potato')}</option>
                <option value="wheat_continuous">{tr(language, 'Trigo / cereal', 'Wheat')}</option>
                <option value="rice_continuous">{tr(language, 'Arroz continuo', 'Rice')}</option>
                <option value="mixed_vegetables">{tr(language, 'Hortalizas', 'Vegetables')}</option>
              </select>
            </div>

            <div>
              <Label htmlFor="irrigation" className="text-[11px]">
                {tr(language, 'Riego', 'Irrigation')}
              </Label>
              <select
                id="irrigation"
                className={selectClass}
                value={irrigation}
                onChange={(e) => setIrrigation(e.target.value as IrrigationType)}
              >
                <option value="rainfed">{tr(language, 'Solo lluvia', 'Rainfed')}</option>
                <option value="limited">{tr(language, 'Poco riego', 'Limited')}</option>
                <option value="irrigated">{tr(language, 'Con riego', 'Irrigated')}</option>
              </select>
            </div>

            <div>
              <Label htmlFor="soilTexture" className="text-[11px]">
                {tr(language, 'Tipo de tierra', 'Soil type')}
              </Label>
              <select
                id="soilTexture"
                className={selectClass}
                value={soilTexture}
                onChange={(e) => setSoilTexture(e.target.value as SoilTextureType)}
              >
                <option value="unknown">{tr(language, 'No sé', 'Unknown')}</option>
                <option value="loam">{tr(language, 'Franca', 'Loam')}</option>
                <option value="sandy">{tr(language, 'Arenosa', 'Sandy')}</option>
                <option value="clay">{tr(language, 'Arcillosa', 'Clay')}</option>
              </select>
            </div>

            <div>
              <Label htmlFor="priority" className="text-[11px]">
                {tr(language, 'Prioridad', 'Priority')}
              </Label>
              <select
                id="priority"
                className={selectClass}
                value={priority}
                onChange={(e) => setPriority(e.target.value as PriorityType)}
              >
                <option value="forage_income">{tr(language, 'Pasto / Forraje', 'Pasture / Forage')}</option>
                <option value="water">{tr(language, 'Ahorrar agua', 'Save water')}</option>
                <option value="soil_health">{tr(language, 'Cuidar suelo', 'Soil health')}</option>
                <option value="pest_disruption">{tr(language, 'Menos plagas', 'Fewer pests')}</option>
              </select>
            </div>

            <div>
              <Label htmlFor="areaHa" className="text-[11px]">
                {tr(language, 'Hectáreas (opc.)', 'Hectares (opt.)')}
              </Label>
              <Input
                id="areaHa"
                type="number"
                min={0.01}
                max={100000}
                step="0.1"
                className="h-9 text-xs"
                value={areaHa}
                onChange={(e) => setAreaHa(e.target.value)}
                placeholder="2.5"
              />
            </div>
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3 pt-1">
            <div className="flex flex-wrap items-center gap-4">
              <div className="flex items-center gap-2">
                <Label htmlFor="plantingMonth" className="text-[11px] whitespace-nowrap">
                  {tr(language, 'Mes de siembra:', 'Planting month:')}
                </Label>
                <select
                  id="plantingMonth"
                  className="h-8 rounded-lg border border-[#e3e6e1] bg-white px-2 text-xs text-[#111813]"
                  value={plantingMonth}
                  onChange={(e) => setPlantingMonth(e.target.value)}
                >
                  <option value="">{tr(language, 'Opcional', 'Optional')}</option>
                  {Array.from({ length: 12 }, (_, idx) => (
                    <option key={idx + 1} value={String(idx + 1)}>
                      {new Date(2024, idx, 1).toLocaleString(
                        language === 'en' ? 'en-US' : 'es-PE',
                        { month: 'short' },
                      )}
                    </option>
                  ))}
                </select>
              </div>

              <label
                htmlFor="hasLivestock"
                className="flex cursor-pointer items-center gap-2 text-xs font-medium text-[#111813]"
              >
                <input
                  type="checkbox"
                  id="hasLivestock"
                  className="h-3.5 w-3.5 rounded border-[#e3e6e1] accent-[#1b4332]"
                  checked={hasLivestock}
                  onChange={(e) => setHasLivestock(e.target.checked)}
                />
                <span>
                  {tr(language, 'Tengo ganado (necesito pasto / forraje)', 'I raise livestock (forage needed)')}
                </span>
              </label>
            </div>

            <Button type="submit" size="sm" disabled={saving}>
              <Plus className="h-3.5 w-3.5" />
              {editingParcelId
                ? tr(language, 'Guardar cambios', 'Save changes')
                : tr(language, 'Guardar campo', 'Save field')}
            </Button>
          </div>
        </form>
      )}
    </section>
  );
}
