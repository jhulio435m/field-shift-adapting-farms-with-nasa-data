import { useEffect, useState } from "react";
import { api } from "@/api";
export function useManagedData<T>(path: string, revision: number, initial: T) {
  const [data, setData] = useState<T>(initial);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    void api<T>(path)
      .then((value) => {
        if (active) setData(value);
      })
      .catch((cause: unknown) => {
        if (active)
          setError(
            cause instanceof Error
              ? cause.message
              : "No se pudo cargar. Actualiza e inténtalo de nuevo.",
          );
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [path, revision]);
  return { data, loading, error };
}
