//! Wallpaper asset service contracts and shared update models.

use std::{future::Future, pin::Pin};

use desktop_app_contract::{
    ResolvedWallpaperSource, WallpaperAssetRecord, WallpaperCollection, WallpaperImportRequest,
    WallpaperLibrarySnapshot, WallpaperSelection,
};
use serde::{Deserialize, Serialize};

/// Object-safe boxed future used by [`WallpaperAssetService`] async methods.
pub type WallpaperAssetFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Patch payload used to update managed wallpaper asset metadata.
pub struct WallpaperAssetMetadataPatch {
    /// Updated display name when present.
    pub display_name: Option<String>,
    /// Updated favorite flag when present.
    pub favorite: Option<bool>,
    /// Replacement tag list when present.
    pub tags: Option<Vec<String>>,
    /// Replacement collection memberships when present.
    pub collection_ids: Option<Vec<String>>,
}

/// Host service for managed wallpaper asset import, metadata updates, and source resolution.
pub trait WallpaperAssetService {
    /// Imports a wallpaper asset through the host picker flow.
    fn import_from_picker<'a>(
        &'a self,
        request: WallpaperImportRequest,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperAssetRecord, String>>;

    /// Lists the current wallpaper library snapshot.
    fn list_library<'a>(
        &'a self,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>>;

    /// Updates asset metadata by patch.
    fn update_asset_metadata<'a>(
        &'a self,
        asset_id: &'a str,
        patch: WallpaperAssetMetadataPatch,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperAssetRecord, String>>;

    /// Creates a wallpaper collection.
    fn create_collection<'a>(
        &'a self,
        display_name: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperCollection, String>>;

    /// Renames a wallpaper collection.
    fn rename_collection<'a>(
        &'a self,
        collection_id: &'a str,
        display_name: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperCollection, String>>;

    /// Deletes a wallpaper collection and removes its memberships.
    fn delete_collection<'a>(
        &'a self,
        collection_id: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>>;

    /// Deletes a wallpaper asset.
    fn delete_asset<'a>(
        &'a self,
        asset_id: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>>;

    /// Resolves a wallpaper selection to a renderer-safe source.
    fn resolve_source<'a>(
        &'a self,
        selection: WallpaperSelection,
    ) -> WallpaperAssetFuture<'a, Result<Option<ResolvedWallpaperSource>, String>>;
}

#[derive(Debug, Clone, Copy, Default)]
/// No-op wallpaper host adapter used by unsupported targets and baseline tests.
pub struct NoopWallpaperAssetService;

impl NoopWallpaperAssetService {
    fn unsupported(op: &str) -> String {
        format!("wallpaper asset service unavailable: {op}")
    }
}

impl WallpaperAssetService for NoopWallpaperAssetService {
    fn import_from_picker<'a>(
        &'a self,
        _request: WallpaperImportRequest,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperAssetRecord, String>> {
        Box::pin(async { Err(Self::unsupported("import_from_picker")) })
    }

    fn list_library<'a>(
        &'a self,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>> {
        Box::pin(async { Ok(WallpaperLibrarySnapshot::default()) })
    }

    fn update_asset_metadata<'a>(
        &'a self,
        _asset_id: &'a str,
        _patch: WallpaperAssetMetadataPatch,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperAssetRecord, String>> {
        Box::pin(async { Err(Self::unsupported("update_asset_metadata")) })
    }

    fn create_collection<'a>(
        &'a self,
        _display_name: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperCollection, String>> {
        Box::pin(async { Err(Self::unsupported("create_collection")) })
    }

    fn rename_collection<'a>(
        &'a self,
        _collection_id: &'a str,
        _display_name: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperCollection, String>> {
        Box::pin(async { Err(Self::unsupported("rename_collection")) })
    }

    fn delete_collection<'a>(
        &'a self,
        _collection_id: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>> {
        Box::pin(async { Err(Self::unsupported("delete_collection")) })
    }

    fn delete_asset<'a>(
        &'a self,
        _asset_id: &'a str,
    ) -> WallpaperAssetFuture<'a, Result<WallpaperLibrarySnapshot, String>> {
        Box::pin(async { Err(Self::unsupported("delete_asset")) })
    }

    fn resolve_source<'a>(
        &'a self,
        _selection: WallpaperSelection,
    ) -> WallpaperAssetFuture<'a, Result<Option<ResolvedWallpaperSource>, String>> {
        Box::pin(async { Ok(None) })
    }
}
