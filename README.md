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

## License

MIT
