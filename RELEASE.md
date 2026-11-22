# verdant-release(1)

## 构建

编辑器、CLI、worker、声库安装器和打包器共用 [Cargo 工作区版本](frontend/Cargo.toml)。在目标系统的开发环境构建。Linux 使用 [开发容器](frontend/Dockerfile)，Windows x64 使用 MSVC 和 Windows SDK，Apple Silicon 使用带 Metal 工具链的 Xcode。

```sh
cd frontend
cargo fetch --locked
cargo build --offline --locked --release \
  -p verdant-app -p verdant-cli -p verdant-launcher -p verdant-worker -p verdant-plughost \
  -p verdant-voice -p verdant-release
cd ..
```

设置 `CARGO_TARGET_DIR` 时，选择其中的 `release/`。构建环境的 Rust 版本见 [rust-toolchain.toml](frontend/rust-toolchain.toml)。

## 用法

`verdant-release runtime-inputs --target TARGET` 输出 ONNX Runtime 1.23.2 CPU 输入的 URL、文件和 SHA-256。打包器把 wheel 当作 ZIP 读取，只提取原生库和许可文件。也可提供预先解包的目录。打包命令在目标平台运行，输入可事先下载，组装可离线完成。

```sh
release=frontend/target/release/verdant-release
selection_sha=$("$release" select-source --repo . --output build/source.json | jq -er .sha256)
"$release" package --repo . --binaries frontend/target/release --output dist \
  --source-selection build/source.json --source-selection-sha256 "$selection_sha" \
  --onnxruntime-wheel /path/to/pinned-runtime.whl
"$release" check --check dist --require-package --expected-source-commit "$(git rev-parse HEAD)"
```

`--onnxruntime-dir DIR` 接受同一版本的原生库、`LICENSE` 与 `ThirdPartyNotices.txt`。Windows 输出 ZIP，Linux 和 Apple Silicon 输出 tar.gz。macOS 产物包含临时签名的应用，分发签名和公证按渠道办理。

打包器检查受选源码与实际文件，附可重建的对应源码、上游许可、`RELEASE-ASSETS.json` 和 SHA-256。已有同名文件时拒绝覆盖。`check` 校验产物、嵌套源码归档和包成员。软件包由 [OpenGreendam/verdant](https://github.com/OpenGreendam/verdant/releases) 发行。

### 公开验证源码

对应源码包含用于复核 Linux 原生输入和辅助功能的开发工具。Python 分类仅接受下列具体路径，清单由 [source.rs](frontend/crates/verdant-release/src/source.rs) 执行；不按整个 `tools/` 目录或所有 `.py` 文件放行。

| 目录 | 已分类的 Python 源码 |
|---|---|
| `tools/a11y/` | `observe-atspi.py`、`run-isolated.py` |
| `tools/native-input/` | `ci.py`、`probe.py`、`run-linux-isolated.py`、`x11_screenshot.py`、`test_ci.py`、`test_probe.py` |

两个目录中的 `README.md` 按既有 Markdown 文档规则纳入。公开的验证 Markdown/JSON 记录也必须先提交，再由 `select-source` 从实际 HEAD 选择；未跟踪文档、已选择文件的工作树改动，以及未分类的新增 Python 路径都会被拒绝。选择清单记录每个文件的 Git blob、SHA-256、大小和 mode，并由嵌套源码检查重新核对。运行日志、截图、依赖与缓存仍受既有排除规则约束。

这些脚本用于开发和 CI 验证，只作为对应源码收录；它们不成为应用或 worker 的生产 Python 依赖，不改变 `production_python_required: false`。增加新验证脚本时应明确更新分类并通过源码选择和归档检查。

运行包使用 `verdant-voice` 所需的 ONNX Runtime。历史 OpenVPI 声码器通知随对应源码保存。[原文通知](docs/archive/research/licenses/vocoder/)

## 命令入口与离线技能

当前原生包清单为 `verdant-native-package/3`。`verdant` 和兼容别名 `verdant-cli` 是无 GUI 依赖的 CLI，默认显示帮助；`verdant gui` 解析同包 `verdant-gui` 并启动。归档根目录提供可执行 `verdant-launcher`，Windows 使用无额外控制台的 `verdant-launcher.exe`。macOS `.app` 的主程序为 `verdant-gui`。

八个 native 程序、根启动器权限、Windows GUI/console 子系统和离线 `share/verdant/skills` 清单均由打包器检查。组装阶段在空格／Unicode 路径和不同 cwd 运行实际 CLI help、GUI 路径探测及隔离技能安装／幂等重装／卸载；这些命令不打开 GUI，不改当前用户技能目录。源码包同时包括 manifest、全部技能内容、生成脚本和安装器源码。[完整入口与安装说明](frontend/docs/AGENT-AUTOMATION.md)。

## 声库

正式发行默认随包交付 Verdant Voice 0.1.0 ONNX 基座。先按[发行清单](frontend/packaging/verdant-voice/POLICY.json)准备完整声库目录，再构建 worker 和组装软件包。验收在干净、断网环境中读取首次启动的 worker 能力，并用随附声库完成合成。源码构建可用[本地安装](docs/verdant-voice.md)指定用户已有的模型。

使用 `--voice-bundle` 组装声库时，传入 `--artifact-policy` 和其 SHA-256。策略绑定每项运行素材的角色、SHA-256、许可和再分发授权，记录权利人许可或发行方对该清单的授权声明。以相同摘要设置 `VERDANT_BUNDLED_VOICE_POLICY_SHA256` 后构建 worker。检查时提供同一份外部策略并使用 `--require-voice`。本地使用声明只用于本地安装。

```sh
policy=frontend/packaging/verdant-voice/POLICY.json
policy_sha=$(sha256sum "$policy" | cut -d ' ' -f 1)
VERDANT_BUNDLED_VOICE_POLICY_SHA256="$policy_sha" \
  cargo build --manifest-path frontend/Cargo.toml --offline --locked --release -p verdant-worker
"$release" package --repo . --binaries frontend/target/release --output dist \
  --source-selection build/source.json --source-selection-sha256 "$selection_sha" \
  --onnxruntime-wheel /path/to/pinned-runtime.whl --voice-bundle /path/to/voice-base \
  --artifact-policy "$policy" --artifact-policy-sha256 "$policy_sha"
"$release" check --check dist --require-package --require-voice \
  --artifact-policy "$policy" --artifact-policy-sha256 "$policy_sha"
```

0.1.0 的分发依据为[发行方声明](frontend/packaging/verdant-voice/AUTHORIZATION.json)。基座内的 `rights.json` 保留本地使用的历史记录。原始安装器、单独的 `sn16k.bin` 和训练录音留在本地。源包、npm 包和工程交付包按各自清单组装。[归属](NOTICE.md) · [来源政策](https://github.com/OpenGreendam/docs/blob/main/MODEL-FORMAT-AND-RELEASE.md)

## 验收

发布前在每个目标环境检查启动、版本、原生库、安装和回滚，并运行实际工程合成。Windows x64、Linux、Apple Silicon 的收据分别记载。CLI 与 DSH 以同工程、同有效输入核对产物。[平台验收](frontend/docs/PLATFORM-ACCEPTANCE.md) · [CLI／DSH](spec/CLI-DSH.md)
