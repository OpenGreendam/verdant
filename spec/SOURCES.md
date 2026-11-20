# 固定官方来源

本页记录历史来源审阅；当前合同见[主规格](../SPEC.md)，当前编译版本见[编译规格](SPEC.md)。
Verdant 源码入口为 [OpenGreendam/verdant](https://github.com/OpenGreendam/verdant)。下表固定 URL、哈希和原始版本声明保持不变。

核验日期：2026-11-13。Proto-UI commit `35195a6e6f1ba6a7d800da0029f2683013abd18b`。其现有原生GPUI Adapter固定Zed/GPUI `62e5991dd0f0c8a3af8d5e7e9c4652490d468db8`；本规格独立审阅的较新GPUI API快照是 `48741fd17d0dec28be6ad664b0a06dc7202086be`。两版本的兼容性未验证，不得混用证据。

网页主入口是 [Proto-UI翻译层白皮书](https://www.proto-ui.com/en/whitepaper/5-translation-layer/)，证据固定到下表仓库文件，不依赖网页未来内容不变。SHA256对下载原始字节计算；Git blob SHA1按Git对象头与原始字节计算，完整值见 [source-manifest.json](source-manifest.json)。

补充核验确认已有真实Rust GPUI Adapter工作区，语义仍由未发布的仓库内TS peer运行现有Runtime；其private是`package.json`的`private: true`发布标记，源码公开可见。不能把未发布公共profile或未完成Compiler误写成没有任何GPUI实现。本次未执行上游CI、原生窗口测试或跨GPUI版本迁移。

| 标记 | 官方固定链接 | SHA256 |
|---|---|---|
| S01 | [Proto-UI README / 当前发布边界](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/README.md) | `1c8d362448ae191d9216d6536c289fbf2554ae88f40c0e3d6368489039fe6491` |
| S02 | [翻译层白皮书 / Compiler与证据范围](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/apps/www/src/content/docs/en/whitepaper/5-translation-layer.md) | `8ba726cedbdb4b75b06f5e55c161972efe5c3c4e5868f8d5a7c010d7bc27ecf7` |
| S03 | [模板规范化contract](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/internal/contracts/template/normalize.v0.md) | `e1133c76f974fe485e935cc833f8f3a0915a38c9b3668f7b95909cc816c97fa3` |
| S04 | [事件contract](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/internal/contracts/event/event.v0.md) | `0f790cc64b0e3106f15ace63a840e2be800771e3dd2c96348dad719c1ade1bac` |
| S05 | [GPUI README / Entity、Render与平台入口](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/README.md) | `989b10e5914dcc8a1f2568303849aeab2d6aa26f2c5dfead96de0491330ada79` |
| S06 | [GPUI完整文本输入示例](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/examples/input.rs) | `0c2f9c60620f83896e78b24e46e0e3fadb5dcdcd712d470437e402406d178547` |
| S07 | [EntityInputHandler与UTF-16接口](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/src/input.rs) | `0cec4099126edf35806ede65a80c73ead8bf3db464c5a5cc02bae1f64f20130c` |
| S08 | [Canvas prepaint/paint接口](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/src/elements/canvas.rs) | `df0a482dd80561cfed49e81dd2fdc833f713c04460ce126851bf53d937779e3a` |
| S09 | [原生拖放示例](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/examples/drag_drop.rs) | `4011afde7c64fac2559b78918e6fdabdc563fe9f7591c74425d72da906dcc47a` |
| S10 | [GPUI AccessKit无障碍指南](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/src/_accessibility.rs) | `bcdeb209fa6309f5c507d9e8918235e75c18751c1985d0ebed632de643d7ea2e` |
| S11 | [GPUI模块与公开接口](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/src/gpui.rs) | `05e066b0294e08febde9737ea32cb589f06c160df3e610432acecc093fed4060` |
| S12 | [GPUI crate元数据与许可](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/Cargo.toml) | `0ff233f390120f67b85b1fdedfd64724e26c5cefd5a42fb57565567ccdf31c7b` |
| S13 | [滚动示例](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/examples/scrollable.rs) | `20074e207ce78efec9e7c11a6d2532227ce57a8d3c6b18055b0504e1457fdcb1` |
| S14 | [最小GPUI应用示例](https://github.com/zed-industries/zed/blob/48741fd17d0dec28be6ad664b0a06dc7202086be/crates/gpui/examples/hello_world.rs) | `7e9d145d93470b02ce5641290ed7b444776813de580fe1d613b45443cc890aa0` |
| S15 | [Proto-UI官方实验性GPUI workspace说明](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/native/gpui/README.md) | `01e8d57b5cb9083c8e9e6a6a04bf6d5e0bd180b763a60d9c292a2fd54d0316ec` |
| S16 | [Proto-UI GPUI workspace与依赖版本](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/native/gpui/Cargo.toml) | `2737cd5239ba034972091132832246f5136c4f46ed9eb4e457f3ceac7e587156` |
| S17 | [Proto-UI GPUI Rust工具链](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/native/gpui/rust-toolchain.toml) | `e5cc0545f232bde807bd9394c2433322d1cd2cf42b060ceab2ac7329bc9cdaa2` |
| S18 | [Proto-UI GPUI实际Rust host实现](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/native/gpui/crates/proto-ui-gpui/src/host.rs) | `ec2b60aa6b0733e60170d297f206df56fb111e56eb6b0fa94c0d35f58f38bf66` |
| S19 | [Proto-UI GPUI原生输入路由](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/native/gpui/crates/proto-ui-gpui/src/input.rs) | `c916820bdb5bc2bf87f254504526ba8e08fe6b3aa0462315d4b003a978364144` |
| S20 | [Proto-UI原生session/epoch/lease模型](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/native/gpui/crates/proto-ui-host-protocol/src/model.rs) | `2cb55a939b57b70307210f2a735b957aa025674229bff353f1360d5b02da468f` |
| S21 | [Proto-UI原生style库](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/native/gpui/crates/proto-ui-style/src/lib.rs) | `c2b9aad04426b1032cd19f2685147b397843c90463cb21e39bdaf55ce4e52ec2` |
| S22 | [Proto-UI GPUI未发布仓库内TypeScript peer边界（源码公开）](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/packages/adapters/gpui-peer/README.md) | `f79740d657733b233cdad3f0fae98eb176ababc1557f051ea1826d175ac5e09f` |
| S23 | [Proto-UI GPUI peer依赖与private:true发布标记](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/packages/adapters/gpui-peer/package.json) | `00dc145260ed40104839342db05773cb90f73bcaa964d0b147789be6c1db2c68` |
| S24 | [Proto-UI GPUI有限AccessKit投影](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/native/gpui/crates/proto-ui-gpui/src/a11y.rs) | `396e5a573d3d73cb89fa0585da0fe70ba6270daf7fc3a550165167991367cd63` |
| S25 | [Proto-UI GPUI peer实际调用TypeScript Runtime的session实现](https://github.com/Proto-UI/Proto-UI/blob/35195a6e6f1ba6a7d800da0029f2683013abd18b/packages/adapters/gpui-peer/src/session.ts) | `0f1a69daef0d9f386821eb33e9f2b4c29efd9293b1fa7dc24ab7c246d2f0aea5` |

合成程序与请求格式随 Verdant 源码维护。数值复验使用对应报告记录的源码与原输入。[运行合同](../backend/RUNTIME.md) · [请求 schema](../backend/worker-request.schema.json) · [Rust worker](../frontend/crates/verdant-worker/src/main.rs)

所有快照的原始来源许可见 [licenses/README.md](licenses/README.md)。
