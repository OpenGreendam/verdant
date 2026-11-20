# 官方来源核验快照

S01–S25文件为 [source-manifest.json](../source-manifest.json) 所列固定Git提交上的原始字节快照。文件名仅加来源编号，正文未改写。它们用于核验SPEC中涉及的上游事实，不是本项目新实现的前端或Compiler；其中S15–S25记录上游已经存在的原生GPUI Adapter工作区及其TS Runtime peer。

上游Markdown正文以 `.md.source.txt` 保存，避免把其中相对于上游仓库的链接误当成本包导航；原始路径、固定提交和canonical URL均在manifest中。文件名变化不改变SHA256核验的原始字节。

Proto-UI文件保留MIT许可来源，GPUI crate文件保留Apache 2.0许可来源，详见 [licenses/README.md](../licenses/README.md)。
