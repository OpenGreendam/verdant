# verdant-voice(1) — 本地声库包

## 用法

```text
verdant-voice install ZIP_OR_DIR [--registry FILE] [--set-default]
    [--artifact-policy FILE --artifact-policy-sha256 REVIEWED_SHA256]
verdant-voice list [--registry FILE]
verdant-voice remove REFERENCE [--registry FILE]
verdant-voice set-default REFERENCE [--registry FILE]
verdant-voice import-originals --token FILE --model FILE [--registry FILE]
```

## 安装包

从包内 `bin/verdant-voice` 运行。Windows 使用 `bin\verdant-voice.exe`，macOS 使用与 `Verdant.app` 同级的 `verdant-voice`。

```sh
verdant-voice install /path/to/voice.zip --set-default
```

ZIP 或目录的根应包含 `manifest.json`、`bank.safetensors`、三个 ONNX 图、音素表及清单列出的来源记录。合并 bank 保存运行时需要的 token 数据。安装器校验版本、清单成员和摘要，按许可策略核对外部授权记录，然后安装并更新注册表。普通安装使用本地声库包。输出分别记录注册结果和是否具备渲染条件。

包安装到用户 data 目录的 `voices/` 下。注册表依次使用 `--registry`、`VERDANT_VOICE_REGISTRY` 或用户 state 目录下的 `voices.json`。`VERDANT_STATE_DIR` 和 `VERDANT_USER_DATA_DIR` 可覆盖平台默认目录。[目录规则](../backend/RUNTIME.md#文件与目录)

`set-default` 切换后续使用的默认声库。`remove` 取消注册并在必要时清除默认选择。声库文件与已有工程引用保留。

## 解锁原版

```sh
verdant-voice import-originals --token /path/to/sn16k.tkn --model /path/to/sn16k.bin
```

两个文件的 SHA-256 均须匹配记录的固定摘要。命令将通过校验的原始文件登记为 `verdant-original-sn16k`，保留现有默认声库。渲染原版歌手时仍需能够读取这两个文件。

## 开发转换

```sh
verdant-voice convert --source DIR --destination DIR \
  --source-token FILE --source-model FILE
```

转换使用同版本旧包与匹配的本地原始文件，写入包含合并 bank 的新目录。工具在写入前比对旧 bank 和原始 token，写入后重新读取并验证包内容。分发条件见 [RELEASE.md](../RELEASE.md)。
