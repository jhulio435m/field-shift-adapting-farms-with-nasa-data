import {
  Collapsible,
  CollapsibleTrigger,
  CollapsibleContent,
} from "@/components/ui/collapsible";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { Card } from "@/components/ui/card";
import React, { useState, useEffect } from "react";
import { CalendarDays as CalendarDots, Check } from "lucide-react";
import { api, date, type Asset, type Publication } from "../api";
import { Badge, Empty, Preview, Approval, getForm, text } from "../ui";
import type { Context } from "../types";
import { useList } from "../hooks";
export default function Publications(ctx: Context) {
  const pubs = useList<Publication>("/v1/local/publications", ctx, 3000),
    assets = useList<Asset>("/v1/local/assets", ctx),
    [id, setId] = useState(ctx.selected),
    [calendar, setCalendar] = useState(false);
  const p = pubs.find((p) => p.id === id),
    edit = ["owner", "editor"].includes(ctx.user.role),
    approve = ["owner", "approver"].includes(ctx.user.role);
  useEffect(() => {
    if (!id && pubs[0]) setId(pubs[0].id);
  }, [pubs, id]);
  return (
    <>
      <div className="section-line">
        <p className="lead">
          Prepara, programa y aprueba. Los comprobantes de este entorno son
          simulados.
        </p>
        <Button onClick={() => setCalendar((v) => !v)}>
          <CalendarDots size={18} />
          {calendar ? "Ver lista" : "Ver calendario"}
        </Button>
      </div>
      {edit && (
        <Collapsible className="panel">
          <CollapsibleTrigger asChild>
            <Button variant="ghost" className="justify-start p-0">
              Crear un borrador manual
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className="pt-4">
            <form
              className="form-grid"
              onSubmit={(e) => {
                const f = getForm(e);
                void ctx.run(async () => {
                  const p = await api<Publication>(
                    "/v1/local/publications",
                    "POST",
                    {
                      asset_id: text(f, "asset_id"),
                      platform: text(f, "platform"),
                      caption: text(f, "caption"),
                      privacy: "private",
                      scheduled_at: null,
                      idempotency_key: crypto.randomUUID(),
                    },
                  );
                  setId(p.id);
                  ctx.updated();
                });
              }}
            >
              <Label>
                Archivo
                <NativeSelect name="asset_id" required>
                  <option value="">Seleccionar</option>
                  {assets.map((a) => (
                    <option value={a.id} key={a.id}>
                      {a.label}
                    </option>
                  ))}
                </NativeSelect>
              </Label>
              <Label>
                Destino
                <NativeSelect name="platform">
                  <option value="instagram">Instagram</option>
                  <option value="facebook">Facebook</option>
                  <option value="tiktok">TikTok</option>
                </NativeSelect>
              </Label>
              <Label className="wide">
                Texto
                <Textarea name="caption" required maxLength={2200} />
              </Label>
              <Button
                variant="outline"
                className="primary"
                disabled={!assets.length}
              >
                Crear borrador
              </Button>
            </form>
          </CollapsibleContent>
        </Collapsible>
      )}
      {calendar && (
        <Card className="calendar panel">
          <h2>Agenda de publicaciones</h2>
          {[...pubs]
            .sort((a, b) => (a.scheduled_at || 0) - (b.scheduled_at || 0))
            .map((p) => (
              <Button key={p.id} onClick={() => setId(p.id)}>
                <time>{date(p.scheduled_at)}</time>
                <strong>{p.platform}</strong>
                <span>{p.caption}</span>
                <Badge value={p.state} />
              </Button>
            ))}
          {!pubs.length && <Empty>No hay publicaciones programadas.</Empty>}
        </Card>
      )}
      <div className="split">
        <Card className="record-list panel">
          {pubs.map((p) => (
            <Button
              variant="outline"
              key={p.id}
              className={id === p.id ? "selected" : ""}
              onClick={() => setId(p.id)}
            >
              <span className="capitalize">
                {p.platform}
                <small>{p.caption}</small>
              </span>
              <Badge value={p.state} />
            </Button>
          ))}
          {!pubs.length && (
            <Empty>
              Prepara una publicación desde la conversación o crea un borrador
              manual.
            </Empty>
          )}
        </Card>
        <Card className="detail panel">
          {p ? (
            <React.Fragment key={p.id + ":" + p.revision + ":" + p.state}>
              <div className="section-line">
                <h2 className="capitalize">
                  {p.platform} · versión {p.revision}
                </h2>
                <Badge value={p.state} />
              </div>
              {assets.find((a) => a.id === p.asset_id) && (
                <div className="publication-preview">
                  <Preview asset={assets.find((a) => a.id === p.asset_id)!} />
                </div>
              )}
              <form
                className="stack"
                onSubmit={(e) => {
                  const f = getForm(e);
                  void ctx.run(async () => {
                    await api(`/v1/local/publications/${p.id}`, "PATCH", {
                      asset_id: text(f, "asset_id"),
                      platform: p.platform,
                      caption: text(f, "caption"),
                      privacy: text(f, "privacy"),
                      scheduled_at: text(f, "scheduled_at")
                        ? Math.floor(
                            new Date(text(f, "scheduled_at")).getTime() / 1000,
                          )
                        : null,
                      idempotency_key: p.idempotency_key,
                    });
                    ctx.updated();
                    ctx.setNotice(
                      "Nueva versión guardada. Requiere una nueva aprobación.",
                    );
                  });
                }}
              >
                <Label>
                  Archivo
                  <NativeSelect
                    name="asset_id"
                    defaultValue={p.asset_id}
                    disabled={
                      !edit ||
                      !["draft", "approved", "scheduled", "failed"].includes(
                        p.state,
                      )
                    }
                  >
                    {assets.map((a) => (
                      <option value={a.id} key={a.id}>
                        {a.label}
                      </option>
                    ))}
                  </NativeSelect>
                </Label>
                <Label>
                  Texto
                  <Textarea
                    name="caption"
                    defaultValue={p.caption}
                    maxLength={2200}
                    required
                    disabled={
                      !edit ||
                      !["draft", "approved", "scheduled", "failed"].includes(
                        p.state,
                      )
                    }
                  />
                </Label>
                <div className="form-grid">
                  <Label>
                    Privacidad
                    <NativeSelect
                      name="privacy"
                      defaultValue={p.privacy}
                      disabled={
                        !edit ||
                        !["draft", "approved", "scheduled", "failed"].includes(
                          p.state,
                        )
                      }
                    >
                      <option value="private">Privada</option>
                      <option value="friends">Amigos</option>
                      <option value="public">Pública</option>
                    </NativeSelect>
                  </Label>
                  <Label>
                    Programar (hora local)
                    <Input
                      name="scheduled_at"
                      type="datetime-local"
                      defaultValue={
                        p.scheduled_at
                          ? new Date(
                              p.scheduled_at * 1000 -
                                new Date().getTimezoneOffset() * 60000,
                            )
                              .toISOString()
                              .slice(0, 16)
                          : ""
                      }
                      disabled={
                        !edit ||
                        !["draft", "approved", "scheduled", "failed"].includes(
                          p.state,
                        )
                      }
                    />
                  </Label>
                </div>
                {edit &&
                  ["draft", "approved", "scheduled", "failed"].includes(
                    p.state,
                  ) && <Button variant="outline">Guardar nueva versión</Button>}
              </form>
              <p className="hint">
                Fecha: {date(p.scheduled_at)} · Privacidad: {p.privacy}
              </p>
              <p className="mono digest">{p.approval_digest}</p>
              {p.state === "draft" && (
                <Approval
                  disabled={!approve}
                  text="Revisé el archivo, texto, destino, privacidad y horario de esta versión."
                  onApprove={() =>
                    void ctx.run(async () => {
                      await api(
                        `/v1/local/publications/${p.id}/approve`,
                        "POST",
                        { confirm: true, approval_digest: p.approval_digest },
                      );
                      ctx.updated();
                    })
                  }
                />
              )}
              {(edit || approve) &&
                ["draft", "approved", "scheduled"].includes(p.state) && (
                  <Button
                    variant="outline"
                    className="danger"
                    onClick={() =>
                      void ctx.run(async () => {
                        await api(
                          `/v1/local/publications/${p.id}/cancel`,
                          "POST",
                        );
                        ctx.updated();
                      })
                    }
                  >
                    Cancelar publicación
                  </Button>
                )}
              {p.receipt && (
                <p className="receipt">
                  <Check />
                  Comprobante local: <code>{p.receipt}</code>
                </p>
              )}
            </React.Fragment>
          ) : (
            <Empty>Selecciona una publicación para revisar su versión.</Empty>
          )}
        </Card>
      </div>
    </>
  );
}
