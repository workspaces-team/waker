use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifest {
    pub family: String,
    pub frontend_contract: String,
    pub backbone_contract: String,
    pub detector_contract: String,
    #[serde(default)]
    pub verifier_contract: Option<String>,
    #[serde(default)]
    pub detector_path: Option<String>,
    #[serde(default)]
    pub verifier_path: Option<String>,
    #[serde(default)]
    pub decision_policy_path: Option<String>,
    #[serde(default)]
    pub benchmark_summary_path: Option<String>,
}

impl BundleManifest {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.family != "waker-desktop" {
            errors.push(format!("family must be waker-desktop, got {}", self.family));
        }
        if self.frontend_contract.trim().is_empty() {
            errors.push("frontendContract must be non-empty".to_string());
        }
        if self.backbone_contract.trim().is_empty() {
            errors.push("backboneContract must be non-empty".to_string());
        }
        if self.detector_contract.trim().is_empty() {
            errors.push("detectorContract must be non-empty".to_string());
        }

        errors
    }
}

pub fn load_bundle_manifest(bundle_url: &str) -> Result<BundleManifest> {
    let manifest_path = resolve_manifest_path(bundle_url);
    let contents = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let manifest: BundleManifest = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse manifest {}", manifest_path.display()))?;

    let errors = manifest.validate();
    if !errors.is_empty() {
        bail!(
            "bundle manifest validation failed:\n- {}",
            errors.join("\n- ")
        );
    }

    Ok(manifest)
}

fn resolve_manifest_path(bundle_url: &str) -> PathBuf {
    let path = Path::new(bundle_url);
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value == "manifest.json")
    {
        return path.to_path_buf();
    }
    path.join("manifest.json")
}

#[cfg(test)]
mod tests {
    use super::{load_bundle_manifest, BundleManifest};
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_manifest_dir() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        env::temp_dir().join(format!("waker-sdk-desktop-{suffix}"))
    }

    fn valid_manifest() -> BundleManifest {
        BundleManifest {
            family: "waker-desktop".to_string(),
            frontend_contract: "waker-mel-frontend-v1".to_string(),
            backbone_contract: "waker-web-backbone-v1".to_string(),
            detector_contract: "waker-detector-head-v1".to_string(),
            verifier_contract: None,
            detector_path: Some("detector.onnx".to_string()),
            verifier_path: None,
            decision_policy_path: Some("decision-policy.json".to_string()),
            benchmark_summary_path: Some("benchmark-summary.json".to_string()),
        }
    }

    #[test]
    fn validates_required_fields() {
        let mut manifest = valid_manifest();
        manifest.family = "waker-web".to_string();

        assert!(manifest
            .validate()
            .iter()
            .any(|error| error.contains("waker-desktop")));
    }

    #[test]
    fn loads_manifest_from_directory_path() {
        let dir = temp_manifest_dir();
        fs::create_dir_all(&dir).expect("create temp dir");
        let manifest_path = dir.join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");

        let loaded = load_bundle_manifest(dir.to_str().expect("utf8 path")).expect("load manifest");
        assert_eq!(loaded.family, "waker-desktop");

        fs::remove_dir_all(&dir).expect("cleanup temp dir");
    }
}
