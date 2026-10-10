# 中英释义表

程序员模式的数据：候选词右侧那一行英文小字。点按 `~`（·）进入程序员模式后，按数字键 `1`-`9` 或空格就能把当前候选的英文直接上屏，
适合写代码、查英文说法的场景，不用切到别的输入法。这张表只在本机查表，不联网。

格式：`词\t[词性. ]释义[|读音]\t…`，UTF-8 **无 BOM**、LF 换行、按码点排序。
解析在 `crates/lightbookinput-dictionary/src/english_glossary.rs`，规则是严格的：任一行缺词或缺释义，
整张表加载失败（`.qj` 打包走同一套解析）。

本目录只有 `glossary-en.tsv` 一张表，随代码以 **GPL-3.0-or-later** 发布，与仓库一致。

由 LLM（DeepSeek）离线批量生成，不含任何第三方词典内容。2026-09-05 对 `assets/lexicon/dict.tsv` 全量生成初版，
2026-10-10 又并入 `developer-terms.tsv` 的 279 条轻书开发者术语；当前运行时表 232,388 条，词库多字词 91% 有英文释义。
要重跑就照原提示词再来一轮，模型原始输出的 JSONL 只是续跑用的中间产物，放在 `data/generated/`、不进 git，随生成时间在 Release 附件留档。

`developer-terms.tsv` 是人工维护的覆盖层，覆盖程序设计、并发、数据库、网络、安全、架构、测试与 AI 等专业词；
它先独立校对，再按词面合并进 `glossary-en.tsv`，同名词以覆盖层为准。`crates/lightbookinput-dictionary/tests/developer_data.rs`
会同时检查覆盖层与运行时表，防止后续重跑数据管道时把补充术语丢掉。

没有产品数据的环境（`--sample`、开发期）用 `assets/sample/glossary-en.tsv`，格式与这张一致。
