# 规格索引

| 文档 | 内容 |
| --- | --- |
| [产品规格](../SPEC.md) | 工程、事务、合成、音频和桌面要求。 |
| [CLI 与 DSH](CLI-DSH.md) | 命令、机器协议、工程交付和验收。 |
| [多语言读音](MULTILINGUAL-PHONEMES.md) | 输入音标、映射、时长、项目字段和诊断。 |
| [编译规格](SPEC.md) | 上游卷帘声明、Rust 产物、宿主接口与重新编译。 |
| [固定来源](SOURCES.md) | 上游文件、版本、哈希及许可证。 |

控件使用检入的 Rust 源码。卷帘投影与会话手势通过 `frontend/proto-ui/` 的上游编译入口构建。`examples/` 保存带来源的历史格式示例。[编辑器](../frontend/README.md) · [worker](../backend/RUNTIME.md)
