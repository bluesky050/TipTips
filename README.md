# TipTips

A lightweight, always-on-top desktop daily planner widget built with **Tauri 2** + **Vue 3** + **TypeScript** + **Rust**.

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![Tauri](https://img.shields.io/badge/Tauri-2-green)
![Vue](https://img.shields.io/badge/Vue-3-brightgreen)
![Rust](https://img.shields.io/badge/Rust-1-orange)

## Features

- **Always-on-top floating widget** — stays visible while you work, no need to switch windows
- **Daily task management** — add, complete, reorder, and delete tasks with one click
- **Color-coded status** — undone items show light red background, completed items show light green
- **Auto day rollover** — unfinished tasks automatically carry over to the new day
- **Calendar view** — switch between list view and calendar view to see tasks across dates
- **Adjustable transparency** — set window opacity from the settings panel to keep it unobtrusive
- **Custom title bar** — frameless window with drag support, minimize/close buttons
- **System tray** — minimize to tray, double-click tray icon to restore
- **Window position memory** — remembers window size and position across sessions
- **Data persistence** — tasks stored locally via Rust backend (JSON file), no cloud required

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | Vue 3 + TypeScript + Vite |
| Backend | Rust + Tauri 2 |
| Data Storage | Local JSON file (via Rust `serde_json`) |
| Date Handling | `chrono` (Rust) |

## Project Structure

```
TipTips/
├── src/                        # Vue 3 frontend
│   ├── App.vue                 # Root component (opacity, view switching)
│   ├── main.ts                 # Entry point
│   ├── types.ts                # TypeScript interfaces
│   ├── components/
│   │   ├── TitleBar.vue        # Custom draggable title bar
│   │   ├── PlannerInput.vue    # Task input field
│   │   ├── PlannerItem.vue     # Single task item (toggle, reorder, delete)
│   │   ├── CalendarView.vue    # Calendar date grid with task counts
│   │   ├── ViewSwitcher.vue    # List/Calendar view toggle
│   │   └── SettingsPanel.vue   # Opacity and settings panel
│   └── composables/
│       ├── usePlanner.ts       # Task CRUD logic (invoke Tauri commands)
│       ├── useCalendar.ts      # Calendar navigation helpers
│       └── useDataSync.ts      # Data synchronization utilities
├── src-tauri/                  # Rust backend
│   ├── Cargo.toml              # Rust dependencies
│   ├── tauri.conf.json         # Tauri window & bundle config
│   └── src/
│       ├── main.rs             # App entry
│       ├── lib.rs              # Tauri setup (window restore, tray icon)
│       ├── commands.rs         # Tauri commands (CRUD, reorder, config)
│       ├── data.rs             # JSON data persistence layer
│       └── config.rs           # Window config read/write
├── public/                     # Static assets
├── index.html                  # HTML shell
├── package.json                # Node dependencies & scripts
├── vite.config.ts              # Vite + Tauri plugin config
└── tsconfig.json               # TypeScript config
```

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (Windows)

### Install Dependencies

```bash
# Frontend
npm install

# Rust (handled automatically by Tauri CLI on first build)
```

### Development

```bash
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
```

The output installer will be in `src-tauri/target/release/bundle/`.

## How It Works

1. The Vue 3 frontend renders a compact, frameless always-on-top window
2. All task data operations (add, toggle, delete, reorder, date rollover) are delegated to Rust via Tauri's `invoke` IPC
3. Rust persists tasks to a local JSON file using `serde_json` + `chrono`
4. Window position and size are saved/loaded via a separate config file
5. The system tray icon allows hiding/showing the window and quitting the app

## 待办格式规范

新扫描的邮件事项保存结构化信息，并统一显示为：

```text
[面试] 示例公司 · 后端开发 · 二面
2026-10-15 14:00 · 开始 · UTC+08:00
地点：腾讯会议
备注：提前10分钟进入
```

- 时间明确区分开始、截止、可参加时间段；相对日期按邮件发送日期计算。
- 缺失或存在歧义的信息标记“待确认”，不自动编造日期；没有日期的事项留在列表，不显示为确定的日历事件。
- 列表与日历使用同一个展示组件，地点、活动链接、备注和邮件依据分别显示。
- 同一封邮件可以包含多个活动；不同时间、岗位和轮次分别保留。身份明确的重复提醒不会覆盖原事项及人工修改。
- 历史 JSON、手动待办、原有文字编辑与完成状态兼容；历史邮件不自动重新扫描。明确重复的新旧事项与重复提醒合并显示，原始记录仍保存在文件中，关联原文可展开查看；完成状态不同或身份冲突的事项分别显示。
- 卡片预览标题最多 60 字、备注 80 字、地点 50 字，长内容展开查看；邮件段落与材料清单保留，招聘宣传中的流程描述不直接生成待办。
- 合并显示的事项按同组切换完成状态；编辑标题仍保持分组。用户主动删除该事项时，同组的重复记录一并删除。

验证命令：`npm test`、`npm run build`、`cmd /c build.bat test --lib --offline`。

## 官网宣讲会日程

界面以红色区分普通待办、浅黄色区分宣讲会，随系统适配浅色/深色模式；标题、时间和来源分层排版。小窗口中操作按钮置于正文下方，日历宣讲数量允许换行，月份可滚动查看。混合日期保留未完成待办的红色背景，宣讲数量使用黄色标识。

- 宣讲会以东南大学就业网公开预报为自动来源，按北京时间显示今天及未来活动。日历格显示宣讲数量，点击日期可查看完整标题、起止时间、校区地点及官网详情。
- 启动自动同步，顶部“刷新”可再次同步；完整读取所有分页成功后更新独立缓存 `%APPDATA%/TipTips/seu_fairs.json`。网络失败保留上次成功快照并显示提示。
- 官网日程只读，不参与个人待办完成进度或跨日延续。北京时间跨日时两个视图重读缓存，过去官网活动不继续展示。
- 邮件中的宣讲会隐藏并停止新增，历史邮件记录保留；手动事项以及同邮件其他面试、笔试、测评事项保留。

设计与验收标准见 `docs/superpowers/specs/2026-10-11-task-normalization-design.md`。

## License

MIT
