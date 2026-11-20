# 核验快照的上游许可

这里的许可文件按对应固定提交下载，未改写正文；来源、长度与SHA256见 [manifest.json](manifest.json)。它们说明 `../source-audit/` 中相应上游文件的来源，不是为本项目新增设计选择许可证。

- **Proto-UI**：仓库根 `LICENSE`，MIT许可，原文为 `Proto-UI-LICENSE.txt`。本次快照包括README、白皮书、contract，以及其`native/gpui` Rust工作区和`gpui-peer` TypeScript代码；原生workspace的Cargo声明同为MIT。[对应来源见manifest中的S16。] 保留原版权及许可文字。
- **GPUI**：固定提交的 `crates/gpui/Cargo.toml` 声明 `Apache-2.0`，其 `crates/gpui/LICENSE-APACHE` 在Git中为指向根 `../../LICENSE-APACHE` 的链接。`GPUI-crates-gpui-LICENSE-APACHE.txt`保留所取链接内容，`GPUI-LICENSE-APACHE.txt`保存对应根文件的完整Apache 2.0正文。

本次只核验并随包附入上述GPUI crate的源码/示例，没有据此把整个Zed仓库概括为同一种许可。审计快照用于来源复核，应用依赖和分发还应按实际选用的crate及其依赖清单记录。

Rust 产品还逐字节保留以下许可原文：

| 随附文件 | 所取源文件 | 用途 |
| --- | --- | --- |
| `DiffSinger-LICENSE.txt` | `research/python-oracle/neural/source/DiffSinger/LICENSE` | 历史接口参考源码的 Apache-2.0 许可；应用离线归属页嵌入此文件。 |
| `pinyin-data-LICENSE.txt` | `tools/intelligibility/LICENSE-pinyin-data.txt` | mozillazg pinyin-data 的 MIT 原文，保留拼音表及字典来源证据。 |

声码器的 `NOTICE.txt`、`NOTICE.zh-CN.txt`、`STATEMENTS.txt` 原文保存于
`docs/archive/research/licenses/vocoder/`，原文随历史资料保留。
