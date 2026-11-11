# LocalReel Plugin CLI

Scaffold, validate and pack [LocalReel](https://github.com/LocalReel) sidecar
plugins.

A plugin is a small executable that speaks the sidecar protocol over loopback
HTTP: the host passes a per-run token and configuration, and the plugin answers
`stat`, `list`, `walk` and `read` requests. Packages use the `.lrplug` extension:
a zip with `manifest.json` at its root.

See the protocol reference:
<https://github.com/LocalReel/localreel-core/blob/main/docs/plugin-protocol.md>

A complete provider built with this tool:
<https://github.com/LocalReel/localreel-plugin-strm>

## Install

Building the CLI from source requires **Rust 1.88 or newer** (including the
locked compression dependencies). CI checks this minimum with `--locked`.

Prebuilt binaries are published as GitHub releases for Windows x64, Linux
x64/arm64 and macOS x64/arm64.

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/LocalReel/localreel-plugin-cli/main/scripts/install.ps1 | iex
# add -AddToPath when passing the script explicitly:
# .\install.ps1 -AddToPath
```

Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/LocalReel/localreel-plugin-cli/main/scripts/install.sh | sh
```

The scripts verify the published `.sha256` and install to
`%LOCALAPPDATA%\localreel-plugin-cli` (Windows) or `~/.local/bin` (Unix). You can
also download an archive from
[Releases](https://github.com/LocalReel/localreel-plugin-cli/releases) manually.
Building from source is supported too: `cargo build --release`.

## Commands

### `new`

```sh
localreel-plugin-cli new example --dir plugins/example --name "Example storage"
localreel-plugin-cli new example --lang node --dir plugins/example
localreel-plugin-cli new example --lang python --dir plugins/example
```

`--lang` accepts `rust` (default), `node` and `python`.

Rust projects contain a crate and compile to a self-contained binary:

```text
plugins/example/
├── Cargo.toml
├── manifest.json
├── src/main.rs      # sidecar server with a local-directory provider
├── build.ps1        # stages pkg/ on Windows
├── build.sh         # stages pkg/ on Unix
└── README.md
```

Node.js and Python projects have no compile step; the build script only stages
the manifest and the script:

```text
plugins/example/
├── manifest.json    # entry.runtime = "node" or "python3"
├── plugin.js        # or plugin.py
├── build.ps1
├── build.sh
└── README.md
```

The manifest then carries `entry.runtime`; the host runs `node plugin.js` (or
`python plugin.py`) and needs that interpreter on `PATH`. Edit the generated
script to implement your provider.

### `build` (generated project)

```sh
./build.sh            # Windows: .\build.ps1
```

Rust projects compile the crate and write `pkg/manifest.json` plus
`pkg/bin/plugin`; script projects copy the manifest and the script into `pkg/`.

### `pack`

```sh
localreel-plugin-cli pack pkg --out dist/example.lrplug
```

- reads and validates `manifest.json`;
- requires the entry program to exist inside the directory;
- rejects symlinks, `..`, absolute paths and Windows drive prefixes;
- requires output outside the source directory, preventing input overwrites and
  inclusion of an older package when packing again;
- writes entries sorted with fixed timestamps, so the same input produces the
  same SHA-256;
- prints the file count, size and SHA-256; `--force` replaces an existing file.

### `inspect`

```sh
localreel-plugin-cli inspect dist/example.lrplug
```

Prints the manifest of a package without extracting it.

## Install a plugin

```toml
# localreel.toml
[[plugins]]
package = "dist/example.lrplug"

[plugins.config]
root = "D:/media"
```

## Build

```sh
cargo build --release
cargo test
```

## 简体中文

LocalReel 插件命令行工具，用来生成、校验、打包 sidecar 插件（`.lrplug`）。

- `new`：生成一个可编译的 Rust 插件工程，包含 manifest、sidecar 服务器和构建脚本；
- `pack`：校验 manifest、检查入口程序、拒绝符号链接与越界路径，生成确定性 zip
  （同样输入得到同样的 SHA-256）；
- `inspect`：不下载、不解压，直接读取包内 manifest。

预编译二进制发布在 Releases（Windows x64、Linux x64/arm64、macOS x64/arm64），
也可以用安装脚本：

```sh
# Windows PowerShell
irm https://raw.githubusercontent.com/LocalReel/localreel-plugin-cli/main/scripts/install.ps1 | iex
# Linux / macOS
curl -fsSL https://raw.githubusercontent.com/LocalReel/localreel-plugin-cli/main/scripts/install.sh | sh
```

也可以 `cargo build --release` 自行构建。

插件协议见
[plugin-protocol.md](https://github.com/LocalReel/localreel-core/blob/main/docs/plugin-protocol.md)。
