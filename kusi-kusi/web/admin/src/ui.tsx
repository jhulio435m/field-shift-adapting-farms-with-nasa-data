"use client";
import { useId, useState, type ReactNode, type FormEvent } from "react";
import { Check } from "lucide-react";
import { assetUrl, label, type Asset } from "./api";
import { Badge as StatusBadge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
export const getForm = (event: FormEvent<HTMLFormElement>) => {
  event.preventDefault();
  return new FormData(event.currentTarget);
};
export const text = (f: FormData, key: string) => String(f.get(key) || "");
export function Badge({ value }: { value: string }) {
  return (
    <StatusBadge
      variant={
        ["published_simulated", "succeeded"].includes(value)
          ? "default"
          : "secondary"
      }
    >
      {label(value)}
    </StatusBadge>
  );
}
export function Empty({ children }: { children: ReactNode }) {
  return <div className="empty">{children}</div>;
}
export function Preview({ asset }: { asset: Asset }) {
  return asset.media_type.startsWith("video/") ? (
    <video
      controls
      preload="metadata"
      src={assetUrl(asset.id)}
      aria-label={asset.label}
    />
  ) : (
    <img src={assetUrl(asset.id)} alt={asset.label} loading="lazy" />
  );
}
export function Approval({
  onApprove,
  disabled = false,
  text: description = "Confirmo que revisé el archivo, parámetros y costo de esta versión.",
}: {
  onApprove: () => void;
  disabled?: boolean;
  text?: string;
}) {
  const [checked, setChecked] = useState(false);
  const id = useId();
  return (
    <div className="approval">
      <div className="flex items-start gap-3">
        <Checkbox
          id={id}
          checked={checked}
          onCheckedChange={(value) => setChecked(value === true)}
          disabled={disabled}
        />
        <Label htmlFor={id} className="text-xs font-normal leading-relaxed">
          {description}
        </Label>
      </div>
      <Button
        className="mt-4 w-full"
        disabled={!checked || disabled}
        onClick={onApprove}
      >
        <Check />
        Aprobar esta versión
      </Button>
    </div>
  );
}
export function Confirmation({
  open,
  onOpenChange,
  title,
  description,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (value: boolean) => void;
  title: string;
  description: string;
  onConfirm: () => void;
}) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Volver</AlertDialogCancel>
          <AlertDialogAction onClick={onConfirm}>Confirmar</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
