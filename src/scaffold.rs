//! `new`: write a working plugin project from the embedded templates.

use std::path::{Path, PathBuf};

use crate::manifest::{Error, Manifest, PluginKind, API_VERSION, PERMISSION_NETWORK};

const CARGO_TOML: &str = include_str!("../templates/plugin/Cargo.toml.tpl");
const MAIN_RS: &str = include_str!("../templates/plugin/src/main.rs.tpl");
const MANIFEST: &str = include_str!("../templates/plugin/manifest.json.tpl");
const BUILD_PS1: &str = include_str!("../templates/plugin/build.ps1.tpl");
const BUILD_SH: &str = include_str!("../templates/plugin/build.sh.tpl");
const README: &str = include_str!("../templates/plugin/README.md.tpl");

pub struct Created {
    pub dir: PathBuf,
    pub crate_name: String,
    pub files: Vec<PathBuf>,
}

pub fn create(id: &str, name: &str, dir: &Path) -> Result<Created, Error> {
    let exe_suffix = std::env::consts::EXE_SUFFIX;
    let crate_name = format!("{}_plugin", id.replace('-', "_"));
    let manifest = Manifest {
        id: id.to_string(),
        name: name.to_string(),
        version: "0.1.0".into(),
        api_version: API_VERSION.into(),
        kind: PluginKind::Storage,
        entry: crate::manifest::Entry {
            program: format!("bin/plugin{exe_suffix}"),
            args: Vec::new(),
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

    let files = [
        (
            "Cargo.toml",
            render(CARGO_TOML, &crate_name, id, name, exe_suffix),
        ),
        (
            "manifest.json",
            render(MANIFEST, &crate_name, id, name, exe_suffix),
        ),
        (
            "src/main.rs",
            render(MAIN_RS, &crate_name, id, name, exe_suffix),
        ),
        (
            "build.ps1",
            render(BUILD_PS1, &crate_name, id, name, exe_suffix),
        ),
        (
            "build.sh",
            render(BUILD_SH, &crate_name, id, name, exe_suffix),
        ),
        (
            "README.md",
            render(README, &crate_name, id, name, exe_suffix),
        ),
    ];
    let mut written = Vec::new();
    for (relative, contents) in files {
        let path = dir.join(relative);
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
        crate_name,
        files: written,
    })
}

fn render(template: &str, crate_name: &str, id: &str, name: &str, exe_suffix: &str) -> String {
    template
        .replace("{{crate}}", crate_name)
        .replace("{{id}}", id)
        .replace("{{name}}", name)
        .replace("{{exe_suffix}}", exe_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_project() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("example");
        let created = create("example", "Example plugin", &dir).unwrap();
        assert_eq!(created.crate_name, "example_plugin");
        assert!(dir.join("manifest.json").is_file());
        assert!(dir.join("src/main.rs").is_file());
        let manifest = Manifest::parse(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest.id, "example");
        assert_eq!(
            manifest.entry.program,
            format!("bin/plugin{}", std::env::consts::EXE_SUFFIX)
        );
        assert!(!std::fs::read_to_string(dir.join("Cargo.toml"))
            .unwrap()
            .contains("{{crate}}"));
    }

    #[test]
    fn refuses_a_non_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("keep.txt"), b"x").unwrap();
        assert!(create("example", "Example", temp.path()).is_err());
    }
}
