//! Browser explorer/filesystem service backed by the shared JS bridge.

use platform_host::{
    ExplorerBackendStatus, ExplorerFileReadResult, ExplorerFsFuture, ExplorerFsService,
    ExplorerListResult, ExplorerMetadata, ExplorerPermissionMode, ExplorerPermissionState,
};

#[derive(Debug, Clone, Copy, Default)]
/// Browser explorer service backed by IndexedDB VFS + File System Access API bridge code.
pub struct WebExplorerFsService;

impl ExplorerFsService for WebExplorerFsService {
    fn status<'a>(&'a self) -> ExplorerFsFuture<'a, Result<ExplorerBackendStatus, String>> {
        Box::pin(async move { crate::bridge::explorer_status().await })
    }

    fn pick_native_directory<'a>(
        &'a self,
    ) -> ExplorerFsFuture<'a, Result<ExplorerBackendStatus, String>> {
        Box::pin(async move { crate::bridge::explorer_pick_native_directory().await })
    }

    fn request_permission<'a>(
        &'a self,
        mode: ExplorerPermissionMode,
    ) -> ExplorerFsFuture<'a, Result<ExplorerPermissionState, String>> {
        Box::pin(async move { crate::bridge::explorer_request_permission(mode).await })
    }

    fn list_dir<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerListResult, String>> {
        Box::pin(async move { crate::bridge::explorer_list_dir(path).await })
    }

    fn read_text_file<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerFileReadResult, String>> {
        Box::pin(async move { crate::bridge::explorer_read_text_file(path).await })
    }

    fn write_text_file<'a>(
        &'a self,
        path: &'a str,
        text: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        Box::pin(async move { crate::bridge::explorer_write_text_file(path, text).await })
    }

    fn create_dir<'a>(
        &'a self,
        path: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        Box::pin(async move { crate::bridge::explorer_create_dir(path).await })
    }

    fn create_file<'a>(
        &'a self,
        path: &'a str,
        text: &'a str,
    ) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        Box::pin(async move { crate::bridge::explorer_create_file(path, text).await })
    }

    fn delete<'a>(
        &'a self,
        path: &'a str,
        recursive: bool,
    ) -> ExplorerFsFuture<'a, Result<(), String>> {
        Box::pin(async move { crate::bridge::explorer_delete(path, recursive).await })
    }

    fn stat<'a>(&'a self, path: &'a str) -> ExplorerFsFuture<'a, Result<ExplorerMetadata, String>> {
        Box::pin(async move { crate::bridge::explorer_stat(path).await })
    }
}
