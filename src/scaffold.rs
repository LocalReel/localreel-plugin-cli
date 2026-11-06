//! `new`: write a plugin project from the embedded templates.

use std::path::{Path, PathBuf};

use crate::manifest::{Error, Manifest, PluginKind, API_VERSION, PERMISSION_NETWORK};

const RUST_CARGO: &str = include_str!("../templates/plugin/Cargo.toml.tpl");
const RUST_MAIN: &str = include_str!("../templates/plugin/src/main.rs.tpl");
const RUST_MANIFEST: &str = include_str!("../templates/plugin/manifest.json.tpl");
const RUST_BUILD_PS1: &str = include_str!("../templates/plugin/build.ps1.tpl");
const RUST_BUILD_SH: &str = include_str!("../templates/plugin/build.sh.tpl");
const RUST_README: &str = include_str!("../templates/plugin/README.md.tpl");
const NODE_PLUGIN: &str = include_str!("../templates/node/plugin.js.tpl");
const PYTHON_PLUGIN: &str = include_str!("../templates/python/plugin.py.tpl");
const SCRIPT_MANIFEST: &str = include_str!("../templates/script/manifest.json.tpl");
const SCRIPT_BUILD_PS1: &str = include_str!("../templates/script/build.ps1.tpl");
const SCRIPT_BUILD_SH: &str = include_str!("../templates/script/build.sh.tpl");
const SCRIPT_README: &str = include_str!("../templates/script/README.md.tpl");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Node,
    Python,
}

impl Language {
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "rust" => Ok(Self::Rust),
            "node" | "nodejs" => Ok(Self::Node),
            "python" | "py" => Ok(Self::Python),
            other => Err(Error::Manifest(format!(
                "unknown language {other:?} (expected rust, node or python)"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Node => "node",
            Self::Python => "python",
        }
    }

    fn program(self) -> String {
        match self {
            Self::Rust => format!("bin/plugin{}", std::env::consts::EXE_SUFFIX),
            Self::Node => "plugin.js".into(),
            Self::Python => "plugin.py".into(),
        }
    }

    fn runtime(self) -> Option<&'static str> {
        match self {
            Self::Rust => None,
            Self::Node => Some("node"),
            Self::Python => {
                if cfg!(windows) {
                    Some("python")
                } else {
                    Some("python3")
                }
            }
        }
    }
}

pub struct Created {
    pub dir: PathBuf,
    pub language: Language,
    pub files: Vec<PathBuf>,
}

pub fn create(id: &str, name: &str, dir: &Path, language: Language) -> Result<Created, Error> {
    let program = language.program();
    let runtime = language.runtime();
    let manifest = Manifest {
        id: id.to_string(),
        name: name.to_string(),
        version: "0.1.0".into(),
        api_version: API_VERSION.into(),
        kind: PluginKind::Storage,
        entry: crate::manifest::Entry {
            program: program.clone(),
            args: Vec::new(),
            runtime: runtime.map(str::to_string),
        },
        permissions: vec![PERMISSION_NETWORK.into()],
        description: None,
    };
    manifest.validate()?;

    if dir.exists() {
        let mut entries = std::fs::read_dir(dir)
            .map_err(|error| Error::Io(format!("cannot read {}: {error}", dir.display())))?;
        if entries.next().is_some() {
            return Err(Error::Io(format!("{} is not empty", dir.display())));
        }
    }
    std::fs::create_dir_all(dir.join("src"))
        .map_err(|error| Error::Io(format!("cannot create {}: {error}", dir.display())))?;

    let mut files: Vec<(String, String)> = Vec::new();
    match language {
        Language::Rust => {
            let crate_name = format!("{}_plugin", id.replace('-', "_"));
            files.push((
                "Cargo.toml".into(),
                render(RUST_CARGO, id, name, &program, runtime, &crate_name),
            ));
            files.push((
                "src/main.rs".into(),
                render(RUST_MAIN, id, name, &program, runtime, &crate_name),
            ));
            files.push((
                "manifest.json".into(),
                manifest_json(&manifest, RUST_MANIFEST),
            ));
            files.push((
                "build.ps1".into(),
                render(RUST_BUILD_PS1, id, name, &program, runtime, &crate_name),
            ));
            files.push((
                "build.sh".into(),
                render(RUST_BUILD_SH, id, name, &program, runtime, &crate_name),
            ));
            files.push((
                "README.md".into(),
                render(RUST_README, id, name, &program, runtime, language.as_str()),
            ));
        }
        Language::Node | Language::Python => {
            let script = if language == Language::Node {
                "plugin.js"
            } else {
                "plugin.py"
            };
            let source = if language == Language::Node {
                NODE_PLUGIN
            } else {
                PYTHON_PLUGIN
            };
            files.push((
                "manifest.json".into(),
                manifest_json(&manifest, SCRIPT_MANIFEST),
            ));
            files.push((
                script.into(),
                render(source, id, name, script, runtime, language.as_str()),
            ));
            files.push((
                "build.ps1".into(),
                render(
                    SCRIPT_BUILD_PS1,
                    id,
                    name,
                    script,
                    runtime,
                    language.as_str(),
                ),
            ));
            files.push((
                "build.sh".into(),
                render(
                    SCRIPT_BUILD_SH,
                    id,
                    name,
                    script,
                    runtime,
                    language.as_str(),
                ),
            ));
            files.push((
                "README.md".into(),
                render(SCRIPT_README, id, name, script, runtime, language.as_str()),
            ));
        }
    }

    let mut written = Vec::new();
    for (relative, contents) in files {
        let path = dir.join(&relative);
        std::fs::write(&path, contents)
            .map_err(|error| Error::Io(format!("cannot write {}: {error}", path.display())))?;
        written.push(path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("build.sh");
        let _ = std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755));
    }
    Ok(Created {
        dir: dir.to_path_buf(),
        language,
        files: written,
    })
}

fn manifest_json(manifest: &Manifest, template: &str) -> String {
    if template == RUST_MANIFEST {
        return render(
            template,
            &manifest.id,
            &manifest.name,
            &manifest.entry.program,
            manifest.entry.runtime.as_deref(),
            "rust",
        );
    }
    render(
        template,
        &manifest.id,
        &manifest.name,
        &manifest.entry.program,
        manifest.entry.runtime.as_deref(),
        "script",
    )
}

fn render(
    template: &str,
    id: &str,
    name: &str,
    script: &str,
    runtime: Option<&str>,
    language: &str,
) -> String {
    let runtime = runtime.unwrap_or("");
    let script_name = Path::new(script)
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| script.to_string());
    template
        .replace("{{id}}", id)
        .replace("{{name}}", name)
        .replace("{{program}}", script)
        .replace("{{script}}", &script_name)
        .replace("{{runtime}}", runtime)
        .replace("{{language}}", language)
        .replace("{{crate}}", &format!("{}_plugin", id.replace('-', "_")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_rust_project() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("example");
        let created = create("example", "Example plugin", &dir, Language::Rust).unwrap();
        assert_eq!(created.language, Language::Rust);
        assert!(dir.join("manifest.json").is_file());
        assert!(dir.join("src/main.rs").is_file());
        let manifest = Manifest::parse(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest.id, "example");
        assert!(manifest.entry.runtime.is_none());
    }

    #[test]
    fn creates_a_node_project_with_a_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("example");
        let created = create("example", "Example plugin", &dir, Language::Node).unwrap();
        assert_eq!(created.language, Language::Node);
        assert!(dir.join("plugin.js").is_file());
        assert!(!dir.join("Cargo.toml").exists());
        let manifest = Manifest::parse(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest.entry.program, "plugin.js");
        assert_eq!(manifest.entry.runtime.as_deref(), Some("node"));
    }

    #[test]
    fn creates_a_python_project_with_a_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("example");
        create("example", "Example plugin", &dir, Language::Python).unwrap();
        assert!(dir.join("plugin.py").is_file());
        let manifest = Manifest::parse(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest.entry.program, "plugin.py");
        assert!(manifest
            .entry
            .runtime
            .as_deref()
            .is_some_and(|value| value == "python" || value == "python3"));
    }

    #[test]
    fn refuses_a_non_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("keep.txt"), b"x").unwrap();
        assert!(create("example", "Example", temp.path(), Language::Rust).is_err());
    }
}
