//! Reading and writing `.lrplug` packages: a zip with `manifest.json` at its
//! root.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::ZipArchive;

use crate::manifest::{
    Error, Manifest, MAX_EXTRACTED_BYTES, MAX_MANIFEST_BYTES, MAX_PACKAGE_BYTES,
    MAX_PACKAGE_ENTRIES,
};

#[derive(Debug, Clone)]
pub struct PackReport {
    pub manifest: Manifest,
    pub path: PathBuf,
    pub files: usize,
    pub bytes: u64,
    pub sha256: String,
}

/// Read the manifest without extracting anything.
pub fn inspect(path: &Path) -> Result<Manifest, Error> {
    let file = File::open(path)
        .map_err(|error| Error::Io(format!("cannot open {}: {error}", path.display())))?;
    let mut archive = ZipArchive::new(file).map_err(|error| Error::Archive(error.to_string()))?;
    if archive.len() > MAX_PACKAGE_ENTRIES {
        return Err(Error::TooManyEntries);
    }
    let mut manifest: Option<Manifest> = None;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| Error::Archive(error.to_string()))?;
        if entry.name() != "manifest.json" {
            continue;
        }
        if manifest.is_some() {
            return Err(Error::UnsafeEntry("duplicate manifest.json".into()));
        }
        if entry.size() as usize > MAX_MANIFEST_BYTES {
            return Err(Error::ManifestTooLarge);
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .take(MAX_MANIFEST_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| Error::Io(error.to_string()))?;
        manifest = Some(Manifest::parse(&bytes)?);
    }
    manifest.ok_or_else(|| Error::Missing("manifest.json".into()))
}

/// Build a deterministic `.lrplug` from a directory containing `manifest.json`.
pub fn pack(source: &Path, output: &Path) -> Result<PackReport, Error> {
    let source = source
        .canonicalize()
        .map_err(|error| Error::Io(format!("cannot resolve {}: {error}", source.display())))?;
    if !source.is_dir() {
        return Err(Error::Io(format!(
            "{} is not a directory",
            source.display()
        )));
    }
    let manifest_bytes = std::fs::read(source.join("manifest.json"))
        .map_err(|_| Error::Missing("manifest.json".into()))?;
    let manifest = Manifest::parse(&manifest_bytes)?;
    let program = PathBuf::from(&manifest.entry.program);
    // Outputs must live outside the input tree. This also excludes stale output
    // and temporary archives on repeated packs, and protects every source file
    // (not just the manifest and executable) from accidental replacement.
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    // Check the nearest existing ancestor before creating missing output
    // directories, so a rejected source-local output does not alter the input.
    let absolute_parent = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| Error::Io(error.to_string()))?
            .join(parent)
    };
    let ancestor = absolute_parent
        .ancestors()
        .find(|path| path.exists())
        .ok_or_else(|| Error::Io("output directory has no existing ancestor".into()))?;
    if ancestor
        .canonicalize()
        .map_err(|error| Error::Io(error.to_string()))?
        .starts_with(&source)
    {
        return Err(Error::UnsafeEntry(
            "package output must be outside the source directory".into(),
        ));
    }
    std::fs::create_dir_all(parent).map_err(|error| Error::Io(error.to_string()))?;
    let output_parent = parent
        .canonicalize()
        .map_err(|error| Error::Io(format!("cannot resolve output directory: {error}")))?;
    if output_parent.starts_with(&source)
        || output
            .canonicalize()
            .is_ok_and(|path| path.starts_with(&source))
    {
        return Err(Error::UnsafeEntry(
            "package output must be outside the source directory".into(),
        ));
    }
    let mut files = collect_files(&source)?;
    if !files.iter().any(|path| path == &program) {
        return Err(Error::Missing(manifest.entry.program.clone()));
    }
    files.sort();
    if let Some(index) = files
        .iter()
        .position(|path| path == Path::new("manifest.json"))
    {
        let entry = files.remove(index);
        files.insert(0, entry);
    }
    if files.len() > MAX_PACKAGE_ENTRIES {
        return Err(Error::TooManyEntries);
    }

    let name = output
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "plugin.lrplug".into());
    let temp = output.with_file_name(format!(".{name}.tmp-{}", std::process::id()));
    if let Err(error) = write_package(&temp, &source, &files, &program) {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    let size = std::fs::metadata(&temp)
        .map_err(|error| Error::Io(error.to_string()))?
        .len();
    if size > MAX_PACKAGE_BYTES {
        let _ = std::fs::remove_file(&temp);
        return Err(Error::PackageTooLarge);
    }
    if output.exists() {
        std::fs::remove_file(output)
            .map_err(|error| Error::Io(format!("cannot replace {}: {error}", output.display())))?;
    }
    std::fs::rename(&temp, output).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        Error::Io(format!("cannot write {}: {error}", output.display()))
    })?;

    let inspected = inspect(output)?;
    if inspected.id != manifest.id || inspected.version != manifest.version {
        return Err(Error::Archive("packed manifest does not round-trip".into()));
    }
    Ok(PackReport {
        manifest,
        path: output.to_path_buf(),
        files: files.len(),
        bytes: size,
        sha256: sha256_file(output)?,
    })
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| Error::Io(error.to_string()))?;
        if entry.path() == root {
            continue;
        }
        let file_type = entry.file_type();
        if file_type.is_symlink() {
            return Err(Error::UnsafeEntry(format!(
                "symlink {}",
                entry.path().display()
            )));
        }
        if file_type.is_dir() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| Error::UnsafeEntry(entry.path().display().to_string()))?
            .to_path_buf();
        if relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(Error::UnsafeEntry(relative.display().to_string()));
        }
        files.push(relative);
    }
    if files.is_empty() {
        return Err(Error::Missing("manifest.json".into()));
    }
    Ok(files)
}

fn write_package(
    temp: &Path,
    source: &Path,
    files: &[PathBuf],
    program: &Path,
) -> Result<(), Error> {
    let file = File::create(temp)
        .map_err(|error| Error::Io(format!("cannot create {}: {error}", temp.display())))?;
    let mut writer = zip::ZipWriter::new(file);
    let mut total = 0u64;
    for relative in files {
        let bytes = std::fs::read(source.join(relative))
            .map_err(|error| Error::Io(format!("cannot read {}: {error}", relative.display())))?;
        total = total.saturating_add(bytes.len() as u64);
        if total > MAX_EXTRACTED_BYTES {
            return Err(Error::ExtractedTooLarge);
        }
        let name = relative.to_string_lossy().replace('\\', "/");
        let mode = if relative == program || relative.starts_with("bin") {
            0o755
        } else {
            0o644
        };
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(mode);
        writer
            .start_file(name, options)
            .map_err(|error| Error::Archive(error.to_string()))?;
        writer
            .write_all(&bytes)
            .map_err(|error| Error::Io(error.to_string()))?;
    }
    let file = writer
        .finish()
        .map_err(|error| Error::Archive(error.to_string()))?;
    file.sync_all()
        .map_err(|error| Error::Io(error.to_string()))?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, Error> {
    let mut file = File::open(path)
        .map_err(|error| Error::Io(format!("cannot read {}: {error}", path.display())))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|error| Error::Io(error.to_string()))?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_outputs_inside_source_without_overwriting_inputs() {
        let root = tempfile::tempdir().unwrap();
        let source = plugin_dir(root.path());
        let original = std::fs::read(source.join("manifest.json")).unwrap();
        for relative in ["manifest.json", "bin/plugin", "output.lrplug"] {
            assert!(pack(&source, &source.join(relative)).is_err());
        }
        assert_eq!(
            std::fs::read(source.join("manifest.json")).unwrap(),
            original
        );
        assert!(pack(&source, &source.join("new/sub/output.lrplug")).is_err());
        assert!(!source.join("new").exists());
        let output = root.path().join("dist/nested/output.lrplug");
        let first = pack(&source, &output).unwrap();
        let second = pack(&source, &output).unwrap();
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(first.files, second.files);
    }

    fn plugin_dir(root: &Path) -> PathBuf {
        let dir = root.join("plugin");
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::json!({
                "id": "example",
                "name": "Example",
                "version": "0.1.0",
                "api_version": crate::manifest::API_VERSION,
                "kind": "storage",
                "entry": { "program": "bin/plugin", "args": [] },
                "permissions": ["network"]
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(dir.join("bin/plugin"), b"#!/bin/sh\n").unwrap();
        std::fs::write(dir.join("README.txt"), b"hello").unwrap();
        dir
    }

    #[test]
    fn packs_and_inspects() {
        let temp = tempfile::tempdir().unwrap();
        let dir = plugin_dir(temp.path());
        let output = temp.path().join("example.lrplug");
        let report = pack(&dir, &output).unwrap();
        assert_eq!(report.manifest.id, "example");
        assert_eq!(report.files, 3);
        assert_eq!(report.sha256.len(), 64);
        assert_eq!(inspect(&output).unwrap().id, "example");
    }

    #[test]
    fn packing_is_deterministic() {
        let temp = tempfile::tempdir().unwrap();
        let dir = plugin_dir(temp.path());
        let first = pack(&dir, &temp.path().join("a.lrplug")).unwrap();
        let second = pack(&dir, &temp.path().join("b.lrplug")).unwrap();
        assert_eq!(first.sha256, second.sha256);
    }

    #[test]
    fn rejects_a_missing_entry_program() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("plugin");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::json!({
                "id": "example",
                "name": "Example",
                "version": "0.1.0",
                "api_version": crate::manifest::API_VERSION,
                "kind": "storage",
                "entry": { "program": "bin/missing" }
            })
            .to_string(),
        )
        .unwrap();
        assert!(matches!(
            pack(&dir, &temp.path().join("bad.lrplug")),
            Err(Error::Missing(_))
        ));
    }
}
