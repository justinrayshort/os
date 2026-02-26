//! Cache API-backed content cache implementation.

use platform_host::{ContentCache, ContentCacheFuture};

#[derive(Debug, Clone, Copy, Default)]
/// Browser content cache backed by the Cache API.
pub struct WebContentCache;

impl ContentCache for WebContentCache {
    fn put_text<'a>(
        &'a self,
        cache_name: &'a str,
        key: &'a str,
        value: &'a str,
    ) -> ContentCacheFuture<'a, Result<(), String>> {
        Box::pin(async move { crate::bridge::cache_put_text(cache_name, key, value).await })
    }

    fn get_text<'a>(
        &'a self,
        cache_name: &'a str,
        key: &'a str,
    ) -> ContentCacheFuture<'a, Result<Option<String>, String>> {
        Box::pin(async move { crate::bridge::cache_get_text(cache_name, key).await })
    }

    fn delete<'a>(
        &'a self,
        cache_name: &'a str,
        key: &'a str,
    ) -> ContentCacheFuture<'a, Result<(), String>> {
        Box::pin(async move { crate::bridge::cache_delete(cache_name, key).await })
    }
}
