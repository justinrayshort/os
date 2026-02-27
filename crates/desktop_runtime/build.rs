use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowDefaults {
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppManifest {
    schema_version: u32,
    app_id: String,
    display_name: String,
    version: String,
    runtime_contract_version: String,
    requested_capabilities: Vec<String>,
    single_instance: bool,
    suspend_policy: String,
    show_in_launcher: bool,
    show_on_desktop: bool,
    window_defaults: WindowDefaults,
}

fn app_manifest_paths(root: &Path) -> Vec<PathBuf> {
    ["calculator", "explorer", "notepad", "terminal", "settings"]
        .iter()
        .map(|name| {
            root.join("..")
                .join("apps")
                .join(name)
                .join("app.manifest.toml")
        })
        .collect()
}

fn main() {
    let crate_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let mut manifests = Vec::<AppManifest>::new();

    for path in app_manifest_paths(&crate_root) {
        println!("cargo:rerun-if-changed={}", path.display());
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let manifest: AppManifest = toml::from_str(&raw)
            .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()));
        if manifest.schema_version != 1 {
            panic!(
                "manifest schema mismatch in {}: expected 1 found {}",
                path.display(),
                manifest.schema_version
            );
        }
        if !manifest.runtime_contract_version.starts_with("2.") {
            panic!(
                "runtime contract mismatch in {}: expected 2.x found {}",
                path.display(),
                manifest.runtime_contract_version
            );
        }
        manifests.push(manifest);
    }

    manifests.sort_by(|a, b| a.app_id.cmp(&b.app_id));
    let json = serde_json::to_string_pretty(&manifests).expect("serialize app manifest catalog");
    let generated = format!(
        "/// Build-time generated app manifest catalog JSON.\n\
pub const APP_MANIFEST_CATALOG_JSON: &str = r##\"{}\"##;\n",
        json
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let out_file = out_dir.join("app_catalog_generated.rs");
    fs::write(&out_file, generated)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", out_file.display()));
}
