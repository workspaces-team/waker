mod manifest;

use anyhow::Error as AnyError;
use napi::bindgen_prelude::{Error, Float32Array};
use napi_derive::napi;

use crate::manifest::{load_bundle_manifest, BundleManifest};

#[napi(object)]
pub struct WakerDesktopDetectionResult {
    pub detected: bool,
    pub score: f64,
}

#[napi]
pub struct WakerDesktopNativeDetector {
    bundle_url: Option<String>,
    manifest: Option<BundleManifest>,
    manifest_json: Option<String>,
}

#[napi]
impl WakerDesktopNativeDetector {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            bundle_url: None,
            manifest: None,
            manifest_json: None,
        }
    }

    #[napi]
    pub fn load(&mut self, bundle_url: String) -> napi::Result<String> {
        let manifest = load_bundle_manifest(&bundle_url).map_err(to_napi_error)?;
        let manifest_json =
            serde_json::to_string(&manifest).map_err(|error| to_napi_error(error.into()))?;

        self.bundle_url = Some(bundle_url);
        self.manifest = Some(manifest);
        self.manifest_json = Some(manifest_json.clone());

        Ok(manifest_json)
    }

    #[napi(js_name = "processChunk")]
    pub fn process_chunk(&self, pcm: Float32Array) -> napi::Result<WakerDesktopDetectionResult> {
        if self.manifest.is_none() {
            return Err(Error::from_reason(
                "Waker desktop detector is not loaded. Call load(bundleUrl) first.".to_string(),
            ));
        }

        let _sample_count = pcm.len();

        Ok(WakerDesktopDetectionResult {
            detected: false,
            score: 0.0,
        })
    }

    #[napi]
    pub fn dispose(&mut self) {
        self.bundle_url = None;
        self.manifest = None;
        self.manifest_json = None;
    }

    #[napi(getter, js_name = "isLoaded")]
    pub fn is_loaded(&self) -> bool {
        self.manifest.is_some()
    }

    #[napi(getter, js_name = "bundleUrl")]
    pub fn bundle_url(&self) -> Option<String> {
        self.bundle_url.clone()
    }

    #[napi(getter, js_name = "manifestJson")]
    pub fn manifest_json(&self) -> Option<String> {
        self.manifest_json.clone()
    }
}

impl Default for WakerDesktopNativeDetector {
    fn default() -> Self {
        Self::new()
    }
}

fn to_napi_error(error: AnyError) -> Error {
    Error::from_reason(format!("{error:#}"))
}
