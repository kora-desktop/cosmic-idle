// Tells logind when the session goes idle, so its `IdleHint` is true while no one
// has used the computer for a while. Services read it to know no one is watching.

use futures_lite::{Stream, StreamExt};
use logind_zbus::{manager::ManagerProxy, session::SessionProxy};

// Sets this session's `IdleHint` to each value `idle` yields that differs from the last.
pub async fn follow(system: &zbus::Connection, idle: impl Stream<Item = bool>) -> zbus::Result<()> {
    let session_path = ManagerProxy::new(system).await?.get_session("auto").await?;
    let session = SessionProxy::builder(system)
        .path(session_path)?
        .build()
        .await?;
    let mut idle = std::pin::pin!(idle);
    let mut last = None;
    while let Some(idle) = idle.next().await {
        if last != Some(idle) {
            session.set_idle_hint(idle).await?;
            last = Some(idle);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_peers::peers;
    use futures_lite::future;
    use std::sync::{Arc, Mutex};
    use zbus::zvariant::OwnedObjectPath;

    const SESSION_PATH: &str = "/org/freedesktop/login1/session/_32";

    struct FakeManager;

    #[zbus::interface(name = "org.freedesktop.login1.Manager")]
    impl FakeManager {
        fn get_session(&self, session_id: &str) -> OwnedObjectPath {
            assert_eq!(session_id, "auto");
            OwnedObjectPath::try_from(SESSION_PATH).unwrap()
        }
    }

    struct FakeSession(Arc<Mutex<Vec<bool>>>);

    #[zbus::interface(name = "org.freedesktop.login1.Session")]
    impl FakeSession {
        fn set_idle_hint(&self, idle: bool) {
            self.0.lock().unwrap().push(idle);
        }
    }

    #[test]
    fn idle_hint_is_set_once_per_change() {
        future::block_on(async {
            let hints = Arc::new(Mutex::new(Vec::new()));
            let session = FakeSession(Arc::clone(&hints));
            let (_logind, system) = peers(|builder| {
                builder
                    .serve_at("/org/freedesktop/login1", FakeManager)?
                    .serve_at(SESSION_PATH, session)
            })
            .await;

            let idle = futures_lite::stream::iter([false, true, true, false]);
            follow(&system, idle).await.unwrap();
            assert_eq!(*hints.lock().unwrap(), [false, true, false]);
        });
    }
}
