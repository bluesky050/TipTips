# Task Normalization Implementation Plan

**Goal:** 统一新增邮件待办的存储、校验与展示，并兼容旧数据。

**Architecture:** 两个提取器输出 ExtractedItem，共享 task 模块规范化为 Schedule。PlannerItem 保留 text 并附加可选 schedule。前端 TaskContent 在列表和日历中共用。

**Tech Stack:** Rust / serde / chrono / regex；Vue 3 / TypeScript；不增加依赖。

## 约束与顺序

- 当前工作区已有用户改动，保留它们；不删除文件，不改凭据，不推送。
- 日期缺失时保留待办，日历不展示虚构日期。
- 验证使用合成邮件，不读取真实邮件内容。

## Task 1: 兼容模型与后端规范化

- [x] 先为规则解析增加回归测试：缺日期不填 +3 天、相对日期按邮件日期、截止与时间段、多个事件。
- [x] 运行 `cmd /c build.bat test --lib --offline`，确认缺失行为导致断言失败。
- [x] 新增 `src-tauri/src/task.rs`：Schedule、时间校验、标题构造、邮件依据、明确事件身份比较；修改 data.rs 添加可选 schedule，修改所有构造点。
- [x] 修改 ai.rs 扩展提取字段和邮件日期基准；规则多事件解析接入同一入口。
- [x] 修改 email_scanner.rs：纯函数创建规范化待办与事件去重，不按 UID 一概拦截；测试不同场次、多个事项、跨邮件提醒和旧数据。
- [x] 再次运行 Rust 测试，修正失败。

## Task 2: 统一展示与人工编辑兼容

- [x] 修改 src/types.ts；新增 src/components/TaskContent.vue，由 PlannerItem.vue 和 CalendarView.vue 共用。
- [x] 修改两个 composable，更新后刷新服务端数据，使结构化信息与手工标题一致。
- [x] 运行 `npm run build`，确认类型检查及 Vite 构建通过。

## Task 3: 发布和最终验证

- [x] 运行后端全部测试并自审新改动，重点检查去重、时间歧义、旧 JSON。
- [x] 运行发布构建；在新路径保存新版可执行文件，保留现有正在运行的版本。
- [x] 记录测试数量、构建结果与未覆盖的实际联网扫描/界面限制。


## 验证记录

- 先观察到 4 个时间解析断言失败，再修复通过。
- 独立审查发现并复现 5 个行为缺陷：多事项字段串场、无日期活动误继承日期/被遗漏、非法时间截断、缺链接提醒重复。
- 复审追加邀请说明误拆分、主题字段丢失两项回归，均先失败再通过。
- 最终 Rust 单元测试：43 passed / 0 failed。
- 实际 TaskContent.vue 编译后 SSR 渲染测试：6 passed / 0 failed。
- npm run build：Vue TypeScript 检查及 Vite 发布构建通过。
- 未调用真实邮箱或 Gemini API；实际邮件质量尚需在使用中观察。
- 改动保持在 codex/task-normalization 本地分支，保留原有未提交工作。


- 最终 Windows 发布构建：通过；输出 D:\TipTips\TipTips-normalized.exe，与 target/release/tauri-app.exe SHA256 相同。
- 45 秒隔离启动 smoke test：PASS，主窗口标题 TipTips、响应正常；仅使用临时合成数据与独立 WebView 缓存，测试结束仅停止自己的实例。
- 尚未验证实际邮箱联网扫描、Gemini 服务响应或人工操作完整流程；已验证纯解析/去重、真实 Vue 组件输出、发布构建和持续启动。

## 重复与长文本修复（第二轮）

- 根因：旧版无 schedule 的记录未参与去重；HTML 段落被压成一行，备注关键词使整封正文被提取；招聘流程关键词被当作实际邀请；工作日范围被当作日历日期。
- 修复：保留 HTML 段落，关联相邻日期/纯时间行；备注按句提取并保留完整分号材料清单；收紧招聘广告候选过滤；排除数字持续时间误读。
- 卡片标题最多预览 60 字、备注 80 字、地点 50 字，完整文本可展开，原始内容保留。
- 对确定重复的历史记录仅合并显示，保留磁盘记录与原文；同组切换状态同步，用户主动删除时按同组执行，编辑标题不解除分组。
- 分组检查组内每条记录的已知字段，避免缺链接/地点桥接有冲突的场次；类型或状态冲突保留独立显示。
- 先运行失败回归，再修复通过。第二轮最终 Rust 51 passed，Vue SSR 7 passed，npm run build 通过。
- 真实数据只读预演：41 条记录中有 12 组类型和状态一致的新旧配对；未修改用户数据，未调用邮箱/Gemini 服务。
- 发布构建通过（2m20s）；D:\TipTips\TipTips.exe、TipTips-normalized.exe、TipTips-refined.exe 均为修复版，SHA256 为 423C7015316484FA54E8D50F8FF652DF8CA4E8EF5589A1B171FC3CA598E8E87A。
- 原程序备份：TipTips-original-20261011.exe；上一版备份：TipTips-normalized-v1.exe。未删除文件。
- 首次隔离启动被正在运行的旧实例拦截（退出码0）；停止旧实例后45秒隔离启动验证 PASS、窗口TipTips响应正常。
