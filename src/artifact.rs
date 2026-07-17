//! Deterministic manifests for Partizan-oriented dataset artifacts.

use crate::dataset_label;
use ring::digest::{SHA256, digest};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Component, Path},
};

/// Schema identifier written into every Astralbase artifact manifest.
pub const ARTIFACT_MANIFEST_SCHEMA_VERSION: &str = "astralbase.artifact-manifest.v0";

/// A deterministic dataset-generation manifest.
///
/// No timestamp or machine-specific path is stored, so two runs of the same
/// crate version and command produce byte-identical files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactManifest {
    /// Version of the manifest schema.
    pub schema_version: String,
    /// Stable generator identifier.
    pub generator: String,
    /// Astralbase package version used to generate the artifact.
    pub generator_version: String,
    /// Normalized command that describes the generation operation.
    pub command: String,
    /// Whether the generator contract excludes time, randomness, and host paths.
    pub deterministic: bool,
    /// Files covered by the manifest, in stable lexical order.
    pub files: Vec<ArtifactFile>,
}

/// Size and cryptographic checksum for one artifact payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactFile {
    /// Slash-separated path relative to the manifest directory.
    pub path: String,
    /// Payload length in bytes.
    pub bytes: u64,
    /// Lowercase SHA-256 digest of the exact payload bytes.
    pub sha256: String,
}

impl ArtifactManifest {
    /// Serializes the manifest as stable pretty JSON ending in one newline.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut output = serde_json::to_string_pretty(self)?;
        output.push('\n');
        Ok(output)
    }
}

/// Writes the checked sample shard and `manifest.json` into `output_directory`.
///
/// Existing files with the same names are replaced. The returned manifest is
/// identical to the JSON written to disk.
pub fn write_sample_dataset_artifact(output_directory: &Path) -> io::Result<ArtifactManifest> {
    fs::create_dir_all(output_directory)?;
    let payload = dataset_label::sample_audited_shard_jsonl().map_err(io::Error::other)?;
    let payload_path = output_directory.join("sample-label-shard.jsonl");
    fs::write(&payload_path, payload.as_bytes())?;

    let manifest = ArtifactManifest {
        schema_version: ARTIFACT_MANIFEST_SCHEMA_VERSION.to_owned(),
        generator: "astralbase.sample-audited-shard".to_owned(),
        generator_version: env!("CARGO_PKG_VERSION").to_owned(),
        command: "astralbase --sample-label-artifact <OUTPUT_DIRECTORY>".to_owned(),
        deterministic: true,
        files: vec![ArtifactFile {
            path: "sample-label-shard.jsonl".to_owned(),
            bytes: payload.len() as u64,
            sha256: sha256_hex(payload.as_bytes()),
        }],
    };

    let manifest_json = manifest.to_json().map_err(io::Error::other)?;
    fs::write(output_directory.join("manifest.json"), manifest_json)?;
    Ok(manifest)
}

/// Verifies manifest structure, payload lengths, and SHA-256 checksums.
///
/// The manifest may reference only normal relative paths beneath its own
/// directory. An error string names the first failed invariant.
pub fn verify_artifact_manifest(manifest_path: &Path) -> Result<ArtifactManifest, String> {
    let manifest_text = fs::read_to_string(manifest_path)
        .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?;
    let manifest: ArtifactManifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("invalid manifest JSON: {error}"))?;

    if manifest.schema_version != ARTIFACT_MANIFEST_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {:?}",
            manifest.schema_version
        ));
    }
    if !manifest.deterministic {
        return Err("manifest does not declare deterministic generation".to_owned());
    }

    let base = manifest_path
        .parent()
        .ok_or_else(|| "manifest path has no parent directory".to_owned())?;
    let mut previous_path: Option<&str> = None;
    for file in &manifest.files {
        if file.path.is_empty()
            || Path::new(&file.path).is_absolute()
            || Path::new(&file.path)
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("unsafe artifact path {:?}", file.path));
        }
        if previous_path.is_some_and(|previous| previous >= file.path.as_str()) {
            return Err("manifest file paths must be unique and lexically sorted".to_owned());
        }
        previous_path = Some(file.path.as_str());

        let payload = fs::read(base.join(&file.path))
            .map_err(|error| format!("could not read {}: {error}", file.path))?;
        if payload.len() as u64 != file.bytes {
            return Err(format!("byte length mismatch for {}", file.path));
        }
        if sha256_hex(&payload) != file.sha256 {
            return Err(format!("SHA-256 mismatch for {}", file.path));
        }
    }

    Ok(manifest)
}

fn sha256_hex(payload: &[u8]) -> String {
    let value = digest(&SHA256, payload);
    let mut output = String::with_capacity(64);
    for byte in value.as_ref() {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sample_manifest_is_deterministic_and_verifiable() {
        let root =
            std::env::temp_dir().join(format!("astralbase-artifact-test-{}", std::process::id()));
        let first = root.join("first");
        let second = root.join("second");
        let _ = fs::remove_dir_all(&root);

        let first_manifest = write_sample_dataset_artifact(&first).unwrap();
        let second_manifest = write_sample_dataset_artifact(&second).unwrap();
        assert_eq!(first_manifest, second_manifest);
        assert_eq!(
            fs::read(first.join("sample-label-shard.jsonl")).unwrap(),
            fs::read(second.join("sample-label-shard.jsonl")).unwrap()
        );
        assert_eq!(
            fs::read(first.join("manifest.json")).unwrap(),
            fs::read(second.join("manifest.json")).unwrap()
        );
        assert_eq!(
            verify_artifact_manifest(&first.join("manifest.json")).unwrap(),
            first_manifest
        );

        fs::remove_dir_all(root).unwrap();
    }
}
