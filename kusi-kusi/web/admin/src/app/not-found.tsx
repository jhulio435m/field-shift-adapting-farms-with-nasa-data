import Link from "next/link";
import { Button } from "@/components/ui/button";
export default function NotFound() {
  return (
    <main className="mx-auto max-w-lg space-y-4 p-8">
      <h1 className="text-2xl font-semibold">Esta sección no existe</h1>
      <p className="text-muted-foreground">
        Vuelve al espacio de trabajo para continuar.
      </p>
      <Button asChild>
        <Link href="/conversaciones">Ir a conversaciones</Link>
      </Button>
    </main>
  );
}
