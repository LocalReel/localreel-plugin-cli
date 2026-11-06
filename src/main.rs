//! Scaffold, validate and pack LocalReel sidecar plugins.

mod manifest;
mod package;
mod scaffold;

use std::path::PathBuf;

fn usage(message: &str) -> ! {
    eprintln!("{message}");
    eprintln!(
        "usage:\n  localreel-plugin-cli new <id> [--dir <dir>] [--name <name>] [--lang rust|node|python]\n  localreel-plugin-cli pack <dir> --out <file> [--force]\n  localreel-plugin-cli inspect <file>"
    );
    std::process::exit(2);
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> String {
    args.next()
        .unwrap_or_else(|| usage(&format!("{flag} needs a value")))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        usage("missing command");
    };
    match command.as_str() {
        "new" => {
            let id = args
                .next()
                .unwrap_or_else(|| usage("new needs a plugin id"));
            let mut dir: Option<PathBuf> = None;
            let mut name: Option<String> = None;
            let mut language = scaffold::Language::Rust;
            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--dir" => dir = Some(PathBuf::from(value(&mut args, "--dir"))),
                    "--name" => name = Some(value(&mut args, "--name")),
                    "--lang" => {
                        let raw = value(&mut args, "--lang");
                        language = scaffold::Language::parse(&raw)
                            .unwrap_or_else(|error| usage(&error.to_string()));
                    }
                    other => usage(&format!("unknown argument {other:?}")),
                }
            }
            let dir = dir.unwrap_or_else(|| PathBuf::from(&id));
            let name = name.unwrap_or_else(|| id.clone());
            match scaffold::create(&id, &name, &dir, language) {
                Ok(created) => {
                    println!(
                        "created plugin {} in {} ({} files, {})",
                        id,
                        created.dir.display(),
                        created.files.len(),
                        created.language.as_str()
                    );
                    println!(
                        "next: cd {} && {} && localreel-plugin-cli pack pkg --out dist/{}.lrplug",
                        created.dir.display(),
                        if cfg!(windows) {
                            ".\\build.ps1"
                        } else {
                            "./build.sh"
                        },
                        id
                    );
                }
                Err(error) => {
                    eprintln!("new failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        "pack" => {
            let dir = PathBuf::from(
                args.next()
                    .unwrap_or_else(|| usage("pack needs a directory")),
            );
            let mut out: Option<PathBuf> = None;
            let mut force = false;
            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--out" => out = Some(PathBuf::from(value(&mut args, "--out"))),
                    "--force" => force = true,
                    other => usage(&format!("unknown argument {other:?}")),
                }
            }
            let out = out.unwrap_or_else(|| usage("--out is required"));
            if out.exists() && !force {
                eprintln!(
                    "{} already exists; pass --force to replace it",
                    out.display()
                );
                std::process::exit(1);
            }
            match package::pack(&dir, &out) {
                Ok(report) => {
                    println!(
                        "packed {} {} -> {} ({} files, {} bytes)",
                        report.manifest.id,
                        report.manifest.version,
                        report.path.display(),
                        report.files,
                        report.bytes
                    );
                    println!("sha256: {}", report.sha256);
                }
                Err(error) => {
                    eprintln!("pack failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        "inspect" => {
            let path = PathBuf::from(
                args.next()
                    .unwrap_or_else(|| usage("inspect needs a package")),
            );
            match package::inspect(&path) {
                Ok(manifest) => {
                    println!("id: {}", manifest.id);
                    println!("name: {}", manifest.name);
                    println!("version: {}", manifest.version);
                    println!("api_version: {}", manifest.api_version);
                    println!("kind: storage");
                    println!("entry: {}", manifest.entry.program);
                    println!("permissions: {}", manifest.permissions.join(", "));
                }
                Err(error) => {
                    eprintln!("inspect failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        "--version" | "-V" => {
            println!("localreel-plugin-cli {}", env!("CARGO_PKG_VERSION"));
        }
        "--help" | "-h" | "help" => {
            println!(
                "usage:\n  localreel-plugin-cli new <id> [--dir <dir>] [--name <name>] [--lang rust|node|python]\n  localreel-plugin-cli pack <dir> --out <file> [--force]\n  localreel-plugin-cli inspect <file>"
            );
        }
        other => usage(&format!("unknown command {other:?}")),
    }
}
