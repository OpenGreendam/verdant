# 编译规格

Verdant 0.1.0 的完整合同见[主规格](../SPEC.md)，源码入口为
[OpenGreendam/verdant](https://github.com/OpenGreendam/verdant)。本文件规定编译输入、生成物与宿主准入；
工程持久化、事务、worker、音频、性能目标与强制 Pride 条款均适用主规格。

## 输入与所有权

| 路径 | 用途 |
|---|---|
| `frontend/crates/verdant-app/src/generated/controls.rs` | 检入的控件与命令表 Rust 源码。 |
| `frontend/crates/verdant-session/` | Rust 会话、手势守卫与领域副作用。 |
| `frontend/proto-ui/src/pitch-gesture-surface.proto.ts` | 原生根、slot 以及移动、复制、调整长度和框选的状态转换声明。 |
| `frontend/proto-ui/compiler.pin.json` | 上游提交、工具链、目标 profile 和 GPUI 版本绑定。 |
| `frontend/proto-ui/generated/pitch-gesture-surface/` | 上游编译输出、source map 与 provenance。 |
| `frontend/proto-ui/receipts/pitch-gesture-surface.json` | 源输入与生成制品的哈希记录。 |

桌面直接编译控件 Rust 源码。卷帘声明由 `frontend/proto-ui/` 下的入口调用上游 Proto UI 编译器，版本、工具链和目标见 `compiler.pin.json`。编译输出包括原生投影与独立数据方法。Rust 会话调用同一份声明编译出的状态转换逻辑，并负责守卫判断与领域副作用。宿主负责卷帘绘制和音乐坐标事件转换。

## 当前版本

上游 Proto UI 提交、包版本、Node 和 pnpm 版本及编译目标由版本锁定文件记录。生成 crate 与 SDK 共用 workspace GPUI 依赖，实际 Rust 依赖版本见 Cargo.lock。[compiler.pin.json](../frontend/proto-ui/compiler.pin.json)

历史来源审阅固定 Proto-UI `35195a6e6f1ba6a7d800da0029f2683013abd18b`，
其实验 Adapter 的 GPUI 为 `62e5991dd0f0c8a3af8d5e7e9c4652490d468db8`；
独立 API 快照为 `48741fd17d0dec28be6ad664b0a06dc7202086be`。
这些来源和哈希保留于 [SOURCES.md](SOURCES.md)，不随当前工作区版本改写。

## 编译产物与宿主接口

上游编译产物包含 Rust 源码、source map 与来源记录。投影通过 native slot 调用卷帘宿主。数据方法负责状态转换，Rust 会话将输入与守卫结果传给数据方法，执行对应的领域操作。模型、词典、音频设备和推理实现由各自 crate 提供。

普通按钮、开关、数值与选择控件经显式 target 分派。数值控件由持有实体的 render owner 提供当前值；
点击回调从实际目标读取新值再步进，不能使用过期 render 捕获。
同一用户意图经一个 `CommandBatch` 提交；preview／cancel 不修改工程。

相同输入、编译器和 profile 必须生成相同字节。生成物带输入／编译器／profile 身份与 source map，
定制逻辑留在宿主接口中；禁止手改生成文件。

## 宿主 profile 与诊断

上游编译目标由 `frontend/proto-ui/compiler.pin.json` 固定。编译产物核验、模型 trace、原生输入与目标平台验收分别记录证据。[平台验收](../frontend/docs/PLATFORM-ACCEPTANCE.md)

动态 JS、DOM／selector／React hook、无界迭代、未实现 primitive、写入只读工程投影及实时域推理均须拒绝。
错误须含源位置、失败类型／义务和目标 capability，不能生成貌似工作的空实现。

mount 建立订阅和焦点，unmount 结束临时手势并移除监听。后台回调以弱引用／会话 token 拒绝过期目标。
paint 只消费布局；worker 任务和文件处理不能阻塞 Render；音频回调不能执行编译、推理或 I/O。

## 再生成

在开发环境准备锁定的上游检出与依赖后，从仓库根目录执行：

```sh
node frontend/proto-ui/tools/build-proto-ui.mjs --upstream /path/to/Proto-UI
node frontend/proto-ui/tools/build-proto-ui.mjs --upstream /path/to/Proto-UI --check
```

第一条命令重新编译卷帘投影与手势数据方法，第二条核对产物字节与来源记录。窗口、IME 与辅助技术按目标平台独立验收。
构建方法见[前端手册](../frontend/README.md)。

## 来源与历史材料

2026-11-13 的上游确有 Rust host、style、session/epoch/lease 与有限 AccessKit，实现由仓库内 TS Runtime peer 驱动。
其 `private:true` 是包发布标记，源码公开；它与当前编译器和纯 Rust 成品分别归属。
保留原许可与归属，不复制 Prototype 私有业务逻辑到编译器以制造符合性。
声音协议 `ogd-worker/1` 与上游 host protocol 不共享消息或 framing。

[examples/](examples/) 中的协议、IR、host profile、工程和 trace 是早期设计样例；保留原状态和路径。
样例复验以记录的来源及哈希为准。实际编译、平台输入、读屏和声音分别验收。
后续控制与发音扩展见[目录索引](README.md)。当前工程入口为 `.verp`，旧 `.gdmproj`／JSON 示例只用于显式导入与历史参考。
