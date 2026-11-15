import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useState } from "react";
import { Download as DownloadSimple } from "lucide-react";
import { api, upload, assetUrl, type Asset } from "../api";
import { Empty, Preview, Confirmation } from "../ui";
import type { Context } from "../types";
import { useList } from "../hooks";
export default function Assets(ctx: Context) {
  const assets = useList<Asset>("/v1/local/assets", ctx),
    [file, setFile] = useState<File | null>(null),
    [name, setName] = useState(""),
    [busy, setBusy] = useState(false),
    [deleting, setDeleting] = useState<Asset | null>(null);
  const edit = ["owner", "editor"].includes(ctx.user.role);
  return (
    <>
      <p className="lead">
        Archivos privados de tu organización. Las imágenes se validan y se
        guardan sin metadatos.
      </p>
      {edit && (
        <form
          className="panel upload-form"
          onSubmit={(e) => {
            e.preventDefault();
            if (!file) return;
            setBusy(true);
            void ctx
              .run(async () => {
                await upload(file, name);
                setFile(null);
                setName("");
                ctx.updated();
                ctx.setNotice("Archivo guardado y validado.");
              })
              .finally(() => setBusy(false));
          }}
        >
          <Label>
            Nombre del producto o pieza
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={100}
              required
              placeholder="Ej. Café de origen"
            />
          </Label>
          <Label>
            Imagen o video · hasta 10 MB
            <Input
              type="file"
              accept="image/png,image/jpeg,image/webp,video/mp4,video/webm"
              onChange={(e) => setFile(e.target.files?.[0] || null)}
              required
            />
          </Label>
          <Button
            variant="outline"
            className="primary"
            disabled={!file || busy}
          >
            {busy ? "Validando…" : "Guardar archivo"}
          </Button>
        </form>
      )}
      <div className="asset-grid">
        {assets.map((a) => (
          <Card className="asset-card gap-0 py-0 shadow-none" key={a.id}>
            <div className="asset-preview">
              <Preview asset={a} />
              {a.simulated && (
                <span className="simulation-stamp">SIMULACIÓN LOCAL</span>
              )}
            </div>
            <div className="asset-details">
              <h2>{a.label}</h2>
              <small>
                {(a.size_bytes / 1024).toFixed(0)} KB · {a.media_type}
              </small>
              <div className="actions">
                <a href={assetUrl(a.id)} download={a.filename}>
                  <DownloadSimple size={17} />
                  Descargar
                </a>
                {edit && (
                  <Button
                    variant="outline"
                    className="danger text-link"
                    onClick={() => setDeleting(a)}
                  >
                    Eliminar
                  </Button>
                )}
              </div>
            </div>
          </Card>
        ))}
      </div>
      {!assets.length && (
        <Empty>
          Aún no hay archivos. Sube una imagen para iniciar el recorrido
          creativo.
        </Empty>
      )}
      <Confirmation
        open={!!deleting}
        onOpenChange={(open) => {
          if (!open) setDeleting(null);
        }}
        title="Eliminar archivo"
        description={`Se eliminará «${deleting?.label || ""}». Los archivos usados por publicaciones o trabajos se conservarán.`}
        onConfirm={() => {
          if (deleting)
            void ctx.run(async () => {
              await api(`/v1/local/assets/${deleting.id}`, "DELETE");
              setDeleting(null);
              ctx.updated();
            });
        }}
      />
    </>
  );
}
