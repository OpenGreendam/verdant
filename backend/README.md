# verdant-worker(1) — 合成协议服务

## 用法

```text
verdant-worker [--workspace DIR] [--project-root DIR] [--voice-registry FILE]
    [--voice-package DIR] [--onnxruntime-library FILE]
verdant-worker --voice-registry FILE --set-default-voice REF [选项]
verdant-worker --voice-registry FILE --upgrade-default-voice REF [选项]
verdant-worker --help
verdant-worker --runtime-features
```

## 说明

Rust worker 在 stdin／stdout 上交换 UTF-8 NDJSON，协议为 `ogd-worker/1`。参数生成、DSP、ONNX 和制品写入在受监督的 Rust／原生进程中执行。客户端负责工程、调度、缓存、播放及最终混音导出。

默认值、平台目录、来源选择及原生库见[运行配置](RUNTIME.md)。本地声库准备见[安装手册](../docs/verdant-voice.md)。合成不要求网络、GPU 或音频设备。

## 请求与能力

worker 启动先发 `ready`。客户端查询实际能力后才提交任务；未知 capability 视为不支持。`ready` 表示服务可收请求，不表示模型已可合成。

```json
{"protocol":"ogd-worker/1","request_id":"cap-1","op":"capabilities"}
```

每行最多 8 MiB。stdout 只写协议；诊断使用 stderr。请求字段及条件约束见 [worker-request.schema.json](worker-request.schema.json)，worker 另做跨字段与资产校验。公开 CLI 的 `verdant-cli/1` 另有[规格](../spec/CLI-DSH.md)，不能将两种 envelope 混用。

| route | 输入 |
| --- | --- |
| `verdant-voice` | 与软件同版本的已准入声库、`native_score`；原生 48 kHz。 |
| `C0` | 双摘要验证并登记的原版声库、`native_score`。 |

缺文件、哈希不符、来源未准入和能力不足均拒绝。声库包和原版声库的选择规则见 [RUNTIME.md](RUNTIME.md#来源与准入)。

## 生命周期

| 事件 | 客户端含义 |
| --- | --- |
| `ready` | 服务已启动。 |
| `capabilities` | 实际能力、资源状态、并发与指纹。 |
| `voice_admitted` | 声库完整验证并原子持久化。 |
| `accepted` | 已接收任务，固定 job ID 与 revision。 |
| `progress` | 真实阶段边界；未知比例为 `null`。 |
| `completed` | 已完成并绑定完整产物、相对路径、字节数与 SHA-256。 |
| `failed`／`cancelled` | 任务失败或取消；部分产物无效。 |
| `rejected` | 请求未接收，例如参数错误、重复 ID 或 `BUSY`。 |
| `cancel_ack` | 取消请求已处理；任务仍以其终态为准。 |
| `shutdown` | 活动作业清理完成。 |

并发上限读取 `capabilities.max_active_jobs`。当前基线在可用并行度至少为四时上限为 2，否则为 1；客户端还须施加自身资源预算。每个接收任务只有一个终态，job ID 不复用，失败或取消目录也不覆盖。

```json
{"protocol":"ogd-worker/1","request_id":"cancel-1","op":"cancel","job_id":"phrase_001"}
```

取消可能与成功完成竞争；验证完成的结果可以仍为 `completed`。用 job ID、revision 和内容摘要决定是否采用，迟到结果不能覆盖新编辑。EOF、`shutdown` 及受处理的中断会收束活动任务；进程所有权和升级终止规则见[运行配置](RUNTIME.md#任务与产物)。

## 文件与结果

任务位于 `workspace/jobs/<job_id>/`，保留请求、有效 score／控制轨、实际命令、日志、终态及 WAV／NPZ／JSON。文件存在不足以采用结果；客户端还须验证完整产物图、边界、字节数、摘要和解码内容。

历史协议夹具保留在 [validation-final](validation-final/verification.json)及其[记录](validation-final/protocol-transcript.json)，供原有消费者使用。其 Python 环境、模型和版本范围保持原义；当前源码与平台验收见[平台验收](../frontend/docs/PLATFORM-ACCEPTANCE.md)。

## 退出状态

`--help`、`--runtime-features`、正常 EOF／shutdown，以及成功的一次性操作退出 0；启动、参数解析或一次性处理失败退出 1。服务内的任务失败和请求拒绝通过 NDJSON 报告，服务仍可继续，因此进程退出码不能代替任务终态。

## 参见

[运行手册](RUNTIME.md) · [桌面编辑器](../frontend/README.md) · [合成来源与验证](../SYNTHESIS-FIDELITY.md)
