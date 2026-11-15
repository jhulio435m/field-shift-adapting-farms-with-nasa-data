"use client";
import { Button } from "@/components/ui/button";
export default function ErrorPage({ reset }: { reset: () => void }) {
  return (
    <main className="mx-auto max-w-lg space-y-4 p-8">
      <h1 className="text-xl font-semibold">No se pudo abrir esta sección</h1>
      <p className="text-muted-foreground">
        Vuelve a intentarlo. Las operaciones guardadas permanecen en el
        servidor.
      </p>
      <Button onClick={reset}>Reintentar</Button>
    </main>
  );
}
