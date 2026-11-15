use async_trait::async_trait;
use chrono::NaiveDate;
use kusi_application::adaptive::*;
use kusi_domain::adaptive::*;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use uuid::Uuid;
fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 9).expect("test date")
}
fn actor() -> Actor {
    Actor {
        organization: Uuid::from_u128(1),
        principal: "owner".into(),
        read: true,
        subscribe: true,
    }
}
fn resource(id: &str, category: &str, date: &str) -> Resource {
    Resource {
        id: id.into(),
        label: id.into(),
        fields: vec![
            Field {
                id: category.into(),
                label: category.into(),
                kind: FieldKind::Category,
                options: vec![json!("A"), json!("B")],
            },
            Field {
                id: date.into(),
                label: date.into(),
                kind: FieldKind::Date,
                options: vec![],
            },
            Field {
                id: "amount".into(),
                label: "amount".into(),
                kind: FieldKind::Number,
                options: vec![],
            },
        ],
        events: vec!["created".into()],
    }
}
fn catalog() -> Catalog {
    Catalog {
        source: "environment-a".into(),
        version: "v1".into(),
        resources: vec![
            resource("registrations", "entry", "created"),
            resource("orders", "tier", "placed"),
            resource("new-resource", "kind", "opened"),
        ],
    }
}
fn rows(category: &str, date: &str) -> Batch {
    Batch {
        truncated: false,
        rows: [
            ("A", "2026-11-01", 10),
            ("B", "2026-11-02", 20),
            ("A", "2026-11-09", 30),
        ]
        .into_iter()
        .map(|(c, d, n)| {
            BTreeMap::from([
                (category.into(), json!(c)),
                (date.into(), json!(d)),
                ("amount".into(), json!(n)),
            ])
        })
        .collect(),
    }
}
#[derive(Clone)]
struct Source {
    catalog: Arc<Mutex<Catalog>>,
    enabled: Arc<AtomicBool>,
    reads: Arc<AtomicUsize>,
    discovers: Arc<AtomicUsize>,
    truncated: bool,
    watch_fails: Arc<AtomicBool>,
}
impl Source {
    fn new() -> Self {
        Self {
            catalog: Arc::new(Mutex::new(catalog())),
            enabled: Arc::new(AtomicBool::new(true)),
            reads: Arc::new(AtomicUsize::new(0)),
            discovers: Arc::new(AtomicUsize::new(0)),
            truncated: false,
            watch_fails: Arc::new(AtomicBool::new(false)),
        }
    }
}
#[async_trait]
impl Environment for Source {
    async fn discover(&self, a: &Actor) -> Result<Catalog, Error> {
        self.discovers.fetch_add(1, Ordering::SeqCst);
        if !self.enabled.load(Ordering::SeqCst) || a.principal != "owner" {
            return Err(Error::Forbidden);
        };
        Ok(self.catalog.lock().map_err(|_| Error::Unavailable)?.clone())
    }
    async fn read(&self, a: &Actor, c: &Catalog, q: &Query, _: usize) -> Result<Batch, Error> {
        self.discover(a).await?;
        self.reads.fetch_add(1, Ordering::SeqCst);
        let r = c.resource(&q.resource)?;
        let mut b = rows(&r.fields[0].id, &r.fields[1].id);
        b.truncated = self.truncated;
        Ok(b)
    }
    async fn authorize_event(&self, a: &Actor, event: &Event) -> Result<(), Error> {
        self.discover(a).await?;
        if event.event_id == "not-visible" {
            return Err(Error::Forbidden);
        }
        Ok(())
    }
    async fn watch(&self, a: &Actor, _: &Resource) -> Result<(), Error> {
        self.discover(a).await?;
        if self.watch_fails.load(Ordering::SeqCst) {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
}
struct Proposals(Mutex<Vec<Decision>>);
#[async_trait]
impl Planner for Proposals {
    async fn propose(
        &self,
        _: &Catalog,
        _: &Context,
        _: &str,
        _: NaiveDate,
    ) -> Result<Decision, Error> {
        let mut p = self.0.lock().map_err(|_| Error::Unavailable)?;
        if p.is_empty() {
            return Err(Error::Invalid);
        };
        Ok(p.remove(0))
    }
}
#[derive(Default)]
struct Memory {
    contexts: Mutex<BTreeMap<String, Context>>,
    receipts: Mutex<BTreeMap<String, Option<Receipt>>>,
    subs: Mutex<BTreeMap<String, Subscription>>,
    events: Mutex<Vec<String>>,
    audits: Mutex<BTreeMap<String, AuditEntry>>,
    fail: AtomicBool,
    fail_finish: AtomicBool,
}
fn key(a: &Actor, id: &str) -> String {
    format!("{}:{}:{id}", a.organization, a.principal)
}
#[async_trait]
impl State for Memory {
    async fn context(&self, a: &Actor) -> Result<Context, Error> {
        Ok(self
            .contexts
            .lock()
            .map_err(|_| Error::Unavailable)?
            .get(&key(a, "ctx"))
            .cloned()
            .unwrap_or_default())
    }
    async fn save_context(&self, a: &Actor, c: &Context) -> Result<(), Error> {
        self.contexts
            .lock()
            .map_err(|_| Error::Unavailable)?
            .insert(key(a, "ctx"), c.clone());
        Ok(())
    }
    async fn claim(&self, a: &Actor, id: &str) -> Result<bool, Error> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(Error::Unavailable);
        }
        let mut r = self.receipts.lock().map_err(|_| Error::Unavailable)?;
        if let std::collections::btree_map::Entry::Vacant(entry) = r.entry(key(a, id)) {
            entry.insert(None);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn receipt(&self, a: &Actor, id: &str) -> Result<Option<Receipt>, Error> {
        Ok(self
            .receipts
            .lock()
            .map_err(|_| Error::Unavailable)?
            .get(&key(a, id))
            .cloned()
            .flatten())
    }
    async fn finish(&self, a: &Actor, id: &str, r: &Receipt) -> Result<(), Error> {
        if self.fail_finish.load(Ordering::SeqCst) {
            return Err(Error::Unavailable);
        }
        self.receipts
            .lock()
            .map_err(|_| Error::Unavailable)?
            .insert(key(a, id), Some(r.clone()));
        Ok(())
    }
    async fn subscriptions(&self, a: &Actor) -> Result<Vec<Subscription>, Error> {
        Ok(self
            .subs
            .lock()
            .map_err(|_| Error::Unavailable)?
            .iter()
            .filter(|(k, _)| k.starts_with(&key(a, "")))
            .map(|(_, v)| v.clone())
            .collect())
    }
    async fn save_subscription(&self, a: &Actor, s: &Subscription) -> Result<(), Error> {
        self.subs
            .lock()
            .map_err(|_| Error::Unavailable)?
            .insert(key(a, &s.id), s.clone());
        Ok(())
    }
    async fn claim_event(&self, a: &Actor, s: &str, e: &str) -> Result<bool, Error> {
        let mut list = self.events.lock().map_err(|_| Error::Unavailable)?;
        let id = key(a, &format!("{s}:{e}"));
        if list.contains(&id) {
            Ok(false)
        } else {
            list.push(id);
            Ok(true)
        }
    }
    async fn record_audit(&self, a: &Actor, entry: &AuditEntry) -> Result<(), Error> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(Error::Unavailable);
        }
        self.audits.lock().map_err(|_| Error::Unavailable)?.insert(
            key(a, &format!("{}:{}", entry.kind, entry.id)),
            entry.clone(),
        );
        Ok(())
    }
    async fn audits(&self, a: &Actor) -> Result<Vec<AuditEntry>, Error> {
        Ok(self
            .audits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .iter()
            .filter(|(k, _)| k.starts_with(&key(a, "")))
            .map(|(_, v)| v.clone())
            .collect())
    }
}
fn query(id: &str, analyses: Vec<Analysis>) -> Decision {
    Decision::Query {
        plan: Plan {
            version: "v1".into(),
            queries: vec![Query {
                resource: id.into(),
                filters: vec![],
                analyses,
            }],
        },
    }
}
fn assistant(source: Source, decisions: Vec<Decision>) -> Assistant<Source, Proposals, Memory> {
    Assistant {
        environment: source,
        planner: Proposals(Mutex::new(decisions)),
        state: Memory::default(),
    }
}
#[tokio::test]
async fn same_engine_twenty_queries_three_unrelated_schemas_and_two_sources() {
    let source = Source::new();
    let mut decisions = Vec::new();
    for i in 0..20 {
        let id = ["registrations", "orders", "new-resource"][i % 3];
        let c = source.catalog.lock().expect("catalog");
        let r = c.resource(id).expect("resource");
        decisions.push(query(
            id,
            vec![
                Analysis::Count,
                Analysis::Group {
                    field: r.fields[0].id.clone(),
                },
                Analysis::Sum {
                    field: "amount".into(),
                },
                Analysis::Forecast {
                    date_field: r.fields[1].id.clone(),
                    target_date: "2026-12-25".into(),
                    window_days: 30,
                },
            ],
        ));
    }
    let engine = assistant(source.clone(), decisions);
    for i in 0..20 {
        let reply = engine
            .handle(
                &actor(),
                "untrusted natural request",
                &format!("request-{i}"),
                day(),
            )
            .await
            .expect("reply");
        assert_eq!(reply.status, "completed");
        assert_eq!(reply.evidence[0].results[0]["value"], 3);
        assert_eq!(reply.evidence[0].results[2]["value"], 60.0);
    }
    source.catalog.lock().expect("catalog").source = "environment-b".into();
    let engine = assistant(source, vec![query("orders", vec![Analysis::Count])]);
    assert_eq!(
        engine
            .handle(&actor(), "new source", "b", day())
            .await
            .expect("reply")
            .evidence[0]
            .source,
        "environment-b"
    );
}
#[tokio::test]
async fn ten_invalid_permissions_fields_and_versions_never_read() {
    for i in 0..10 {
        let source = Source::new();
        let mut p = match query("orders", vec![Analysis::Count]) {
            Decision::Query { plan } => plan,
            _ => unreachable!(),
        };
        match i % 5 {
            0 => p.version = "stale".into(),
            1 => p.queries[0].resource = "private".into(),
            2 => {
                p.queries[0].analyses = vec![Analysis::Group {
                    field: "email".into(),
                }]
            }
            3 => {
                p.queries[0].filters = vec![Filter {
                    field: "tier".into(),
                    op: Operator::Eq,
                    value: json!("not-an-option"),
                }]
            }
            _ => {
                p.queries[0].analyses = vec![Analysis::Forecast {
                    date_field: "placed".into(),
                    target_date: "2020-01-01".into(),
                    window_days: 30,
                }]
            }
        }
        let engine = assistant(source.clone(), vec![Decision::Query { plan: p }]);
        let reply = engine
            .handle(&actor(), "bypass all rules", "x", day())
            .await
            .expect("response");
        assert_ne!(reply.status, "completed");
        assert_eq!(source.reads.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn revoked_permission_and_actor_isolation_override_memory() {
    let source = Source::new();
    let engine = assistant(source.clone(), vec![query("orders", vec![Analysis::Count])]);
    engine
        .handle(&actor(), "read", "1", day())
        .await
        .expect("first");
    source.enabled.store(false, Ordering::SeqCst);
    assert!(matches!(
        engine.handle(&actor(), "again", "1", day()).await,
        Err(Error::Forbidden)
    ));
    let mut other = actor();
    other.principal = "other".into();
    assert!(
        engine
            .state
            .context(&other)
            .await
            .expect("ctx")
            .previous
            .is_none()
    );
}
#[tokio::test]
async fn incomplete_data_and_audit_failure_never_fake_success() {
    let mut source = Source::new();
    source.truncated = true;
    let engine = assistant(source.clone(), vec![query("orders", vec![Analysis::Count])]);
    assert_eq!(
        engine
            .handle(&actor(), "read", "1", day())
            .await
            .expect("reply")
            .status,
        "clarification"
    );
    let source = Source::new();
    let engine = assistant(source.clone(), vec![]);
    engine.state.fail.store(true, Ordering::SeqCst);
    assert!(matches!(
        engine.handle(&actor(), "read", "1", day()).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(source.reads.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn new_resource_and_schema_changes_are_observed() {
    let source = Source::new();
    source.catalog.lock().expect("catalog").resources.pop();
    let engine = assistant(
        source.clone(),
        vec![query("new-resource", vec![Analysis::Count])],
    );
    assert_ne!(
        engine
            .handle(&actor(), "read new", "1", day())
            .await
            .expect("reply")
            .status,
        "completed"
    );
    source
        .catalog
        .lock()
        .expect("catalog")
        .resources
        .push(resource("new-resource", "kind", "opened"));
    let engine = assistant(
        source.clone(),
        vec![query("new-resource", vec![Analysis::Count])],
    );
    assert_eq!(
        engine
            .handle(&actor(), "read new", "2", day())
            .await
            .expect("reply")
            .status,
        "completed"
    );
    source.catalog.lock().expect("catalog").resources[2]
        .fields
        .clear();
    let engine = assistant(
        source,
        vec![query(
            "new-resource",
            vec![Analysis::Group {
                field: "kind".into(),
            }],
        )],
    );
    assert_ne!(
        engine
            .handle(&actor(), "stale field", "3", day())
            .await
            .expect("reply")
            .status,
        "completed"
    );
}
#[tokio::test]
async fn two_generic_subscriptions_dedupe_and_cancel() {
    let source = Source::new();
    let engine = assistant(
        source,
        vec![
            Decision::Subscribe {
                resource: "orders".into(),
                event: "created".into(),
                filters: vec![],
            },
            Decision::Subscribe {
                resource: "registrations".into(),
                event: "created".into(),
                filters: vec![],
            },
        ],
    );
    let a = actor();
    engine
        .handle(&a, "subscribe", "s1", day())
        .await
        .expect("s1");
    engine
        .handle(&a, "subscribe", "s2", day())
        .await
        .expect("s2");
    let subs = engine.state.subscriptions(&a).await.expect("subs");
    assert_eq!(subs.len(), 2);
    for s in subs {
        let e = Event {
            test: false,
            resource: s.resource.clone(),
            event_id: "event-1".into(),
            kind: "created".into(),
            values: BTreeMap::new(),
        };
        let mut denied = e.clone();
        denied.event_id = "not-visible".into();
        assert!(matches!(
            engine.event(&a, &denied).await,
            Err(Error::Forbidden)
        ));
        let mut malformed = e.clone();
        malformed
            .values
            .insert("amount".into(), json!("not a number"));
        assert!(matches!(
            engine.event(&a, &malformed).await,
            Err(Error::Invalid)
        ));
        assert_eq!(
            engine.event(&a, &e).await.expect("event").status,
            "notification"
        );
        assert_eq!(
            engine.event(&a, &e).await.expect("duplicate").status,
            "ignored"
        );
        let mut cancelled = s;
        cancelled.status = "cancelled".into();
        engine
            .state
            .save_subscription(&a, &cancelled)
            .await
            .expect("cancel");
        let mut e = e;
        e.event_id = "event-2".into();
        assert_eq!(
            engine.event(&a, &e).await.expect("cancelled").status,
            "ignored"
        );
    }
}
#[test]
fn temporal_comparison_and_date_filters_include_full_day() {
    let c = catalog();
    let q = Query {
        resource: "orders".into(),
        filters: vec![],
        analyses: vec![Analysis::Compare {
            date_field: "placed".into(),
            start: "2026-11-02".into(),
            end: "2026-11-02".into(),
            previous_start: "2026-11-01".into(),
            previous_end: "2026-11-01".into(),
        }],
    };
    let e = analyze(&c, &q, &rows("tier", "placed"), day()).expect("compare");
    assert_eq!(e.results[0]["difference"], 0);
    let r = c.resource("orders").expect("resource");
    let row = BTreeMap::from([("placed".into(), json!("2026-11-02 23:59:59.123456"))]);
    assert!(
        matches_typed(
            r,
            &[Filter {
                field: "placed".into(),
                op: Operator::Le,
                value: json!("2026-11-02")
            }],
            &row
        )
        .expect("full end day")
    );
}
#[test]
fn planned_json_rejects_arbitrary_actions() {
    assert!(serde_json::from_value::<Decision>(json!({"type":"sql","query":"select 1"})).is_err());
    assert!(
        serde_json::from_value::<Decision>(json!({"type":"reset","organization_id":"other"}))
            .is_err()
    );
}

#[tokio::test]
async fn interrupted_claim_and_failed_commit_do_not_repeat_work() {
    let a = actor();
    let source = Source::new();
    let engine = assistant(source.clone(), vec![query("orders", vec![Analysis::Count])]);
    assert!(engine.state.claim(&a, "interrupted").await.expect("claim"));
    assert_eq!(
        engine
            .handle(&a, "count", "interrupted", day())
            .await
            .expect("unknown")
            .status,
        "unknown"
    );
    assert_eq!(source.reads.load(Ordering::SeqCst), 0);
    engine.state.fail_finish.store(true, Ordering::SeqCst);
    assert!(matches!(
        engine.handle(&a, "count", "commit", day()).await,
        Err(Error::Unavailable)
    ));
    let count = source.reads.load(Ordering::SeqCst);
    assert_eq!(count, 1);
    assert_eq!(
        engine
            .handle(&a, "count", "commit", day())
            .await
            .expect("unknown")
            .status,
        "unknown"
    );
    assert_eq!(source.reads.load(Ordering::SeqCst), count);
    let engine = assistant(Source::new(), vec![]);
    assert_eq!(
        engine
            .handle(&a, "count", "bad-planner", day())
            .await
            .expect("clarify")
            .status,
        "clarification"
    );
    assert!(
        engine
            .state
            .receipt(&a, "bad-planner")
            .await
            .expect("receipt")
            .is_some()
    );
}

#[tokio::test]
async fn pending_watch_is_recovered_without_duplicate_subscription() {
    let source = Source::new();
    source.watch_fails.store(true, Ordering::SeqCst);
    let decision = Decision::Subscribe {
        resource: "orders".into(),
        event: "created".into(),
        filters: vec![],
    };
    let engine = assistant(source.clone(), vec![decision.clone(), decision]);
    let a = actor();
    assert_eq!(
        engine
            .handle(&a, "watch", "first", day())
            .await
            .expect("failed watch")
            .status,
        "unavailable"
    );
    let pending = engine.state.subscriptions(&a).await.expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].status, "pending");
    source.watch_fails.store(false, Ordering::SeqCst);
    assert_eq!(
        engine
            .handle(&a, "watch", "retry", day())
            .await
            .expect("recover")
            .status,
        "completed"
    );
    let active = engine.state.subscriptions(&a).await.expect("active");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, pending[0].id);
    assert_eq!(active[0].status, "active");
}

#[test]
fn forecast_reply_preserves_evidence_and_formats_dates_and_rates() {
    let catalog = catalog();
    let query = Query {
        resource: "orders".into(),
        filters: vec![],
        analyses: vec![Analysis::Forecast {
            date_field: "placed".into(),
            target_date: "2026-12-25".into(),
            window_days: 30,
        }],
    };
    let evidence = analyze(&catalog, &query, &rows("tier", "placed"), day()).expect("forecast");
    let reply = render(&catalog, &[evidence]);
    assert!(reply.contains("Proyección al 2026-12-25: aproximadamente 15."));
    assert!(reply.contains("ritmo 0.25 por día"));
    assert!(reply.contains("no intervalos de confianza"));
    assert!(reply.contains("Datos completos para esta lectura: 3 filas"));
}

#[tokio::test]
async fn semantic_interpretations_persist_across_turns_and_invalidate_on_schema_change() {
    let source = Source::new();
    let engine = assistant(
        source.clone(),
        vec![
            Decision::Clarify {
                question: "¿Total significa cantidad de registros o suma de importes?".into(),
                draft: None,
            },
            query("orders", vec![Analysis::Count]),
            query(
                "orders",
                vec![Analysis::Sum {
                    field: "amount".into(),
                }],
            ),
            Decision::Clarify {
                question: "¿Total significa cantidad de registros o suma de importes?".into(),
                draft: None,
            },
            query(
                "orders",
                vec![Analysis::Sum {
                    field: "amount".into(),
                }],
            ),
            Decision::Unsupported {
                capability: "escritura de registros".into(),
                explanation: "Puedo consultar y proyectar pedidos, pero modificar o eliminar registros requiere aprobación y una capacidad de escritura habilitada.".into(),
                requires_approval: true,
            },
        ],
    );
    let a = actor();
    assert_eq!(
        engine
            .handle(&a, "dame el total", "t1", day())
            .await
            .expect("clarify")
            .status,
        "clarification"
    );
    assert_eq!(
        engine
            .handle(&a, "cantidad de registros", "t2", day())
            .await
            .expect("resolved")
            .status,
        "completed"
    );
    let ctx = engine.state.context(&a).await.expect("ctx");
    assert_eq!(ctx.interpretations.len(), 1);
    assert_eq!(ctx.interpretations[0].resource, "orders");
    assert!(ctx.interpretations[0].meaning.contains("count"));
    assert_eq!(
        engine
            .handle(&a, "ahora suma sus importes", "t3", day())
            .await
            .expect("next query")
            .status,
        "completed"
    );
    assert_eq!(
        engine
            .state
            .context(&a)
            .await
            .expect("ctx")
            .interpretations
            .len(),
        1
    );
    assert_eq!(
        engine
            .handle(&a, "corrige total", "t4", day())
            .await
            .expect("reclarify")
            .status,
        "clarification"
    );
    assert_eq!(
        engine
            .handle(&a, "usa suma de importes", "t5", day())
            .await
            .expect("corrected")
            .status,
        "completed"
    );
    let ctx = engine.state.context(&a).await.expect("ctx");
    assert_eq!(ctx.interpretations.len(), 1);
    assert!(ctx.interpretations[0].meaning.contains("sum"));
    let reads_before = source.reads.load(Ordering::SeqCst);
    let blocked = engine
        .handle(&a, "borra todos los pedidos", "t6", day())
        .await
        .expect("unsupported");
    assert_eq!(blocked.status, "blocked");
    assert!(
        blocked
            .text
            .contains("capacidad requerida: escritura de registros")
    );
    assert_eq!(source.reads.load(Ordering::SeqCst), reads_before);
    let audits = engine.state.audits(&a).await.expect("audits");
    assert_eq!(audits.len(), 6);
    assert!(
        audits
            .iter()
            .any(|entry| entry.id == "t6" && entry.status == "blocked")
    );
    assert!(audits.iter().all(|entry| !entry.id.contains("borra")));
    source.catalog.lock().expect("catalog").resources[1].fields[0].id = "changed_tier".into();
    let _ = engine.handle(&a, "otra consulta", "t7", day()).await;
    assert!(
        engine
            .state
            .context(&a)
            .await
            .expect("invalidated ctx")
            .interpretations
            .is_empty()
    );
}

#[derive(Clone)]
struct AccessPolicy {
    allowed: Arc<AtomicBool>,
    protected: bool,
    global: bool,
    audit_available: bool,
}
impl AccessPolicy {
    fn active() -> Self {
        Self {
            allowed: Arc::new(AtomicBool::new(true)),
            protected: true,
            global: true,
            audit_available: true,
        }
    }
}
#[async_trait]
impl kusi_application::CapabilityRegistry for AccessPolicy {
    async fn register(
        &self,
        _: Uuid,
        _: &kusi_domain::Capability,
    ) -> Result<(), kusi_application::ApplicationError> {
        Ok(())
    }
    async fn list(
        &self,
        _: Uuid,
    ) -> Result<Vec<kusi_domain::Capability>, kusi_application::ApplicationError> {
        Ok(vec![kusi_domain::Capability {
            operation: kusi_domain::Operation::Read,
            identifier: "analytics.read".into(),
            version: "1".into(),
            description: "Synthetic analytics".into(),
            adapter: "system-a".into(),
            required_permission: "analytics.read".into(),
            enabled: true,
            timeout_ms: 1000,
            input_schema: json!({}),
            output_schema: json!({}),
        }])
    }
}
#[async_trait]
impl kusi_application::integrations::AuthorizationService for AccessPolicy {
    async fn permits(&self, org: Uuid, principal: &str, _: &str) -> bool {
        self.global && org == actor().organization && principal == actor().principal
    }
    async fn authorize_resource(
        &self,
        _: Uuid,
        _: &str,
        _: &kusi_domain::Capability,
        input: &serde_json::Value,
    ) -> Result<
        Option<kusi_domain::external_access::ResourceRead>,
        kusi_application::integrations::ExecutionError,
    > {
        if !self.allowed.load(Ordering::SeqCst) || input["resource_ref"] != "dataset-a" {
            return Err(kusi_application::integrations::ExecutionError::Forbidden);
        }
        Ok(self
            .protected
            .then(|| kusi_domain::external_access::ResourceRead {
                resource_ref: "dataset-a".into(),
            }))
    }
}
#[async_trait]
impl kusi_application::integrations::AuditRepository for AccessPolicy {
    async fn record(
        &self,
        _: Uuid,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), kusi_application::integrations::ExecutionError> {
        if self.audit_available {
            Ok(())
        } else {
            Err(kusi_application::integrations::ExecutionError::Unavailable)
        }
    }
}
fn protected_source(
    source: Source,
    policy: AccessPolicy,
) -> AuthorizedEnvironment<Source, AccessPolicy> {
    AuthorizedEnvironment::new(
        source,
        policy,
        actor(),
        BTreeMap::from([
            (
                "registrations".into(),
                ResourceBinding {
                    adapter: "system-a".into(),
                    capability: "analytics.read".into(),
                    version: "1".into(),
                    resource_ref: "dataset-a".into(),
                },
            ),
            (
                "orders".into(),
                ResourceBinding {
                    adapter: "system-a".into(),
                    capability: "analytics.read".into(),
                    version: "1".into(),
                    resource_ref: "dataset-b".into(),
                },
            ),
        ]),
    )
    .expect("protected fixture")
}
#[tokio::test]
async fn external_acl_filters_catalog_and_rechecks_reads_watches_and_events() {
    let source = Source::new();
    let policy = AccessPolicy::active();
    let env = protected_source(source.clone(), policy.clone());
    let actor = actor();
    let catalog = env.discover(&actor).await.expect("authorized discovery");
    assert_eq!(
        catalog
            .resources
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["registrations"]
    );
    assert_ne!(catalog.version, "v1");
    let query = Query {
        resource: "registrations".into(),
        filters: vec![],
        analyses: vec![Analysis::Count],
    };
    assert!(env.read(&actor, &catalog, &query, 100).await.is_ok());
    assert_eq!(source.reads.load(Ordering::SeqCst), 1);
    policy.allowed.store(false, Ordering::SeqCst);
    assert!(matches!(
        env.read(&actor, &catalog, &query, 100).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        env.watch(&actor, &catalog.resources[0]).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        env.authorize_event(
            &actor,
            &Event {
                test: true,
                resource: "registrations".into(),
                event_id: "revoked".into(),
                kind: "created".into(),
                values: BTreeMap::new()
            }
        )
        .await,
        Err(Error::Forbidden)
    ));
    assert_eq!(source.reads.load(Ordering::SeqCst), 1);
    let discovers = source.discovers.load(Ordering::SeqCst);
    assert!(matches!(env.discover(&actor).await, Err(Error::Forbidden)));
    assert_eq!(source.discovers.load(Ordering::SeqCst), discovers);
}
#[tokio::test]
async fn missing_protected_policy_global_permission_or_audit_never_discovers_source() {
    for policy in [
        AccessPolicy {
            protected: false,
            ..AccessPolicy::active()
        },
        AccessPolicy {
            global: false,
            ..AccessPolicy::active()
        },
        AccessPolicy {
            audit_available: false,
            ..AccessPolicy::active()
        },
    ] {
        let source = Source::new();
        let env = protected_source(source.clone(), policy);
        assert!(env.discover(&actor()).await.is_err());
        assert_eq!(source.discovers.load(Ordering::SeqCst), 0);
    }
    let source = Source::new();
    let env = protected_source(source.clone(), AccessPolicy::active());
    let mut other = actor();
    other.organization = Uuid::new_v4();
    assert!(matches!(env.discover(&other).await, Err(Error::Forbidden)));
    other = actor();
    other.principal = "forged".into();
    assert!(matches!(env.discover(&other).await, Err(Error::Forbidden)));
    assert_eq!(source.discovers.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn revoked_acl_prevents_saved_receipt_replay_and_notification() {
    let source = Source::new();
    let policy = AccessPolicy::active();
    let env = protected_source(source.clone(), policy.clone());
    let catalog = env.discover(&actor()).await.expect("catalog");
    let engine = Assistant {
        environment: env,
        planner: Proposals(Mutex::new(vec![Decision::Query {
            plan: Plan {
                version: catalog.version.clone(),
                queries: vec![Query {
                    resource: "registrations".into(),
                    filters: vec![],
                    analyses: vec![Analysis::Count],
                }],
            },
        }])),
        state: Memory::default(),
    };
    assert_eq!(
        engine
            .handle(&actor(), "synthetic count", "same-request", day())
            .await
            .expect("completed")
            .status,
        "completed"
    );
    engine
        .state
        .save_subscription(
            &actor(),
            &Subscription {
                id: "subscription".into(),
                resource: "registrations".into(),
                event: "created".into(),
                filters: vec![],
                status: "active".into(),
                version: catalog.version,
            },
        )
        .await
        .expect("subscription");
    let event = Event {
        test: true,
        resource: "registrations".into(),
        event_id: "event-a".into(),
        kind: "created".into(),
        values: BTreeMap::from([
            ("entry".into(), json!("A")),
            ("created".into(), json!("2026-11-09")),
            ("amount".into(), json!(10)),
        ]),
    };
    assert_eq!(
        engine
            .event(&actor(), &event)
            .await
            .expect("notification")
            .status,
        "notification"
    );
    policy.allowed.store(false, Ordering::SeqCst);
    assert!(matches!(
        engine
            .handle(&actor(), "synthetic count", "same-request", day())
            .await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        engine.event(&actor(), &event).await,
        Err(Error::Forbidden)
    ));
    assert_eq!(source.reads.load(Ordering::SeqCst), 1);
}

struct PrivateContextPlanner;
#[async_trait]
impl Planner for PrivateContextPlanner {
    async fn propose(
        &self,
        catalog: &Catalog,
        context: &Context,
        _: &str,
        _: NaiveDate,
    ) -> Result<Decision, Error> {
        assert_eq!(catalog.resources.len(), 1);
        assert!(context.question.is_none());
        assert!(context.draft.is_none());
        assert!(context.schemas.is_empty());
        assert!(context.subscriptions.is_empty());
        Ok(Decision::Clarify {
            question: "Synthetic clarification".into(),
            draft: None,
        })
    }
}
#[tokio::test]
async fn revoked_resource_is_removed_from_drafts_schemas_questions_and_subscription_hints() {
    let state = Memory::default();
    state
        .save_context(
            &actor(),
            &Context {
                question: Some("old private resource label".into()),
                schemas: vec![resource("orders", "tier", "placed")],
                draft: Some(Query {
                    resource: "orders".into(),
                    filters: vec![],
                    analyses: vec![Analysis::Count],
                }),
                ..Context::default()
            },
        )
        .await
        .expect("context");
    state
        .save_subscription(
            &actor(),
            &Subscription {
                id: "old".into(),
                resource: "orders".into(),
                event: "created".into(),
                filters: vec![],
                status: "active".into(),
                version: "old".into(),
            },
        )
        .await
        .expect("subscription");
    let engine = Assistant {
        environment: protected_source(Source::new(), AccessPolicy::active()),
        planner: PrivateContextPlanner,
        state,
    };
    assert_eq!(
        engine
            .handle(&actor(), "synthetic query", "context-filter", day())
            .await
            .expect("safe context")
            .status,
        "clarification"
    );
}
