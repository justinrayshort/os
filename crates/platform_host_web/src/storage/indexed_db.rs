//! IndexedDB-backed app-state store implementation.

use platform_host::{AppStateEnvelope, AppStateStore, AppStateStoreFuture};

#[derive(Debug, Clone, Copy, Default)]
/// Browser app-state store backed by IndexedDB.
pub struct WebAppStateStore;

impl AppStateStore for WebAppStateStore {
    fn load_app_state_envelope<'a>(
        &'a self,
        namespace: &'a str,
    ) -> AppStateStoreFuture<'a, Result<Option<AppStateEnvelope>, String>> {
        Box::pin(async move { crate::bridge::load_app_state_envelope(namespace).await })
    }

    fn save_app_state_envelope<'a>(
        &'a self,
        envelope: &'a AppStateEnvelope,
    ) -> AppStateStoreFuture<'a, Result<(), String>> {
        Box::pin(async move { crate::bridge::save_app_state_envelope(envelope).await })
    }

    fn delete_app_state<'a>(
        &'a self,
        namespace: &'a str,
    ) -> AppStateStoreFuture<'a, Result<(), String>> {
        Box::pin(async move { crate::bridge::delete_app_state(namespace).await })
    }

    fn list_app_state_namespaces<'a>(
        &'a self,
    ) -> AppStateStoreFuture<'a, Result<Vec<String>, String>> {
        Box::pin(async move { crate::bridge::list_app_state_namespaces().await })
    }
}

