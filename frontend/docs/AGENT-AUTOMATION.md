# Verdant 命令入口与 Agent 自动化

## 入口

`verdant` 是不依赖 GPUI 的命令行程序。无参数、`--help` 和 `--runtime-features` 都在无显示服务的环境输出信息并退出。原有 `verdant-cli` 保留为同一 CLI 的兼容入口；JSON 协议仍为 `verdant-cli/1`。

`verdant gui` 才启动图形界面。它从当前程序所在的软件包解析 `verdant-gui`，保留调用方工作目录和原始路径参数；不通过 shell 拼接命令。默认启动后返回 PID 和日志路径，GUI 输出不占用 Agent 的 stdout/stderr 管道。`--wait` 明确要求等待 GUI 关闭，`--check` 只解析路径而不启动。

```sh
verdant
verdant capabilities --json
verdant schema plugin.set --json
verdant gui --check --json
verdant gui "工程 名.verp"
verdant gui --wait -- --import "乐谱 名.ufdata"
```

GUI 启动选项必须写在桌面参数之前；`--` 分隔启动器选项与桌面参数。仅在诊断或开发时使用 `--gui-binary PATH` 或 `VERDANT_GUI_BINARY` 指定不同 GUI。正常安装不需要依赖 PATH 中的另一份 GUI。

解包后的根目录直接提供 `verdant-launcher.exe`（Windows）或可执行的 `verdant-launcher`（Unix）。它打开本包 GUI 并监督退出，启动/异常退出失败通过本机错误对话框和日志报告。Windows GUI 和根启动器使用 GUI 子系统，不额外弹出控制台。Linux 桌面入口执行 `verdant gui %f`，macOS 的 `Verdant.app` 使用 `verdant-gui`。

| 平台 | CLI | GUI 实体 | 根启动器 |
| --- | --- | --- | --- |
| Linux | `bin/verdant` | `libexec/verdant/verdant-gui` | `verdant-launcher` |
| Windows | `bin/verdant.exe` | `bin/verdant-gui.exe` | `verdant-launcher.exe` |
| macOS | `Verdant.app/Contents/MacOS/verdant` | `Verdant.app/Contents/MacOS/verdant-gui` | `verdant-launcher` |

日志默认位于 Windows 的 `LOCALAPPDATA/Verdant/logs`、macOS 的 `~/Library/Logs/Verdant`，以及 Unix 的 `XDG_STATE_HOME/verdant/logs`（默认 `~/.local/state/verdant/logs`）。可通过 `VERDANT_LOG_DIR` 改到隔离测试目录。根启动器使用可移植的标准进程接口；BSD 的桌面运行仍须在目标机验证。

## 原子操作与工作流 worker

`capabilities` 列出当前二进制的完整操作集合。`schema OPERATION` 返回字段类型、必填项和嵌套结构。插件、混音、预设、切片和分析操作使用 `plugin.*`、`mixer.*`、`preset.*`、`clip.*`、`analysis.*` 命名，与工程、历史、素材、渲染和任务操作共用协议。

```sh
verdant plugin list --json
verdant preset list --json
verdant schema clip.split --json
verdant schema mixer.set --json
verdant request --json < request.json
verdant worker --stdio
```

`worker --stdio` 是显式启动的 CLI 工作流执行器，兼容别名为 `serve --stdio`。它读取每行一个 JSON 请求，严格顺序执行，一次最多一个活动请求，并输出带 request_id/sequence 的 progress/result 事件。空输入并关闭 stdin 会退出。新原子操作自动使用同一 dispatch，不另建不一致的 worker 操作面。

独立的 `verdant-worker` 是声库合成进程，继续使用其原有声库协议。它不代替 `verdant worker --stdio`。

```json
{"protocol":"verdant-cli/1","request_id":"devices-1","op":"plugin.list","params":{}}
{"protocol":"verdant-cli/1","request_id":"presets-1","op":"preset.list","params":{}}
```

修改工程前读取当前 revision，并按对应 schema 提供 `expect_revision`。需要预览的修改使用 `dry_run`；复杂参数可用 `--params FILE` 或单个 JSON request，避免 shell 转义。错误返回结构化 code/message，不能仅凭进程成功启动判定音频或工程修改已经完成。具体操作与持久化语义见 [CLI／DSH](../../spec/CLI-DSH.md)。

## 安装随包 Agent Skills

技能完全随应用分发，安装无需下载。每个技能使用标准 `<name>/SKILL.md`，并附带清单列出的 Agent 元数据和操作参考。总入口技能名为 `verdant`，每个原子技能独立描述实际操作、前置条件和完成检查。

```sh
verdant skill list --json
verdant skill show verdant --json
verdant skill install verdant --agent generic --scope project
verdant skill install --all --agent codex --scope project --project-root /path/to/project
verdant skill install --all --agent claude --scope user
verdant skill install --all --dest "/isolated/project/.agents/skills" --dry-run
verdant skill install --all --dest "/isolated/project/.agents/skills"
verdant skill uninstall --all --dest "/isolated/project/.agents/skills"
```

默认目标是当前工程的 `.agents/skills`。可选 `generic`、`codex` 和 `claude` 分别使用 `.agents/skills`、`.codex/skills` 和 `.claude/skills`。`--scope project` 使用当前目录或 `--project-root`；`--scope user` 使用本账户 home，`global` 是 user 的兼容拼写。显式 `--dest` 已是 skills 目录，不能再与 agent/scope/project-root 混用。

`list` 默认列全部，也接受单个 name；`show`、`install`、`uninstall` 必须选择 name 或 `--all`。list/show/install 可用 `--bundle DIR` 或 `VERDANT_SKILLS_DIR` 指向完整离线 bundle。卸载只依赖目标目录的托管收据，即使旧技能已从新版本 bundle 移除也能卸载。

安装器逐项核对 schema、语义版本、文件清单和原始 SHA-256；路径必须是可移植相对路径，拒绝目录穿越、符号链接和 Windows 保留路径。完整选择集合预检后，才在同目录暂存并原子替换；失败回滚保留原安装或给出恢复目录。每个技能的 `.verdant-skill.json` 仅跟踪 Verdant 管理的文件。

重复安装同版本同内容返回 `unchanged`。更新前必须核对旧托管文件仍与收据一致；已有未托管目录、用户修改、额外文件、缺失文件和链接都会拒绝覆盖。卸载也执行同样检查。没有强制覆盖选项。需要合并本地定制时，先把定制目录备份到 skills 目录之外，再安装新版本并人工比较合并；也可先安装到新的隔离目录进行比较。`--dry-run` 验证和返回计划，完全不写目标目录。

## 发行验证

原生包使用 `verdant-native-package/3` 清单，声明全部八个原生程序和单独的 Agent Skills 清单摘要。组装阶段从受选 Git 源码复制技能，不使用网络或当前个人技能目录。

打包器在包含空格和 Unicode 的暂存路径、不同工作目录中验证：

- 两个 CLI 无参数输出帮助并及时退出，默认不会启动 GUI。
- `gui --check` 和根启动器 `--check` 解析同包 GUI。
- 全套离线技能可列出、安装、幂等重装及卸载，所有写入只在临时目录。
- 所有程序具有可执行权限，Windows GUI/console 子系统与角色一致。
- 每个归档成员、技能摘要、受选源码与原生库符合包清单。

跨平台原生运行和设备验收按 [平台验收](PLATFORM-ACCEPTANCE.md)分别记录；软件包结构验证不替代实际音频设备和桌面行为观察。

## 预检、素材目录与导出范围

三个目录字段有不同用途：

| 字段 | 用途 |
| --- | --- |
| `asset_roots` | 明确允许读取的外部音频素材目录。绝对路径素材在预检与导出时都必须获得授权；不会因为某目录是工作目录而自动授权。 |
| `project_root` | 声库合成 worker 使用的工程资源目录，不代替外部音频授权。 |
| `workspace` | CLI 任务收据、渲染缓存和插件缓存的位置，不改变输入素材的读取范围。 |

对同一导出任务，预检与实际执行应复用参数。以下 Python 片段先验证，再生成文件；项目必须已存在，修订号、时间范围和素材目录须由 inspect 与用户任务确定：

```python
common = {
    "path": project_path,
    "expect_revision": revision,
    "asset_roots": [authorized_audio_directory],
    "workspace": render_workspace,
    "sample_rate": 96000,
    "channels": 2,
    "effects": True,
    "scope": "Mix",
    "start_tick": selected_start,
    "end_tick": selected_end,
}
plan = request("render.plan", common)
result = request("export.audio", {
    **common,
    "output": fresh_wav_path,
    "report": fresh_report_path,
})
```

`render.plan` 使用与导出相同的素材授权与指纹核对，并返回素材、效果器和宿主摘要。`duration_seconds` 表示输出的时间范围；`processing_timeline_seconds` 包含从工程零点推进效果历史所需的时间。例如请求第 1–2 秒，输出长 1 秒，处理时间线长 2 秒。指定范围后不包含范围末端以后的尾音；完整导出按轨道与主总线的有限尾音策略生成文件。显式区间在报告中标为 `audio.range="Selection"`，具体边界是 `range_ticks`。

`dry_run` 返回预览的工程、revision、commit 和可能新建的 ID；这些值未持久化。实际提交继续使用原先检查到的 revision，并以实际提交返回的 ID 为后续操作依据。一次预览生成的 clip ID 不能作为已经保存的片段使用。

## 只读拟合与实际参数应用

`analysis.eq-fit` 接受 Hz／dB 目标点；`analysis.eq-match` 从指定源文件与参考文件提取平均立体声功率谱；`analysis.reverb-ir` 从真实冲激响应估计混响衰减与频谱参数。它们均返回诊断报告与 `values` 参数映射，不改动工程。检查拟合误差、频率范围及源文件指纹后，将 `values` 交给对应实例的 `plugin.set`。这些操作同样可通过 workflow worker 和独立原子 Skill 使用。
