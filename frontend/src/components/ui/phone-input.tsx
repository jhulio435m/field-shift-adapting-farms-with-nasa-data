import { useEffect, useMemo, useState } from 'react';
import { Check, ChevronsUpDown } from 'lucide-react';
import {
  getCountries,
  getCountryCallingCode,
  type CountryCode,
} from 'libphonenumber-js';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { Input } from '@/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { tr } from '@/i18n';
import { cn } from '@/lib/utils';
import type { Language } from '@/types';

function countryToFlag(country: CountryCode): string {
  return country
    .toUpperCase()
    .replace(/./g, (char) => String.fromCodePoint(127397 + char.charCodeAt(0)));
}

interface CountryOption {
  code: CountryCode;
  name: string;
  callingCode: string;
  flag: string;
}

interface PhoneInputProps {
  id?: string;
  language: Language;
  defaultCountry?: CountryCode;
  value: string;
  onChange: (e164: string) => void;
  required?: boolean;
}

export function PhoneInput({
  id,
  language,
  defaultCountry = 'PE',
  value,
  onChange,
  required,
}: PhoneInputProps) {
  const [open, setOpen] = useState(false);
  const [country, setCountry] = useState<CountryCode>(defaultCountry);
  const [nationalNumber, setNationalNumber] = useState(() => {
    const code = getCountryCallingCode(defaultCountry);
    return value.startsWith(`+${code}`) ? value.slice(code.length + 1) : '';
  });

  const countries = useMemo<CountryOption[]>(() => {
    const displayNames = new Intl.DisplayNames([language], { type: 'region' });
    return getCountries()
      .map((code) => ({
        code,
        name: displayNames.of(code) ?? code,
        callingCode: getCountryCallingCode(code),
        flag: countryToFlag(code),
      }))
      .sort((a, b) => a.name.localeCompare(b.name, language));
  }, [language]);

  const activeCallingCode = getCountryCallingCode(country);

  useEffect(() => {
    if (value.startsWith(`+${activeCallingCode}`)) {
      setNationalNumber(value.slice(activeCallingCode.length + 1));
    } else if (!value) {
      setNationalNumber('');
    }
  }, [value, activeCallingCode]);

  function handleSelectCountry(nextCountry: CountryCode) {
    setCountry(nextCountry);
    setOpen(false);
    const nextCode = getCountryCallingCode(nextCountry);
    onChange(nationalNumber ? `+${nextCode}${nationalNumber}` : '');
  }

  function handleNumberChange(raw: string) {
    const digits = raw.replace(/\D/g, '');
    setNationalNumber(digits);
    onChange(digits ? `+${activeCallingCode}${digits}` : '');
  }

  return (
    <div className="flex gap-1.5">
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="outline"
            role="combobox"
            aria-expanded={open}
            aria-label={tr(language, 'Código de país', 'Country code')}
            className="shrink-0 justify-between px-2.5 font-medium"
          >
            <span className="flex items-center gap-1.5">
              <span>{countryToFlag(country)}</span>
              <span>+{activeCallingCode}</span>
            </span>
            <ChevronsUpDown className="h-3.5 w-3.5 opacity-60" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-72 p-0" align="start">
          <Command>
            <CommandInput
              placeholder={tr(
                language,
                'Buscar país o código (+51)...',
                'Search country or code (+51)...',
              )}
            />
            <CommandList>
              <CommandEmpty>
                {tr(language, 'No se encontró ningún país.', 'No country found.')}
              </CommandEmpty>
              <CommandGroup>
                {countries.map((item) => (
                  <CommandItem
                    key={item.code}
                    value={`${item.name} +${item.callingCode} ${item.code}`}
                    onSelect={() => handleSelectCountry(item.code)}
                  >
                    <span className="text-base leading-none">{item.flag}</span>
                    <span className="flex-1 truncate">{item.name}</span>
                    <span className="text-xs text-[#526356] tabular-nums">
                      +{item.callingCode}
                    </span>
                    <Check
                      className={cn(
                        'h-4 w-4 text-[#2d6a4f]',
                        country === item.code ? 'opacity-100' : 'opacity-0',
                      )}
                    />
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>

      <Input
        id={id}
        type="tel"
        inputMode="numeric"
        autoComplete="tel-national"
        required={required}
        value={nationalNumber}
        onChange={(e) => handleNumberChange(e.target.value)}
        placeholder="999999999"
      />
    </div>
  );
}
