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

Prebuilt binaries are published as GitHub releases:

- Windows x64: `localreel-plugin-cli-windows-x64.zip`
- Linux x64: `localreel-plugin-cli-linux-x64.tar.gz`

Unpack the archive and run `localreel-plugin-cli --help`. Building from source
is also supported: `cargo build --release`.

## Commands

### `new`

```sh
localreel-plugin-cli new example --dir plugins/example --name "Example storage"
```

Writes a complete Rust project:

```text
plugins/example/
├── Cargo.toml
├── manifest.json
├── src/main.rs      # sidecar server with a local-directory provider
├── build.ps1        # stages pkg/ on Windows
├── build.sh         # stages pkg/ on Unix
└── README.md
```

`manifest.json` uses api version `localreel.plugin/v1`, kind `storage`, and entry
`bin/plugin` (`.exe` on Windows). Edit `src/main.rs` to implement your provider;
the generated code is a working starting point.

### `build` (generated project)

```sh
./build.sh            # Windows: .\build.ps1
```

This compiles the crate and writes `pkg/manifest.json` plus `pkg/bin/plugin`.

### `pack`

```sh
localreel-plugin-cli pack pkg --out dist/example.lrplug
```

- reads and validates `manifest.json`;
- requires the entry program to exist inside the directory;
- rejects symlinks, `..`, absolute paths and Windows drive prefixes;
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

预编译二进制发布在 Releases（Windows x64 zip、Linux x64 tar.gz），解压即可用；
也可以 `cargo build --release` 自行构建。

插件协议见
[plugin-protocol.md](https://github.com/LocalReel/localreel-core/blob/main/docs/plugin-protocol.md)。
