# DeepSeek Harness 接口来源

DeepSeek Harness、腾讯 QQ 和 dsh-im 的固定上游接口与适用条件。安装与命令见 [CLI／DSH](../../spec/CLI-DSH.md)。

## 审阅身份

| 对象 | 固定提交 | 源码版本 |
| --- | --- | --- |
| DeepSeek Harness | `2e39e70496aa02986ca6a820c5d497e816b69d7e` | `0.2.2-rc.5` |
| 腾讯 dsh-qqbot | `505b64e95f07b276a148034df9f854c78936f203` | `0.5.5` |
| xmanrui/dsh-im | `fc2ceab217309f8973afa03c58cdd8ff3e900130` | `5.10.0` |

这些是 2026-11-20 源码审阅的版本，未运行宿主或真实 IM 账号。审阅时 Verdant 实现基线为 `0d5b870f182cef3b309b2a88ac2bbfa8203c8e41`。源码及发行使用 [OpenGreendam/verdant](https://github.com/OpenGreendam/verdant)。

## 宿主合同

插件使用 Cordis `apply(ctx)`、依赖注入及 `ctx.tools.register(defineTool(...))`。`execute` 返回经 schema 校验的 JSON，`output.render` 负责可读呈现。音乐操作和音频算法由 Rust CLI 执行；TypeScript 负责宿主、进程、附件与投递映射。

以参数数组和 stdin JSON 启动明确的 CLI，不拼接 shell。公开协议、实际 capability 及输入／输出制品分别校验。歌词或歌曲名不代表已有作曲、扒谱或声音转换能力。

固定版本 `JobSpec.owner` 为会话 `SessionId`；cookbook 的旧 `exec.agent` 示例不作为类型依据。长任务发布后使用 job 自身的取消信号，完成须等子进程与资源清理。宿主内存 job 不表示跨重启持久队列；同群共享会话也不提供群成员隔离。

命令附件须声明 `input.attachments`。`ctx.commands.register` 可为 Web／Desktop 提供入口，腾讯 QQ 的固定命令表不会自动发现新增命令；斜杠入口单独验收。

## 通道合同

| 通道 | 固定源码行为 | 完成判断 |
| --- | --- | --- |
| 腾讯 dsh-qqbot 0.5.5 | `qqbot_send_file(file_path)` 按扩展名将 WAV／MP3／OGG 等交给 `sendVoice`，工程交给 `sendFile`；入站下载跳过 voice／audio。 | 发送调用结果与消息 ID按真实通道记录；源码分支不证明实际接收播放。 |
| dsh-im 5.10.0 | `dsh_im_return_file(path)` 登记制品，需当前 turn 的 active delivery consumer；QQ 使用普通 `sendFile`。 | 返回 queued；后台完成不会自动继承原 turn 的 consumer。 |
| Web／Desktop | 返回受管本地产物。 | 可领取文件；IM 投递状态单独记录。 |

合成与投递各有状态。固定原会话、请求人、任务和制品绑定后再执行；超时无确定发送收据时记 unknown，不盲目重复发送。后台无法恢复投递上下文时保留可领取产物。真实平台权限、体积、时长及播放限制按已选通道实测。

## 安装与兼容

DSH bundle 使用 `dsh.bundle.patch`，profile 为运行组合；插件加入用户选择的 profile。预构建包不得依赖仓库外兄弟目录或调用方本地 Rust／TypeScript 编译器。包名及安装地址以真实发行元数据为准。

固定 DSH 快照要求 Node.js `>=24.9.0`；这是宿主条件。`engines.dsh` 及通道上游的兼容声明不能替代 Verdant 的实际版本矩阵。插件、CLI、worker、声库与通道分别记录身份。三项上游代码采用 MIT，Verdant 源码许可与声库分发边界见[归属](../../NOTICE.md)和[发行政策](../../RELEASE.md)。

## 固定一手来源

- [DSH package.json](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/package.json)：版本、Node 条件及许可。
- [工具编写参考](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/docs/cookbook/adding-a-tool.md)、[JobSpec](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/packages/jobs/jobs/src/types.ts)、[一方 workflow](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/packages/workflow/tool-workflow/src/index.ts)：工具结果、owner 与任务生命周期。
- [command 类型](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/packages/interaction/commands/src/types.ts)：命令及附件准入。
- [配置](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/docs/user/develop/basic/config.md)、[发布](https://github.com/deepseek-ai/deepseek-harness/blob/2e39e70496aa02986ca6a820c5d497e816b69d7e/docs/user/develop/basic/publish.md)：schema、bundle、profile 与版本检查。
- [腾讯 package.json](https://github.com/tencent-connect/dsh-qqbot/blob/505b64e95f07b276a148034df9f854c78936f203/package.json)、[媒体分发](https://github.com/tencent-connect/dsh-qqbot/blob/505b64e95f07b276a148034df9f854c78936f203/src/media/send-media.ts)、[文件工具](https://github.com/tencent-connect/dsh-qqbot/blob/505b64e95f07b276a148034df9f854c78936f203/src/media/send-file-tool.ts)：语音／文件分流与返回值。
- [腾讯入站附件](https://github.com/tencent-connect/dsh-qqbot/blob/505b64e95f07b276a148034df9f854c78936f203/src/transport/attachment.ts)、[命令表](https://github.com/tencent-connect/dsh-qqbot/blob/505b64e95f07b276a148034df9f854c78936f203/src/commands/index.ts)：音频跳过和命令接线范围。
- [dsh-im package.json](https://github.com/xmanrui/dsh-im/blob/fc2ceab217309f8973afa03c58cdd8ff3e900130/package.json)、[文件登记](https://github.com/xmanrui/dsh-im/blob/fc2ceab217309f8973afa03c58cdd8ff3e900130/src/channels/shared/semantic/artifact.mjs)、[交付](https://github.com/xmanrui/dsh-im/blob/fc2ceab217309f8973afa03c58cdd8ff3e900130/src/channels/shared/semantic/artifact-delivery.mjs)、[QQ 适配](https://github.com/xmanrui/dsh-im/blob/fc2ceab217309f8973afa03c58cdd8ff3e900130/src/channels/qq/qq-bridge.mjs)：turn consumer、queued 与发送范围。

这些链接固定源码事实；真实宿主、平台账号及跨机工程接续按[规格验收矩阵](../../spec/CLI-DSH.md)另行记录。
