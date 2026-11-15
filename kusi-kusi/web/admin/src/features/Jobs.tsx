import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import React, { useState, useEffect } from "react";
import { Download as DownloadSimple } from "lucide-react";
import { api, assetUrl, type Asset, type Job } from "../api";
import { Badge, Empty, Preview, Approval } from "../ui";
import type { Context } from "../types";
import { useList } from "../hooks";
export default function Jobs(ctx: Context) {
  const jobs = useList<Job>("/v1/creative/jobs", ctx, 3000),
    assets = useList<Asset>("/v1/local/assets", ctx, 3000),
    [id, setId] = useState(ctx.selected),
    [busy, setBusy] = useState(false);
  const job = jobs.find((j) => j.id === id),
    canApprove = ["owner", "approver"].includes(ctx.user.role);
  useEffect(() => {
    if (!id && jobs[0]) setId(jobs[0].id);
  }, [jobs, id]);
  return (
    <>
      <p className="lead">
        Cada trabajo conserva los parámetros exactos aprobados. Costo local: US$
        0.
      </p>
      <div className="split">
        <Card className="record-list panel">
          {jobs.map((j) => (
            <Button
              variant="outline"
              key={j.id}
              className={id === j.id ? "selected" : ""}
              onClick={() => setId(j.id)}
            >
              <span>
                {j.proposal?.input?.kind === "video"
                  ? "Video de producto"
                  : "Diseño de producto"}
                <small>{j.proposal?.input?.prompt || j.id}</small>
              </span>
              <Badge value={j.state} />
            </Button>
          ))}
          {!jobs.length && (
            <Empty>Pide un video o diseño desde Conversaciones.</Empty>
          )}
        </Card>
        <Card className="detail panel">
          {job ? (
            <React.Fragment key={job.id + job.state}>
              <div className="section-line">
                <h2>Revisión del trabajo</h2>
                <Badge value={job.state} />
              </div>
              <p className="muted mono">{job.id}</p>
              <dl>
                <dt>Solicitud</dt>
                <dd>{job.proposal?.input?.prompt}</dd>
                <dt>Motor</dt>
                <dd>
                  {job.proposal?.provider} / {job.proposal?.model}
                </dd>
                <dt>Costo máximo</dt>
                <dd>
                  US${" "}
                  {(job.proposal.estimated_cost_microusd / 1e6 || 0).toFixed(2)}
                </dd>
                <dt>Versión aprobable</dt>
                <dd className="mono digest">{job.approval_digest}</dd>
              </dl>
              {assets.find((a) => a.creative_job_id === job.id) && (
                <div className="result-preview">
                  <Preview
                    asset={assets.find((a) => a.creative_job_id === job.id)!}
                  />
                  <a className="text-link" href={assetUrl(job.id)} download>
                    Descargar resultado <DownloadSimple />
                  </a>
                </div>
              )}
              {job.state === "succeeded" &&
                !assets.some((a) => a.creative_job_id === job.id) && (
                  <p role="status">Archivando la vista previa…</p>
                )}
              {job.state === "awaiting_approval" && (
                <Approval
                  disabled={!canApprove || busy}
                  onApprove={() => {
                    setBusy(true);
                    void ctx
                      .run(async () => {
                        await api(
                          `/v1/creative/jobs/${job.id}/approve`,
                          "POST",
                          {
                            confirm: true,
                            approval_digest: job.approval_digest,
                            max_cost_microusd:
                              job.proposal.estimated_cost_microusd || 0,
                          },
                        );
                        ctx.updated();
                      })
                      .finally(() => setBusy(false));
                  }}
                />
              )}
              {["awaiting_approval", "approved"].includes(job.state) &&
                ["owner", "editor", "approver"].includes(ctx.user.role) && (
                  <Button
                    variant="outline"
                    className="danger"
                    onClick={() =>
                      void ctx.run(async () => {
                        await api(`/v1/creative/jobs/${job.id}/cancel`, "POST");
                        ctx.updated();
                      })
                    }
                  >
                    Cancelar trabajo
                  </Button>
                )}
              {!canApprove && job.state === "awaiting_approval" && (
                <p className="hint">
                  Un propietario o aprobador debe confirmar este trabajo.
                </p>
              )}
              <p className="hint">
                La vista previa anima o adapta el archivo original. No
                representa un video 360 reconstruido ni una generación de IA.
              </p>
            </React.Fragment>
          ) : (
            <Empty>Selecciona un trabajo.</Empty>
          )}
        </Card>
      </div>
    </>
  );
}
