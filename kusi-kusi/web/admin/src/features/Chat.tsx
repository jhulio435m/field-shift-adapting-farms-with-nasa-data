import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { Badge as StatusBadge } from "@/components/ui/badge";
import { useState, useEffect, useRef } from "react";
import {
  MessageCircle as ChatCircleDots,
  ArrowUpRight,
  Plus,
  Send as PaperPlaneTilt,
} from "lucide-react";
import { api, date, type Asset, type Conversation, type Message } from "../api";
import { Badge, Empty } from "../ui";
import type { Context } from "../types";
import { useList } from "../hooks";
export default function Chat(ctx: Context) {
  const conversations = useList<Conversation>("/v1/local/conversations", ctx),
    assets = useList<Asset>("/v1/local/assets", ctx, 5000),
    [id, setId] = useState(""),
    [messages, setMessages] = useState<Message[]>([]),
    [pending, setPending] = useState(0),
    [content, setContent] = useState(""),
    [asset, setAsset] = useState(""),
    [sending, setSending] = useState(false);
  const end = useRef<HTMLDivElement>(null);
  const edit = ctx.user.role === "owner" || ctx.user.role === "editor";
  useEffect(() => {
    if (!id && conversations[0]) setId(conversations[0].id);
  }, [conversations, id]);
  useEffect(() => {
    if (!id) return;
    let live = true;
    const load = () =>
      api<{ messages: Message[]; pending: number }>(
        `/v1/local/conversations/${id}/messages`,
      ).then((v) => {
        if (live) {
          setMessages(v.messages);
          setPending(v.pending);
        }
      });
    void ctx.run(load);
    const timer = setInterval(() => void load().catch(() => {}), 2000);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, [id, ctx.revision]);
  useEffect(() => {
    end.current?.scrollIntoView({ block: "nearest" });
  }, [messages.length, pending]);
  const send = () => {
    if (!id || !content.trim() || sending) return;
    const value = content;
    setSending(true);
    void ctx
      .run(async () => {
        await api(`/v1/local/conversations/${id}/messages`, "POST", {
          content: value,
          asset_id: asset || null,
          client_id: crypto.randomUUID(),
        });
        setContent("");
        setAsset("");
        ctx.updated();
      })
      .finally(() => setSending(false));
  };
  return (
    <>
      <p className="lead">
        Este chat prepara contenido creativo local. Para probar Plane, ERP
        CONEIMERA o Telegram, abre la consulta guiada.
      </p>
      <Button variant="outline" className="mb-5" onClick={() => ctx.go("home")}>
        Probar una consulta del piloto
      </Button>
      <div className="chat-layout">
        <section className="conversation-list">
          <div className="section-line">
            <h2>Recientes</h2>
            {edit && (
              <Button
                variant="outline"
                className="icon-button"
                aria-label="Nueva conversación"
                onClick={() =>
                  void ctx.run(async () => {
                    const c = await api<Conversation>(
                      "/v1/local/conversations",
                      "POST",
                      { title: `Nueva solicitud ${conversations.length + 1}` },
                    );
                    setId(c.id);
                    ctx.updated();
                  })
                }
              >
                <Plus size={20} />
              </Button>
            )}
          </div>
          {conversations.map((c) => (
            <Button
              variant="outline"
              className={id === c.id ? "selected" : ""}
              key={c.id}
              onClick={() => setId(c.id)}
            >
              <ChatCircleDots size={19} />
              <span>
                {c.title}
                <small>{c.local ? "Canal local" : "Canal conectado"}</small>
              </span>
            </Button>
          ))}
          {!conversations.length && (
            <Empty>Crea una conversación para comenzar.</Empty>
          )}
        </section>
        <section className="conversation">
          <div className="conversation-heading">
            <h2>
              {conversations.find((c) => c.id === id)?.title ||
                "Comienza una solicitud"}
            </h2>
            <Badge value="Asistente local" />
          </div>
          <div className="messages" aria-live="polite">
            {!messages.length && (
              <div className="chat-welcome">
                <ChatCircleDots size={36} />
                <h2>¿Qué quieres preparar?</h2>
                <p>
                  Adjunta un producto y pide un video o un diseño. Cada acción
                  se guarda como borrador antes de ejecutarse.
                </p>
                <div className="suggestions">
                  {[
                    "Crea un video de este producto",
                    "Haz un diseño",
                    "Prepara para Instagram",
                  ].map((s) => (
                    <Button
                      variant="outline"
                      key={s}
                      onClick={() => setContent(s)}
                      disabled={!edit}
                    >
                      {s}
                    </Button>
                  ))}
                </div>
                <small>
                  Interpretación por reglas. Las vistas previas usan FFmpeg y no
                  son generación con IA.
                </small>
              </div>
            )}
            {messages.map((m) => (
              <article className={`message ${m.direction}`} key={m.id}>
                <span>{m.direction === "in" ? "Solicitud" : "QORI"}</span>
                {m.direction === "out" && m.metadata.simulated && (
                  <StatusBadge variant="secondary">
                    Respuesta simulada · datos ficticios
                  </StatusBadge>
                )}
                <p>{m.content}</p>
                {m.metadata.creative_job_id && (
                  <Button
                    variant="outline"
                    className="text-link"
                    onClick={() => ctx.go("jobs", m.metadata.creative_job_id)}
                  >
                    Revisar trabajo <ArrowUpRight />
                  </Button>
                )}
                {m.metadata.publication_id && (
                  <Button
                    variant="outline"
                    className="text-link"
                    onClick={() =>
                      ctx.go("publications", m.metadata.publication_id)
                    }
                  >
                    Revisar publicación <ArrowUpRight />
                  </Button>
                )}
                <small>{date(m.created_at)}</small>
              </article>
            ))}
            {pending > 0 && (
              <p className="pending" role="status">
                Agrupando {pending} mensaje(s) y procesando…
              </p>
            )}
            <div ref={end} />
          </div>
          <form
            className="composer"
            onSubmit={(e) => {
              e.preventDefault();
              send();
            }}
          >
            <Label className="sr-only" htmlFor="message">
              Tu solicitud
            </Label>
            <Textarea
              id="message"
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder="Escribe una solicitud…"
              required
              maxLength={4000}
              disabled={!id || !edit}
            />
            <div className="composer-bottom">
              <Label>
                Archivo
                <NativeSelect
                  aria-label="Adjuntar archivo"
                  value={asset}
                  onChange={(e) => setAsset(e.target.value)}
                  disabled={!edit}
                >
                  <option value="">Usar contexto / sin adjunto</option>
                  {assets.map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.label}
                      {a.simulated ? " · simulación" : ""}
                    </option>
                  ))}
                </NativeSelect>
              </Label>
              <Button
                variant="outline"
                className="primary"
                disabled={!id || !edit || sending || !content.trim()}
              >
                <PaperPlaneTilt size={18} />
                {sending ? "Enviando…" : "Enviar"}
              </Button>
            </div>
          </form>
        </section>
      </div>
    </>
  );
}
