import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { useState, useEffect } from "react";
import { ArrowUpRight } from "lucide-react";
import { api } from "../api";
import { Badge, Empty } from "../ui";
import type { Context } from "../types";
import ConnectionSettings from "./ConnectionSettings";
export default function Connections(ctx: Context) {
  const [settings, setSettings] = useState<{
      channels: { id: string; provider: string }[];
      llm_profiles: string[];
      audio_profiles: string[];
    } | null>(null),
    [code, setCode] = useState("");
  useEffect(() => {
    void ctx.run(async () => setSettings(await api("/v1/local/settings")));
  }, [ctx.revision]);
  return (
    <>
      <p className="lead">
        Las credenciales se mantienen en el servidor. Este entorno no realiza
        llamadas a proveedores.
      </p>
      <ConnectionSettings {...ctx} />
      <div className="connection-grid">
        <Card className="panel">
          <p className="eyebrow">MENSAJERÍA</p>
          <h2>Canales activos del servidor</h2>
          {settings?.channels.map((c) => (
            <div className="connection-row" key={c.id}>
              <strong className="capitalize">{c.provider}</strong>
              <code>{c.id}</code>
              {["owner", "editor"].includes(ctx.user.role) && (
                <Button
                  variant="outline"
                  onClick={() =>
                    void ctx.run(async () => {
                      const result = await api<{ code: string }>(
                        "/v1/local/link-codes",
                        "POST",
                        { channel_id: c.id },
                      );
                      setCode(result.code);
                    })
                  }
                >
                  Generar código de vinculación
                </Button>
              )}
            </div>
          ))}
          {!settings?.channels.length && (
            <Empty>
              No hay canales activos. La configuración guardada arriba no se
              activa ni envía mensajes en modo local.
            </Empty>
          )}
          {code && (
            <div className="notice">
              Envía <code>/vincular {code}</code> al bot registrado. Es de un
              solo uso y vence en diez minutos.
            </div>
          )}
          <p className="hint">
            El teléfono o identificador del chat por sí solo no acredita
            identidad.
          </p>
        </Card>
        <Card className="panel">
          <p className="eyebrow">INTELIGENCIA ARTIFICIAL</p>
          <h2>Perfiles por organización</h2>
          <dl>
            <dt>Texto</dt>
            <dd>
              {settings?.llm_profiles.join(", ") || "Sin perfiles configurados"}
            </dd>
            <dt>Audio</dt>
            <dd>
              {settings?.audio_profiles.join(", ") ||
                "Sin perfiles configurados"}
            </dd>
            <dt>Motor local</dt>
            <dd>Comandos deterministas y FFmpeg</dd>
          </dl>
          <p className="hint">
            Los adaptadores de proveedores se configuran en el backend. No se
            transcriben audios ni se generan datos empresariales en este modo.
          </p>
        </Card>
        <Card className="panel">
          <p className="eyebrow">CUENTAS SOCIALES</p>
          <h2>OAuth</h2>
          <Badge value="Desactivado en modo local" />
          <p>
            Las conexiones reales requieren aplicaciones oficiales, credenciales
            y una URL HTTPS autorizada. El calendario local conserva borradores
            y comprobantes simulados.
          </p>
          <a
            href="/openapi.yaml"
            className="text-link"
            target="_blank"
            rel="noreferrer"
          >
            Consultar contrato API <ArrowUpRight />
          </a>
        </Card>
      </div>
    </>
  );
}
