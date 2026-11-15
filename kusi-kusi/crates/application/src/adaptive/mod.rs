//! Adaptive use cases. Knowledge and model proposals never grant authority.
mod access;
mod render;
pub use access::{AuthorizedEnvironment, ResourceBinding};

pub use render::render;

use async_trait::async_trait;
use chrono::NaiveDate;
use kusi_domain::adaptive::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct Actor {
    pub organization: Uuid,
    pub principal: String,
    pub read: bool,
    pub subscribe: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub resource: String,
    pub event: String,
    pub filters: Vec<Filter>,
    pub status: String,
    pub version: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    #[serde(default)]
    pub test: bool,
    pub resource: String,
    pub event_id: String,
    pub kind: String,
    pub values: std::collections::BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub version: String,
    pub reply: Reply,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEntry {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub source: String,
    pub resources: Vec<String>,
    pub version: String,
}
#[async_trait]
pub trait Environment: Send + Sync {
    async fn discover(&self, actor: &Actor) -> Result<Catalog, Error>;
    async fn read(
        &self,
        actor: &Actor,
        catalog: &Catalog,
        query: &Query,
        limit: usize,
    ) -> Result<Batch, Error>;
    async fn watch(&self, actor: &Actor, resource: &Resource) -> Result<(), Error>;
    async fn authorize_event(&self, actor: &Actor, event: &Event) -> Result<(), Error>;
}
#[async_trait]
pub trait Planner: Send + Sync {
    async fn propose(
        &self,
        catalog: &Catalog,
        context: &Context,
        message: &str,
        today: NaiveDate,
    ) -> Result<Decision, Error>;
}
#[async_trait]
pub trait State: Send + Sync {
    async fn context(&self, actor: &Actor) -> Result<Context, Error>;
    async fn save_context(&self, actor: &Actor, context: &Context) -> Result<(), Error>;
    async fn claim(&self, actor: &Actor, request: &str) -> Result<bool, Error>;
    async fn receipt(&self, actor: &Actor, request: &str) -> Result<Option<Receipt>, Error>;
    async fn finish(&self, actor: &Actor, request: &str, receipt: &Receipt) -> Result<(), Error>;
    async fn subscriptions(&self, actor: &Actor) -> Result<Vec<Subscription>, Error>;
    async fn save_subscription(
        &self,
        actor: &Actor,
        subscription: &Subscription,
    ) -> Result<(), Error>;
    async fn claim_event(
        &self,
        actor: &Actor,
        subscription: &str,
        event: &str,
    ) -> Result<bool, Error>;
    async fn record_audit(&self, actor: &Actor, entry: &AuditEntry) -> Result<(), Error>;
    async fn audits(&self, actor: &Actor) -> Result<Vec<AuditEntry>, Error>;
}

pub struct Assistant<E, P, S> {
    pub environment: E,
    pub planner: P,
    pub state: S,
}
impl<E: Environment, P: Planner, S: State> Assistant<E, P, S> {
    pub async fn handle(
        &self,
        actor: &Actor,
        message: &str,
        request: &str,
        today: NaiveDate,
    ) -> Result<Reply, Error> {
        if !actor.read {
            return Err(Error::Forbidden);
        }
        if message.trim().is_empty()
            || message.len() > 4096
            || request.is_empty()
            || request.len() > 160
        {
            return Err(Error::Invalid);
        }
        let catalog = self.environment.discover(actor).await?;
        if !self.state.claim(actor, request).await? {
            return match self.state.receipt(actor, request).await? {
                Some(receipt) if receipt.version == catalog.version => Ok(receipt.reply),
                _ => Ok(Reply::message(
                    "unknown",
                    "Esta solicitud ya se inició o cambió su contexto de acceso. No la repetiré automáticamente; envía una petición nueva para revisarla.",
                )),
            };
        }
        let result = self.process(actor, message, today, &catalog).await;
        let reply = match result {
            Ok(reply) => reply,
            Err(Error::Forbidden) => Reply::message(
                "blocked",
                "El recurso o los campos solicitados no están disponibles con tus permisos actuales. Puedo revisar las capacidades accesibles o aclarar qué acceso falta.",
            ),
            Err(Error::Invalid) => Reply::message(
                "clarification",
                "El plan no coincide con los campos, tipos o fechas disponibles en el entorno actual. Aclara el recurso, el periodo o la medida que quieres utilizar.",
            ),
            Err(Error::Incomplete) => Reply::message(
                "clarification",
                "Los datos son incompletos, exceden el límite de lectura o no contienen un periodo completo para el cálculo. Acota la consulta o indica otro periodo; no presentaré una cifra parcial como completa.",
            ),
            Err(Error::Limited) => Reply::message(
                "deferred",
                "El proveedor alcanzó su límite de uso. Conservé el contexto; espera un minuto y vuelve a pedirlo. No es una negativa a tu objetivo ni ejecuté otra operación.",
            ),
            Err(Error::Unavailable) => Reply::message(
                "unavailable",
                "La fuente, el intérprete o el almacenamiento no están disponibles. No se obtuvo un resultado comprobable; la solicitud no se reintentará automáticamente.",
            ),
        };
        let resources = reply
            .evidence
            .iter()
            .map(|e| e.resource.clone())
            .collect::<Vec<_>>();
        self.state
            .record_audit(
                actor,
                &AuditEntry {
                    id: request.into(),
                    kind: "message".into(),
                    status: reply.status.clone(),
                    source: catalog.source.clone(),
                    resources,
                    version: catalog.version.clone(),
                },
            )
            .await?;
        self.state
            .finish(
                actor,
                request,
                &Receipt {
                    version: catalog.version.clone(),
                    reply: reply.clone(),
                },
            )
            .await?;
        Ok(reply)
    }
    async fn process(
        &self,
        actor: &Actor,
        message: &str,
        today: NaiveDate,
        catalog: &Catalog,
    ) -> Result<Reply, Error> {
        let mut context = self.state.context(actor).await?;
        let changed = context.schemas.iter().any(|old| {
            catalog
                .resource(&old.id)
                .ok()
                .and_then(|new| serde_json::to_value(new).ok())
                != serde_json::to_value(old).ok()
        });
        let stale_draft = context
            .draft
            .as_ref()
            .is_some_and(|draft| catalog.resource(&draft.resource).is_err());
        if changed || stale_draft {
            context = Context::default();
        }
        let prior_len = context.interpretations.len();
        context
            .interpretations
            .retain(|item| !changed && catalog.resource(&item.resource).is_ok());
        for item in &mut context.interpretations {
            item.version = catalog.version.clone();
        }
        let mut invalidated = changed || stale_draft || context.interpretations.len() != prior_len;
        if let Some(previous) = context.previous.as_mut() {
            previous.version = catalog.version.clone();
            if changed || catalog.validate(previous, today).is_err() {
                context = Context::default();
                invalidated = true;
            }
        }
        if invalidated {
            self.state.save_context(actor, &context).await?;
        }
        context.subscriptions = self
            .state
            .subscriptions(actor)
            .await?
            .into_iter()
            .filter(|s| {
                s.status != "cancelled"
                    && catalog.resource(&s.resource).is_ok_and(|resource| {
                        resource.events.contains(&s.event)
                            && resource.validate_filters(&s.filters).is_ok()
                    })
            })
            .map(|s| SubscriptionHint {
                id: s.id,
                resource: s.resource,
                event: s.event,
                filters: s.filters,
                status: s.status,
            })
            .collect();
        let decision = self
            .planner
            .propose(catalog, &context, message, today)
            .await?;
        match decision {
            Decision::Clarify { question, draft } => {
                if question.is_empty() || question.len() > 600 {
                    return Err(Error::Invalid);
                }
                context.question = Some(question.clone());
                context.draft = draft;
                self.state.save_context(actor, &context).await?;
                Ok(Reply::message("clarification", question))
            }
            Decision::Unsupported {
                capability,
                explanation,
                requires_approval,
            } => {
                if capability.trim().is_empty()
                    || capability.len() > 160
                    || explanation.trim().is_empty()
                    || explanation.len() > 600
                {
                    return Err(Error::Invalid);
                }
                let status = if requires_approval {
                    "blocked"
                } else {
                    "clarification"
                };
                Ok(Reply::message(
                    status,
                    format!("{explanation} (capacidad requerida: {capability})"),
                ))
            }
            Decision::Reset {} => {
                self.state.save_context(actor, &Context::default()).await?;
                Ok(Reply::message(
                    "completed",
                    "Contexto de la conversación reiniciado. Las suscripciones conservan su estado hasta que pidas cancelarlas.",
                ))
            }
            Decision::Cancel { subscription } => {
                if !actor.subscribe {
                    return Err(Error::Forbidden);
                }
                let mut existing = self
                    .state
                    .subscriptions(actor)
                    .await?
                    .into_iter()
                    .find(|s| s.id == subscription)
                    .ok_or(Error::Forbidden)?;
                existing.status = "cancelled".into();
                self.state.save_subscription(actor, &existing).await?;
                Ok(Reply::message(
                    "completed",
                    format!(
                        "Suscripción {} cancelada. No se entregarán nuevos avisos de esa suscripción.",
                        existing.id
                    ),
                ))
            }
            Decision::Subscribe {
                resource,
                event,
                filters,
            } => {
                if !actor.subscribe {
                    return Err(Error::Forbidden);
                }
                let current = self.environment.discover(actor).await?;
                if current.version != catalog.version {
                    return Err(Error::Invalid);
                }
                let resource = current.resource(&resource)?;
                resource.validate_filters(&filters)?;
                if !resource.events.contains(&event) {
                    return Ok(Reply::message(
                        "clarification",
                        "Esta fuente no ofrece ese evento para el recurso elegido. No lo sustituiré por consultas periódicas sin acordarlo.",
                    ));
                }
                let existing = self.state.subscriptions(actor).await?;
                if let Some(s) = existing.iter().find(|s| {
                    matches!(s.status.as_str(), "active" | "pending")
                        && s.resource == resource.id
                        && s.event == event
                        && serde_json::to_value(&s.filters).ok()
                            == serde_json::to_value(&filters).ok()
                }) {
                    if s.status == "pending" {
                        self.environment.watch(actor, resource).await?;
                        let mut resumed = s.clone();
                        resumed.status = "active".into();
                        resumed.version = current.version.clone();
                        self.state.save_subscription(actor, &resumed).await?;
                    }
                    return Ok(Reply::message(
                        "completed",
                        format!(
                            "La suscripción {} ya está activa para {}. No crearé un aviso duplicado.",
                            s.id, resource.label
                        ),
                    ));
                }
                if existing.iter().filter(|s| s.status != "cancelled").count() >= 32 {
                    return Err(Error::Incomplete);
                }
                let mut subscription = Subscription {
                    id: Uuid::new_v4().to_string(),
                    resource: resource.id.clone(),
                    event,
                    filters,
                    status: "pending".into(),
                    version: catalog.version.clone(),
                };
                self.state.save_subscription(actor, &subscription).await?;
                self.environment.watch(actor, resource).await?;
                subscription.status = "active".into();
                self.state.save_subscription(actor, &subscription).await?;
                Ok(Reply::message(
                    "completed",
                    format!(
                        "Avisos activados para {}. Suscripción {}. Puedes pedirme cancelarla. Los permisos se comprobarán antes de cada entrega.",
                        resource.label, subscription.id
                    ),
                ))
            }
            Decision::Query { plan } => {
                catalog.validate(&plan, today)?;
                let plan = normalize_plan(plan)?;
                catalog.validate(&plan, today)?;
                let mut evidence = Vec::new();
                let mut total = 0;
                for query in &plan.queries {
                    let current = self.environment.discover(actor).await?;
                    if current.version != catalog.version {
                        return Err(Error::Invalid);
                    }
                    current.validate(
                        &Plan {
                            version: current.version.clone(),
                            queries: vec![query.clone()],
                        },
                        today,
                    )?;
                    let batch = self
                        .environment
                        .read(actor, &current, query, 10000 - total)
                        .await?;
                    total += batch.rows.len();
                    if total > 10000 {
                        return Err(Error::Incomplete);
                    }
                    evidence.push(analyze(&current, query, &batch, today)?);
                }
                // Recheck before presenting results; stored evidence never grants access.
                if self.environment.discover(actor).await?.version != catalog.version {
                    return Err(Error::Invalid);
                }
                let mut interpretations = context.interpretations;
                if let (Some(question), Some(first)) =
                    (context.question.as_ref(), plan.queries.first())
                {
                    let meaning =
                        serde_json::to_string(&first.analyses).map_err(|_| Error::Invalid)?;
                    interpretations.retain(|item| {
                        !(item.resource == first.resource && &item.term == question)
                    });
                    if interpretations.len() >= 16 {
                        interpretations.remove(0);
                    }
                    interpretations.push(Interpretation {
                        term: question.clone(),
                        meaning,
                        resource: first.resource.clone(),
                        version: catalog.version.clone(),
                    });
                }
                let mut referenced = context
                    .schemas
                    .into_iter()
                    .filter(|s| {
                        interpretations.iter().any(|i| i.resource == s.id)
                            && !plan.queries.iter().any(|q| q.resource == s.id)
                    })
                    .collect::<Vec<_>>();
                referenced.extend(
                    catalog
                        .resources
                        .iter()
                        .filter(|r| plan.queries.iter().any(|q| q.resource == r.id))
                        .cloned(),
                );
                self.state
                    .save_context(
                        actor,
                        &Context {
                            previous: Some(plan.clone()),
                            question: None,
                            draft: None,
                            interpretations,
                            subscriptions: Vec::new(),
                            schemas: referenced,
                        },
                    )
                    .await?;
                Ok(Reply {
                    status: "completed".into(),
                    text: render(catalog, &evidence),
                    evidence,
                })
            }
        }
    }
    pub async fn event(&self, actor: &Actor, event: &Event) -> Result<Reply, Error> {
        if !actor.read
            || !actor.subscribe
            || event.event_id.is_empty()
            || event.event_id.len() > 200
        {
            return Err(Error::Forbidden);
        }
        let catalog = self.environment.discover(actor).await?;
        let resource = catalog.resource(&event.resource)?;
        if !resource.events.contains(&event.kind) {
            return Err(Error::Forbidden);
        }
        self.environment.authorize_event(actor, event).await?;
        resource.validate_values(&event.values)?;
        let mut labels = Vec::new();
        for subscription in self.state.subscriptions(actor).await? {
            if subscription.status != "active"
                || subscription.resource != event.resource
                || subscription.event != event.kind
            {
                continue;
            }
            resource.validate_filters(&subscription.filters)?;
            if !matches_typed(resource, &subscription.filters, &event.values)? {
                continue;
            }
            if self
                .state
                .claim_event(actor, &subscription.id, &event.event_id)
                .await?
            {
                labels.push(subscription.id);
            }
        }
        let reply = if labels.is_empty() {
            Reply::message("ignored", "Sin entregas nuevas autorizadas.")
        } else {
            Reply::message(
                "notification",
                format!(
                    "{} {} en {}.\nFuente: {}.\nSuscripciones: {}.\nNo se incluyen datos personales del registro.",
                    if event.test {
                        "Prueba de evento (no es un registro nuevo):"
                    } else {
                        "Nuevo evento"
                    },
                    event.kind,
                    resource.label,
                    catalog.source,
                    labels.join(", ")
                ),
            )
        };
        self.state
            .record_audit(
                actor,
                &AuditEntry {
                    id: event.event_id.clone(),
                    kind: format!("event:{}", event.kind),
                    status: reply.status.clone(),
                    source: catalog.source.clone(),
                    resources: vec![event.resource.clone()],
                    version: catalog.version.clone(),
                },
            )
            .await?;
        Ok(reply)
    }
}

fn normalize_plan(mut plan: Plan) -> Result<Plan, Error> {
    let mut queries: Vec<Query> = Vec::new();
    for mut query in plan.queries.drain(..) {
        query
            .filters
            .sort_by_key(|f| serde_json::to_string(f).unwrap_or_default());
        if let Some(existing) = queries.iter_mut().find(|q| {
            q.resource == query.resource
                && serde_json::to_value(&q.filters).ok()
                    == serde_json::to_value(&query.filters).ok()
                && q.analyses.len() + query.analyses.len() <= 8
        }) {
            for analysis in query.analyses {
                let v = serde_json::to_value(&analysis).map_err(|_| Error::Invalid)?;
                if !existing
                    .analyses
                    .iter()
                    .any(|a| serde_json::to_value(a).ok().as_ref() == Some(&v))
                {
                    existing.analyses.push(analysis);
                }
            }
        } else {
            queries.push(query)
        }
    }
    plan.queries = queries;
    Ok(plan)
}
