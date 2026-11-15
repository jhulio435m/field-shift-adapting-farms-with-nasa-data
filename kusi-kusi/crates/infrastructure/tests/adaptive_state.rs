use kusi_application::adaptive::{Actor, AuditEntry, Receipt, State};
use kusi_domain::adaptive::{Context, Reply};
use kusi_infrastructure::adaptive::FileState;
use uuid::Uuid;

#[tokio::test]
async fn durable_claims_survive_reopen_and_remain_actor_scoped() {
    let root = std::env::temp_dir().join(format!("kusi-adaptive-state-{}", Uuid::new_v4()));
    let state = FileState { root: root.clone() };
    let actor = Actor {
        organization: Uuid::new_v4(),
        principal: "fixture".into(),
        read: true,
        subscribe: true,
    };
    assert!(state.claim(&actor, "request").await.expect("claim"));
    assert!(
        state
            .receipt(&actor, "request")
            .await
            .expect("unknown")
            .is_none()
    );
    let reopened = FileState { root: root.clone() };
    assert!(
        !reopened
            .claim(&actor, "request")
            .await
            .expect("durable claim")
    );
    let receipt = Receipt {
        version: "v1".into(),
        reply: Reply::message("completed", "fixture"),
    };
    reopened
        .finish(&actor, "request", &receipt)
        .await
        .expect("commit");
    assert_eq!(
        state
            .receipt(&actor, "request")
            .await
            .expect("read")
            .expect("receipt")
            .reply
            .status,
        "completed"
    );
    state
        .save_context(&actor, &Context::default())
        .await
        .expect("context");
    assert!(
        state
            .claim_event(&actor, "sub", "event")
            .await
            .expect("delivery")
    );
    assert!(
        !reopened
            .claim_event(&actor, "sub", "event")
            .await
            .expect("dedupe")
    );
    let mut other = actor.clone();
    other.organization = Uuid::new_v4();
    assert!(
        state
            .receipt(&other, "request")
            .await
            .expect("isolation")
            .is_none()
    );
    assert!(
        state
            .claim_event(&other, "sub", "event")
            .await
            .expect("separate")
    );
    let audit = AuditEntry {
        id: "request".into(),
        kind: "message".into(),
        status: "completed".into(),
        source: "fixture-source".into(),
        resources: vec!["fixture::orders".into()],
        version: "v1".into(),
    };
    state.record_audit(&actor, &audit).await.expect("audit");
    assert_eq!(
        reopened.audits(&actor).await.expect("actor audits"),
        vec![audit]
    );
    assert!(
        reopened
            .audits(&other)
            .await
            .expect("isolated audits")
            .is_empty()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for directory in std::fs::read_dir(&root).expect("directories") {
            let directory = directory.expect("entry").path();
            assert_eq!(
                std::fs::metadata(&directory)
                    .expect("mode")
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
            for file in std::fs::read_dir(directory).expect("files") {
                assert_eq!(
                    file.expect("file")
                        .metadata()
                        .expect("mode")
                        .permissions()
                        .mode()
                        & 0o077,
                    0
                );
            }
        }
    }
    std::fs::remove_dir_all(root).expect("cleanup synthetic state");
}
