# Windows Agent 提交的字符与粘贴边界

## Status

Implemented；最终产品回归属于 S6 验收。

## Context

S6 在 Windows 真实 CLI 配合本地模拟模型检查时发现：Codex 原生控制台输入
路径丢失非 BMP emoji；Kimi 的 Runtime 普通文本加 Enter 留在编辑器中。
两者都会造成“接口已发送”却没有相应原生接受内容，不能靠预览显示掩盖。

## Evidence

- Codex 0.165.0 使用的 Windows 输入解析对 UTF-16 代理对有配对要求。
  ConPTY 的逐字符 key-down/key-up 序列会让代理对中间的 key-up 干扰配对。
  官方问题 [37578](https://github.com/openai/codex/issues/37578) 也记录同类症状。
- 本机原生记录确认普通 UTF-8 提交缺失 emoji；相邻代理单元的两个 key-down
  Win32 input record 保留完整中文和 emoji，且服务端原生用户条目确实接受。
- Kimi 2.3.3 在协商 bracketed paste 后，按粘贴结束边界等待回显再提交，
  原生主 Agent wire 中出现完整用户消息；直接突发文本加 Enter 的路径未接受。
- 实现和协议测试见 [terminal_input](../../../../nebula_app/src/input/terminal_input.rs)
  及 [Runtime 输入回归](../../../../nebula_app/src/gpui_shell/terminal/view/output_tests.rs)。

## Decision

Codex Runtime 提交、草稿及本地粘贴共享同一个编码函数；只在已识别 Codex
且启用 Win32 input mode 时为非 BMP 字符生成连续代理单元 key-down 记录。
VT 路径与其他 Shell 继续使用其原有协议。

Kimi 的 Runtime prompt 在 bracketed paste 已协商时，走现有粘贴及回显屏障，
再发送 Enter。草稿继续保留，空 prompt 仍按 API 合同拒绝；有意发送空 Enter
仍使用独立的按键 API。

## Rejected alternatives

- 删除 emoji、替换字符或预览自行补回：会改变用户提交正文。
- 所有终端统一模拟 Win32 键事件：破坏其他已协商的输入协议。
- 固定延时或连续重发 Enter：不能证明粘贴边界，可能重复提交。
- 将 Runtime 提交回执当作接受证据：只能证明写入接口已接受调用。

## Consequences

不新增依赖或配置。编码决策留在共享输入模块，视图只选择已识别 CLI 的
提交路径；实际消息接受及侧栏预览仍由各 CLI 的可靠原生来源证明。

## Validation

协议回归覆盖 BMP/非 BMP、CRLF、多行、Win32/VT、草稿和空输入。
实机探针检查 Codex、Kimi 的 Runtime prompt 与 paste，并核对原生接受记录。
最终四 CLI、共享 daemon 和便携包检查单独列入 S6 报告。

## Supersedes

None。

## Revisit when

Codex 的 Windows 输入解析改用新的原生协议，或 Kimi 改变粘贴和提交语义。
