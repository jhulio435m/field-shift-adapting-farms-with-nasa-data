import { useState, useEffect } from "react";
import { api } from "./api";
import type { Context } from "./types";
export function useList<T>(path: string, ctx: Context, poll = 0) {
  const [data, setData] = useState<T[]>([]);
  useEffect(() => {
    let live = true;
    const load = () =>
      api<T[]>(path).then((v) => {
        if (live) setData(v);
      });
    void ctx.run(load);
    const timer = poll
      ? setInterval(() => {
          void load().catch(() => {});
        }, poll)
      : undefined;
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, [path, ctx.revision]);
  return data;
}
