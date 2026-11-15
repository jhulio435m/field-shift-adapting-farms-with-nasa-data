"use client";

import { FormEvent, useEffect, useState } from "react";
import Image from "next/image";
import { Dialog } from "@/components/ui/dialog";
import { api } from "@/api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";

type Payment = {
  payment_code: string;
  procedure_code: string;
  concept: string;
  amount_cents: number;
  currency: string;
  receipt_reference_masked: string;
  paid_at: number | null;
  payment_state: "confirmed" | "pending" | "not_found" | "reversed";
  process_state: "not_started" | "in_progress" | "interrupted" | "completed";
  platform_name: string;
  voucher_use?: "available" | "used" | "review_required";
  synthetic: boolean;
};

const paymentLabel: Record<Payment["payment_state"], string> = {
  confirmed: "Pago confirmado",
  pending: "Pago pendiente",
  not_found: "No encontrado",
  reversed: "Pago revertido",
};
const processLabel: Record<Payment["process_state"], string> = {
  not_started: "Trámite sin iniciar",
  in_progress: "Trámite en curso",
  interrupted: "Trámite interrumpido",
  completed: "Trámite completado",
};

export default function StudentPaymentsDemo() {
  const [method, setMethod] = useState<"code" | "photo">("code");
  const [panel, setPanel] = useState<"history" | "result" | null>(null);
  const [photo, setPhoto] = useState<File | null>(null);
  const [preview, setPreview] = useState("");
  useEffect(() => {
    if (!photo) {
      setPreview("");
      return;
    }
    const url = URL.createObjectURL(photo);
    setPreview(url);
    return () => URL.revokeObjectURL(url);
  }, [photo]);
  const [payments, setPayments] = useState<Payment[]>([]);
  const [code, setCode] = useState("");
  const [dni, setDni] = useState("");
  const [studentLabel, setStudentLabel] = useState("");
  const [selected, setSelected] = useState<Payment | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    api<{ payments: Payment[] }>("/v1/demo/payments")
      .then((result) => {
        if (active) setPayments(result.payments);
      })
      .catch((reason: unknown) => {
        if (active)
          setError(
            reason instanceof Error
              ? reason.message
              : "No se pudo abrir la plataforma demo.",
          );
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  async function validate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError("");
    setMessage("");
    setSelected(null);
    try {
      const result = await api<{ payment: Payment }>(
        "/v1/demo/payments/validate",
        "POST",
        { code },
      );
      setSelected(result.payment);
      setPanel("result");
      setMessage("Pago encontrado.");
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "No se pudo validar el código.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function lookupStudent(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError("");
    setMessage("");
    setSelected(null);
    try {
      const result = await api<{
        student: {
          display_name: string;
          dni_masked: string;
          synthetic: boolean;
        };
        payments: Payment[];
      }>("/v1/demo/payments/by-dni", "POST", { dni });
      setPayments(result.payments);
      setStudentLabel(
        `${result.student.display_name} · DNI ${result.student.dni_masked}`,
      );
      setMessage("Historial encontrado en el clon local de prueba.");
    } catch (reason) {
      setStudentLabel("");
      setPayments([]);
      setError(
        reason instanceof Error &&
          reason.message === "No se encontró el recurso."
          ? "DNI no encontrado. Verifica los ocho dígitos e intenta nuevamente."
          : "No se pudo consultar el historial. Intenta nuevamente.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function validatePhoto(file?: File) {
    if (!file || busy) return;
    if (
      !["image/jpeg", "image/png", "image/webp"].includes(file.type) ||
      file.size > 5 * 1024 * 1024 ||
      file.size === 0
    ) {
      setError(
        "Solo se permiten fotos JPG, PNG o WebP de hasta 5 MB. No se permiten videos.",
      );
      setSelected(null);
      setMessage("");
      return;
    }
    let signature: Uint8Array;
    try {
      signature = new Uint8Array(await file.slice(0, 12).arrayBuffer());
    } catch {
      setError("No se pudo leer la imagen. Selecciona otra foto.");
      return;
    }
    const jpeg =
      signature[0] === 0xff && signature[1] === 0xd8 && signature[2] === 0xff;
    const png = [137, 80, 78, 71, 13, 10, 26, 10].every(
      (byte, index) => signature[index] === byte,
    );
    const webp =
      String.fromCharCode(...signature.slice(0, 4)) === "RIFF" &&
      String.fromCharCode(...signature.slice(8, 12)) === "WEBP";
    if (!(
      (file.type === "image/jpeg" && jpeg) ||
      (file.type === "image/png" && png) ||
      (file.type === "image/webp" && webp)
    )) {
      setPhoto(null);
      setSelected(null);
      setMessage("");
      setError(
        "El archivo no es una foto válida. No se permiten videos; usa JPG, PNG o WebP.",
      );
      return;
    }
    setPhoto(file);
    setBusy(true);
    setError("");
    setMessage("");
    setSelected(null);
    try {
      const response = await fetch("/v1/demo/payments/validate-receipt", {
        method: "POST",
        credentials: "same-origin",
        headers: { "content-type": file.type },
        body: file,
      });
      const result = (await response.json()) as {
        payment?: Payment;
        error?: string;
      };
      if (!response.ok || !result.payment) {
        throw new Error(
          response.status === 422
            ? "No se reconoció un código de recibo en la foto. Prueba con una imagen más nítida o escribe el código."
            : response.status === 404
              ? "El recibo se leyó, pero la plataforma demo no tiene ese pago."
              : "No se pudo procesar la foto. Usa JPG, PNG o WebP de hasta 5 MB.",
        );
      }
      setSelected(result.payment);
      setPanel("result");

      setMessage("Comprobante leído.");
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "No se pudo procesar la foto.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="min-h-svh bg-background text-foreground">
      <div className="mx-auto max-w-3xl px-5 py-6 sm:px-8 sm:py-8">
        <header className="flex items-center justify-between gap-4 border-b pb-5">
          <a href="/tramites" className="text-lg font-semibold tracking-tight">
            QORI
          </a>
          <Badge variant="outline">Prototipo</Badge>
        </header>
        <section
          className="mx-auto max-w-lg py-10 sm:py-14"
          aria-labelledby="payment-title"
        >
          <h1
            id="payment-title"
            className="text-3xl font-semibold tracking-tight"
          >
            Consulta tus pagos
          </h1>
          <p className="mt-3 text-sm text-muted-foreground">
            Elige cómo quieres consultar tu comprobante.
          </p>
          <div
            className="mt-7 flex gap-2 border-b pb-3"
            aria-label="Método de consulta"
          >
            <Button
              variant={method === "code" ? "default" : "outline"}
              aria-pressed={method === "code"}
              disabled={busy}
              onClick={() => {
                setMethod("code");
                setError("");
              }}
            >
              Por código
            </Button>
            <Button
              variant={method === "photo" ? "default" : "outline"}
              aria-pressed={method === "photo"}
              disabled={busy}
              onClick={() => {
                setMethod("photo");
                setError("");
              }}
            >
              Subir foto
            </Button>
          </div>
          {method === "code" ? (
            <form onSubmit={validate} className="mt-6 space-y-4">
              <label
                htmlFor="payment-code"
                className="block text-sm font-medium"
              >
                Código del pago o recibo
              </label>
              <input
                id="payment-code"
                autoComplete="off"
                value={code}
                onChange={(event) => setCode(event.target.value)}
                className="h-11 w-full rounded-md border bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                placeholder="Ingresa tu código de pago"
                required
              />
              <Button
                type="submit"
                className="w-full"
                disabled={busy || loading}
              >
                {busy ? "Consultando…" : "Consultar pago"}
              </Button>
            </form>
          ) : (
            <div
              className="mt-6 space-y-4 rounded-md border border-dashed p-5"
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                if (!busy && !loading)
                  void validatePhoto(event.dataTransfer.files[0]);
              }}
              aria-busy={busy}
            >
              <label
                htmlFor="receipt-photo"
                className="block text-sm font-medium"
              >
                O sube una foto del recibo
              </label>
              <input
                id="receipt-photo"
                type="file"
                accept="image/jpeg,image/png,image/webp"
                disabled={busy || loading}
                onChange={(event) => {
                  const file = event.target.files?.[0];
                  event.target.value = "";
                  void validatePhoto(file);
                }}
                className="block w-full min-w-0 text-sm file:mr-3 file:rounded-md file:border file:bg-background file:px-3 file:py-2 file:text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              />
              <p className="text-xs text-muted-foreground">
                Arrastra una imagen aquí. JPG, PNG o WebP · hasta 5 MB.
              </p>
              {preview && (
                <div className="flex items-center gap-3 border-t pt-3">
                  <Image
                    src={preview}
                    alt="Vista previa del recibo seleccionado"
                    width={64}
                    height={80}
                    unoptimized
                    className="h-20 w-16 shrink-0 rounded object-contain"
                  />
                  <p className="min-w-0 break-all text-xs text-muted-foreground">
                    {photo?.name}
                  </p>
                </div>
              )}
            </div>
          )}
          {busy && (
            <p role="status" className="mt-4 text-sm text-muted-foreground">
              Consultando el comprobante…
            </p>
          )}
          {loading && (
            <p role="status" className="mt-4 text-sm text-muted-foreground">
              Consultando la plataforma demo…
            </p>
          )}
          {error && panel === null && (
            <Alert variant="destructive" className="mt-4">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          <div className="mt-7 border-t pt-5">
            <Button
              variant="link"
              className="h-auto p-0"
              onClick={() => {
                setError("");
                setPanel("history");
              }}
            >
              Consultar por DNI
            </Button>
          </div>
        </section>
      </div>
      <Dialog
        open={panel !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPanel(null);
            setError("");
          }
        }}
        title={
          panel === "history"
            ? "Historial de pagos"
            : "Resultado de la consulta"
        }
        description={
          panel === "history"
            ? "Consulta los pagos asociados a tu documento."
            : "Estado del pago y uso del comprobante."
        }
      >
        {panel === "history" ? (
          <div className="space-y-5">
            <form onSubmit={lookupStudent} className="space-y-3">
              <label
                htmlFor="student-dni"
                className="block text-sm font-medium"
              >
                Consultar historial por DNI de prueba
              </label>
              <div className="flex gap-2">
                <input
                  id="student-dni"
                  inputMode="numeric"
                  autoComplete="off"
                  maxLength={8}
                  pattern="[0-9]{8}"
                  value={dni}
                  onChange={(event) => {
                    setDni(
                      event.target.value.replace(/[^0-9]/g, "").slice(0, 8),
                    );
                    setStudentLabel("");
                  }}
                  className="h-10 min-w-0 flex-1 rounded-md border bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  placeholder="Ingresa tu DNI"
                  required
                />
                <Button type="submit" disabled={busy || loading}>
                  Consultar
                </Button>
              </div>
            </form>
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            {studentLabel && (
              <>
                <p role="status" className="text-sm text-muted-foreground">
                  {studentLabel}
                </p>
                {payments.length === 0 && (
                  <p role="status" className="rounded-md border p-4 text-sm">
                    Este estudiante no tiene pagos registrados.
                  </p>
                )}
                <ul className="divide-y border-y">
                  {payments.map((payment) => (
                    <li key={payment.payment_code} className="py-4">
                      <div className="flex items-start justify-between gap-4">
                        <p className="text-sm font-medium">{payment.concept}</p>
                        <p className="shrink-0 text-sm font-semibold tabular-nums">
                          S/ {(payment.amount_cents / 100).toFixed(2)}
                        </p>
                      </div>
                      <div className="mt-2 flex flex-wrap items-center justify-between gap-3">
                        <span className="text-xs text-muted-foreground">
                          {paymentLabel[payment.payment_state]}
                        </span>
                        <Button
                          variant="link"
                          className="h-auto p-0 text-xs"
                          onClick={() => {
                            setSelected(payment);
                            setMessage("");
                            setPanel("result");
                          }}
                        >
                          Ver trazabilidad
                        </Button>
                      </div>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </div>
        ) : (
          selected && (
            <div className="space-y-4">
              {message && (
                <p role="status" className="text-sm text-muted-foreground">
                  {message}
                </p>
              )}
              <p className="text-sm font-medium">{selected.concept}</p>
              <p className="text-3xl font-semibold tabular-nums">
                S/ {(selected.amount_cents / 100).toFixed(2)}
              </p>
              <div className="space-y-2 border-t pt-4">
                <p className="text-sm font-medium">
                  {paymentLabel[selected.payment_state]}
                </p>
                <p className="text-sm text-muted-foreground">
                  {processLabel[selected.process_state]}
                </p>
              </div>
              {selected.voucher_use && (
                <p className="border-t pt-4 text-sm">
                  {selected.voucher_use === "used"
                    ? "Comprobante ya utilizado. No disponible para otro trámite."
                    : selected.voucher_use === "review_required"
                      ? "Reutilización pendiente de revisión administrativa."
                      : "Comprobante disponible."}
                </p>
              )}
            </div>
          )
        )}
      </Dialog>
    </main>
  );
}
