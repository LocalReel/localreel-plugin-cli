//! The `manifest.json` contract shared with the LocalReel host.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const API_VERSION: &str = "localreel.plugin/v1";
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_PACKAGE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_EXTRACTED_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_PACKAGE_ENTRIES: usize = 4096;
pub const PERMISSION_NETWORK: &str = "network";
pub const ALLOWED_PERMISSIONS: &[&str] = &[PERMISSION_NETWORK];

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid manifest: {0}")]
    Manifest(String),
    #[error("unsupported plugin api version {0} (expected {API_VERSION})")]
    ApiVersion(String),
    #[error("manifest is too large (max {MAX_MANIFEST_BYTES} bytes)")]
    ManifestTooLarge,
    #[error("package is too large (max {MAX_PACKAGE_BYTES} bytes)")]
    PackageTooLarge,
    #[error("package has too many entries (max {MAX_PACKAGE_ENTRIES})")]
    TooManyEntries,
    #[error("package expands beyond the size limit")]
    ExtractedTooLarge,
    #[error("unsafe entry: {0}")]
    UnsafeEntry(String),
    #[error("package is missing {0}")]
    Missing(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("archive error: {0}")]
    Archive(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub kind: PluginKind,
    pub entry: Entry,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Program inside the package, relative to its root, using `/` separators.
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional interpreter (`node`, `python3`, ...) used to run the program.
    #[serde(default)]
    pub runtime: Option<String>,
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(Error::ManifestTooLarge);
        }
        let manifest: Manifest =
            serde_json::from_slice(bytes).map_err(|error| Error::Manifest(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.api_version != API_VERSION {
            return Err(Error::ApiVersion(self.api_version.clone()));
        }
        validate_id(&self.id)?;
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            return Err(Error::Manifest("name must be 1..=80 characters".into()));
        }
        if self.version.trim().is_empty()
            || self.version.chars().count() > 32
            || self.version.chars().any(char::is_control)
        {
            return Err(Error::Manifest("version is invalid".into()));
        }
        if self
            .description
            .as_ref()
            .is_some_and(|text| text.len() > 2000)
        {
            return Err(Error::Manifest(
                "description is too long (max 2000 bytes)".into(),
            ));
        }
        validate_relative_program(&self.entry.program)?;
        if let Some(runtime) = &self.entry.runtime {
            let valid = !runtime.is_empty()
                && runtime.len() <= 64
                && runtime
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_alphanumeric())
                && runtime
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'));
            if !valid {
                return Err(Error::Manifest(
                    "entry runtime must be a bare command name (no path separators)".into(),
                ));
            }
        }
        if self.entry.args.len() > 32 || self.entry.args.iter().any(|arg| arg.contains('\0')) {
            return Err(Error::Manifest(
                "entry args are invalid (max 32, no NUL)".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        for permission in &self.permissions {
            if !ALLOWED_PERMISSIONS.contains(&permission.as_str()) {
                return Err(Error::Manifest(format!(
                    "unknown permission {permission:?}"
                )));
            }
            if !seen.insert(permission) {
                return Err(Error::Manifest(format!(
                    "duplicate permission {permission:?}"
                )));
            }
        }
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<(), Error> {
    let valid = !id.is_empty()
        && id.len() <= 32
        && id
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if !valid {
        return Err(Error::Manifest(
            "id must be 1..=32 lowercase letters, digits or '-'".into(),
        ));
    }
    Ok(())
}

pub fn validate_relative_program(program: &str) -> Result<(), Error> {
    if program.trim().is_empty() || program.len() > 255 || program.contains('\0') {
        return Err(Error::Manifest("entry program is invalid".into()));
    }
    if program.contains('\\') {
        return Err(Error::Manifest(
            "entry program must use '/' separators".into(),
        ));
    }
    let bytes = program.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(Error::Manifest(
            "entry program must not contain a drive prefix".into(),
        ));
    }
    if program.starts_with('/') {
        return Err(Error::Manifest(
            "entry program must be relative to the package".into(),
        ));
    }
    let mut count = 0usize;
    for component in Path::new(program).components() {
        match component {
            Component::Normal(_) => count += 1,
            _ => {
                return Err(Error::Manifest(
                    "entry program must not contain '.', '..' or a drive prefix".into(),
                ))
            }
        }
    }
    if count == 0 {
        return Err(Error::Manifest("entry program is empty".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str, program: &str) -> String {
        serde_json::json!({
            "id": id,
            "name": "Example",
            "version": "0.1.0",
            "api_version": API_VERSION,
            "kind": "storage",
            "entry": { "program": program, "args": [] },
            "permissions": ["network"]
        })
        .to_string()
    }

    #[test]
    fn parses_a_valid_manifest() {
        let manifest = Manifest::parse(manifest("example", "bin/plugin").as_bytes()).unwrap();
        assert_eq!(manifest.id, "example");
        assert_eq!(manifest.kind, PluginKind::Storage);
    }

    #[test]
    fn rejects_bad_ids() {
        for id in [
            "",
            "Upper",
            "-lead",
            "with space",
            "0123456789012345678901234567890123",
        ] {
            assert!(
                Manifest::parse(manifest(id, "bin/plugin").as_bytes()).is_err(),
                "{id}"
            );
        }
    }

    #[test]
    fn rejects_unsafe_programs() {
        for program in [
            "../plugin",
            "bin\\plugin",
            "C:/plugin",
            "/plugin",
            "./plugin",
        ] {
            assert!(
                Manifest::parse(manifest("example", program).as_bytes()).is_err(),
                "{program}"
            );
        }
    }

    #[test]
    fn rejects_unknown_permissions() {
        let json = serde_json::json!({
            "id": "example",
            "name": "Example",
            "version": "0.1.0",
            "api_version": API_VERSION,
            "kind": "storage",
            "entry": { "program": "bin/plugin" },
            "permissions": ["filesystem"]
        })
        .to_string();
        assert!(Manifest::parse(json.as_bytes()).is_err());
    }
}
