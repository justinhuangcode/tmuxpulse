# TmuxPulse

[English](./README.md) | **中文**

[![CI](https://github.com/justinhuangcode/tmuxpulse/actions/workflows/ci.yml/badge.svg)](https://github.com/justinhuangcode/tmuxpulse/actions/workflows/ci.yml)
[![Release](https://github.com/justinhuangcode/tmuxpulse/actions/workflows/release.yml/badge.svg)](https://github.com/justinhuangcode/tmuxpulse/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=flat-square)](https://github.com/justinhuangcode/tmuxpulse)
[![GitHub Stars](https://img.shields.io/github/stars/justinhuangcode/tmuxpulse?style=flat-square&logo=github)](https://github.com/justinhuangcode/tmuxpulse/stargazers)
[![Last Commit](https://img.shields.io/github/last-commit/justinhuangcode/tmuxpulse?style=flat-square)](https://github.com/justinhuangcode/tmuxpulse/commits/main)
[![Issues](https://img.shields.io/github/issues/justinhuangcode/tmuxpulse?style=flat-square)](https://github.com/justinhuangcode/tmuxpulse/issues)

实时事件驱动的 tmux 终端界面工具，支持会话监控、跨面板搜索与闲置清理。 💓

一目了然地查看所有会话，使用键盘或鼠标导航，跨面板搜索，管理闲置会话 -- 全部在一个终端窗口内完成。

## tmux 术语速查

tmux 把工作组织为三级结构，理解这个层级关系有助于阅读后续文档：

```
Session 会话（独立工作区，如 "backend"、"frontend"）
  └── Window 窗口（会话内的标签页，Ctrl-b n 切换）
        └── Pane 面板（窗口内的分屏，如左边跑服务、右边看日志）
```

| 概念 | 类比 | 示例 |
|---|---|---|
| **Session 会话** | 一个虚拟桌面 | `tmux new -s backend` 创建名为 "backend" 的会话 |
| **Window 窗口** | 虚拟桌面里的标签页 | 一个窗口用于 `vim`，另一个用于 `git` |
| **Pane 面板** | 标签页里的分屏 | 左侧面板跑服务器，右侧面板看日志 |

一个开发者可能有 10 个会话，每个会话 2-3 个窗口，每个窗口 1-2 个面板。这意味着 20-60 个终端视口 -- 而 `tmux ls` 只能显示会话名称。

## 为什么选择 tmuxpulse?

tmux 重度用户通常同时运行数十个会话。想要了解各个会话的状态，你需要手动循环执行：`tmux ls`（列出会话）→ `tmux attach -t <名称>`（进入某个会话）→ 查看内容 → `Ctrl-b d`（退出）→ 对下一个会话重复以上步骤。每次切换都会打断你的工作流。

tmuxpulse 提供**一个实时仪表盘，同时展示所有会话、窗口和面板的状态** -- 无需离开当前终端。

现有工具无法满足这个工作流：

| | tmuxpulse | tmuxwatch | htop/btop | tmux 内置 |
|---|---|---|---|---|
| 专为 tmux 监控设计 | 是 | 是 | 否（进程监控） | 否（会话管理） |
| 架构 | 事件驱动 `tmux -C` | 轮询（1s） | 轮询 | 不适用 |
| 语言 / 运行时 | Rust（单一二进制） | Go（单一二进制） | C/C++ | 不适用 |
| 配置文件 | TOML + CLI 覆盖 | 无 | 有 | tmux.conf（无 TUI 配置） |
| 主题引擎 | 4 套内置主题（20 色位） | 硬编码 256 色 | 有 | 不适用 |
| 插件系统 | 脚本式（TOML 清单 + JSON 协议） | 无 | 无 | 不适用 |
| 守护进程 RPC | JSON-RPC 2.0 via Unix socket | 无 | 无 | 不适用 |
| 工作区快照 | 保存/恢复布局为 JSON | 计划中，未实现 | 不适用 | 不适用 |
| 多路复用器抽象 | Trait 化（未来支持 Zellij） | 仅 tmux | 不适用 | 仅 tmux |
| 增量捕获 | FNV-1a 内容哈希差异比对 | 每次全量快照 | 不适用 | 不适用 |
| 搜索 | 跨会话/窗口/面板模糊搜索 | 精确匹配 | 进程过滤 | 不适用 |
| 二进制大小 | ~2MB（stripped, LTO） | ~15MB | 不等 | 不适用 |

**tmuxpulse 的典型开发者工作流：**

```
开发者有 10+ 个 tmux 会话正在运行
        |
tmuxpulse 以自适应网格渲染所有会话
        |
开发者实时查看面板输出，即时发现错误
        |
开发者按 Enter 聚焦某个会话，滚动查看输出
        |
开发者按 X 批量终止闲置会话，按 / 搜索
        |
开发者按 q 返回工作
```

无需手动切换会话。无需丢失上下文。只需一个持久的概览界面，实时反映你的 tmux 状态。

## 功能特性

- **事件驱动监控** -- tmux 控制模式（`tmux -C`）提供结构化的事件流，而非轮询；事件响应 <10ms
- **自适应网格布局** -- 会话卡片根据终端大小自动排列，可配置最小宽高
- **实时面板捕获** -- 活跃面板的实时输出，支持智能滚动（底部自动滚动，保留手动滚动位置）
- **活动脉冲** -- 有新输出的面板边框会触发 1.5 秒脉冲动画
- **闲置检测** -- 所有面板已死或空闲超过 1 小时的会话被标记为闲置，可批量终止
- **键盘 + 鼠标导航** -- 方向键网格导航，Enter 聚焦，鼠标点击卡片，滚轮滚动输出
- **命令面板** -- `Ctrl+P` 打开可键盘导航的操作菜单（刷新、显示隐藏、全部展开、终止闲置）
- **标签页系统** -- 概览网格标签页 + 单会话详情标签页；`Shift+Left/Right` 切换，`Ctrl+W` 关闭
- **搜索/过滤** -- `/` 打开实时搜索，跨会话名称、窗口名称和面板命令搜索
- **工作区快照** -- `tmuxpulse workspace save dev-setup` 将 tmux 拓扑保存为 JSON 以便恢复
- **插件系统** -- 通过 TOML 清单和 JSON stdin/stdout 协议扩展功能；支持 init、on_snapshot、on_event、shutdown 生命周期钩子
- **守护进程 RPC** -- `tmuxpulse daemon start` 启动 JSON-RPC 2.0 服务器（Unix socket），供 AI 代理、脚本和外部工具使用
- **TOML 配置** -- 所有视觉和行为参数均可配置：主题、按键绑定、捕获深度、闲置阈值、边框样式
- **4 套内置主题** -- Default、Catppuccin Mocha、Dracula、Nord -- 也可在 TOML 中自定义
- **预算式捕获** -- 每 tick 最多捕获 6 个面板，优先级调度：聚焦 > 光标 > 轮询
- **增量差异比对** -- FNV-1a 内容哈希，面板输出无变化时跳过重新渲染
- **JSON 输出** -- `--dump --json` 用于机器可读的快照输出，适合脚本和 AI 代理
- **跨平台** -- macOS 和 Linux（需要 tmux；Windows 通过 WSL 使用）

## 安装

### 预编译二进制文件（推荐）

从 [GitHub Releases](https://github.com/justinhuangcode/tmuxpulse/releases) 下载适合你平台的最新版本：

| 平台 | 文件 |
| --- | --- |
| Linux x86_64 | `tmuxpulse-linux-amd64.tar.gz` |
| Linux ARM64 | `tmuxpulse-linux-arm64.tar.gz` |
| macOS Intel | `tmuxpulse-macos-amd64.tar.gz` |
| macOS Apple Silicon | `tmuxpulse-macos-arm64.tar.gz` |

解压后将二进制文件放入 `$PATH` 即可。

### Homebrew (macOS / Linux)

```bash
brew tap justinhuangcode/tap
brew install tmuxpulse
```

### 通过 Cargo (crates.io) -- 即将上线

```bash
cargo install tmuxpulse
```

### 从源码编译

```bash
git clone https://github.com/justinhuangcode/tmuxpulse.git
cd tmuxpulse
cargo install --path .
```

**依赖条件：** Rust 1.88+ 和 tmux 3.1+。启动 tmuxpulse 前需先运行 tmux（`tmux new -s mysession`）。

## 快速开始

```bash
# 确保 tmux 正在运行
tmux new -s dev -d
tmux new -s build -d

# 启动 TUI
tmuxpulse

# 以文本格式输出会话信息
tmuxpulse --dump

# 以 JSON 格式输出（适合脚本 / AI 代理）
tmuxpulse --dump --json

# 使用指定主题
tmuxpulse --theme catppuccin-mocha

# 自定义轮询间隔
tmuxpulse --interval 500ms

# 生成默认配置文件
tmuxpulse config init

# 显示当前生效的配置
tmuxpulse config show

# 启动守护进程供 AI 代理访问
tmuxpulse daemon start

# 从脚本中查询守护进程
tmuxpulse daemon call pulse.sessions
tmuxpulse daemon call pulse.capture '{"pane_id": "%1", "lines": 50}'
```

## 命令

| 命令 | 描述 |
| --- | --- |
| *（默认）* | 启动 TUI 监控界面 |
| `config init` | 在 `~/.config/tmuxpulse/config.toml` 生成默认配置 |
| `config show` | 打印当前生效的配置 |
| `workspace save <name>` | 将当前 tmux 布局保存为命名快照 |
| `workspace restore <name>` | 恢复已保存的工作区 |
| `workspace list` | 列出所有已保存的工作区 |
| `plugin list` | 列出已安装的插件 |
| `plugin install <path>` | 从本地目录安装插件 |
| `daemon start` | 启动守护进程 RPC 服务器 |
| `daemon status` | 检查守护进程运行状态，显示版本/运行时间 |
| `daemon stop` | 停止运行中的守护进程 |
| `daemon call <method> [params]` | 向守护进程发送 RPC 调用（JSON 参数） |

## 全局选项

| 选项 | 默认值 | 描述 |
| --- | --- | --- |
| `--interval <duration>` | `1s` | 轮询间隔回退值（如 `1s`、`500ms`、`2m`） |
| `--tmux <path>` | 自动检测 | tmux 二进制文件路径 |
| `--dump` | false | 打印快照后退出（文本格式） |
| `--json` | false | 机器可读的 JSON 输出（与 `--dump` 配合使用） |
| `-c, --config <path>` | 自动 | 配置文件路径 |
| `--theme <name>` | 配置值 | 覆盖主题：`default`、`catppuccin-mocha`、`dracula`、`nord` |

## 守护进程启动选项

| 选项 | 默认值 | 描述 |
| --- | --- | --- |
| `--socket <path>` | `$XDG_RUNTIME_DIR/tmuxpulse.sock` | 自定义 Unix socket 路径 |

## 键盘快捷键

### 网格视图（概览）

| 按键 | 操作 |
| --- | --- |
| `方向键` | 在会话卡片间导航 |
| `Enter` | 聚焦选中的会话（全高输出） |
| `/` | 打开实时搜索过滤 |
| `Ctrl+P` | 打开命令面板 |
| `z` | 折叠/展开选中的会话卡片 |
| `Z` | 展开所有已折叠的卡片 |
| `t` | 在新标签页中打开选中的会话 |
| `X` | 终止所有闲置会话 |
| `q` | 退出 tmuxpulse |

### 聚焦视图

| 按键 | 操作 |
| --- | --- |
| `Up / Down` | 滚动面板输出 |
| `Esc` | 取消聚焦（返回网格） |
| `Ctrl+C` | 取消聚焦；再按一次退出 |

### 标签页导航

| 按键 | 操作 |
| --- | --- |
| `Shift+Right` | 下一个标签页 |
| `Shift+Left` | 上一个标签页 |
| `Ctrl+W` | 关闭当前标签页（概览标签页除外） |

### 鼠标支持

| 操作 | 效果 |
| --- | --- |
| 点击卡片 | 聚焦该会话 |
| 滚轮 | 滚动面板输出（聚焦时） |

## 插件系统

tmuxpulse 内置了插件系统，支持通过外部脚本扩展功能。插件是包含 `plugin.toml` 清单和可执行入口点的普通目录 -- 无需编译、WASM 或动态库。

```
~/.local/share/tmuxpulse/plugins/my-plugin/
├── plugin.toml              # 清单文件（必需）
└── run.sh                   # 插件可执行文件（入口点）
```

### 插件清单

```toml
name = "session-monitor"
version = "0.1.0"
description = "Monitors session activity and sends notifications"
entry = "./run.sh"
hooks = ["on_snapshot", "on_event"]
min_version = "0.1.0"
```

### 通信协议

插件通过 JSON stdin/stdout 与 tmuxpulse 通信。TmuxPulse 向插件的 stdin 发送每行一个 JSON 对象；插件在 stdout 上以每行一个 JSON 对象的形式响应。

### 生命周期钩子

| 事件 | 触发时机 | 消息体 |
| --- | --- | --- |
| `init` | 插件启动时加载 | `{"type": "init", "tmuxpulse_version": "0.1.0"}` |
| `on_snapshot` | 每个 tick 发送完整快照 | `{"type": "on_snapshot", "snapshot": {...}}` |
| `on_event` | 控制模式事件触发 | `{"type": "on_event", "event": "SessionCreated(...)"}` |
| `shutdown` | TmuxPulse 退出时 | `{"type": "shutdown"}` |

### 插件响应

插件可以返回状态行和通知：

```json
{"ok": true, "status": "3 active sessions", "notification": null}
```

| 字段 | 类型 | 描述 |
| --- | --- | --- |
| `ok` | bool | 插件是否成功处理了消息 |
| `status` | string? | 可选的状态行文本，显示在 TUI 中 |
| `notification` | string? | 可选的 toast 通知 |
| `log` | string? | 可选的日志消息（写入 tracing） |

### 插件 CLI

```bash
tmuxpulse plugin list              # 列出已安装的插件
tmuxpulse plugin install ./my-plugin  # 从本地目录安装插件
```

插件搜索目录（按优先级）：
1. 配置文件 `[plugins] directories` 中列出的路径
2. `~/.local/share/tmuxpulse/plugins/`
3. `~/.config/tmuxpulse/plugins/`

## 守护进程 RPC

tmuxpulse 包含守护进程模式，通过 Unix domain socket 暴露 JSON-RPC 2.0 API。这使得 AI 代理、脚本和外部工具可以查询 tmux 状态并发送命令，而无需解析 tmux 输出。

### 启动守护进程

```bash
# 使用默认 socket 路径启动
tmuxpulse daemon start

# 使用自定义 socket 启动
tmuxpulse daemon start --socket /tmp/my-tmuxpulse.sock

# 检查状态
tmuxpulse daemon status

# 停止
tmuxpulse daemon stop
```

### RPC 方法

| 方法 | 描述 | 必需参数 |
| --- | --- | --- |
| `pulse.ping` | 健康检查 | -- |
| `pulse.version` | 版本和运行时间 | -- |
| `pulse.snapshot` | 完整的 tmux 快照（会话、窗口、面板） | -- |
| `pulse.sessions` | 列出会话名称、ID 和计数 | -- |
| `pulse.capture` | 捕获面板输出 | `pane_id`，可选 `lines` |
| `pulse.send_keys` | 向面板发送按键 | `pane_id`，`keys` |
| `pulse.kill_session` | 终止会话 | `session_id` |

### 使用示例

```bash
# 通过 CLI
tmuxpulse daemon call pulse.ping
tmuxpulse daemon call pulse.sessions
tmuxpulse daemon call pulse.capture '{"pane_id": "%1", "lines": 100}'
tmuxpulse daemon call pulse.send_keys '{"pane_id": "%1", "keys": ["ls", "Enter"]}'

# 通过 Unix socket 从任何语言调用（NDJSON 协议）
echo '{"jsonrpc":"2.0","method":"pulse.snapshot","params":{},"id":1}' | socat - UNIX-CONNECT:/tmp/tmuxpulse-1000.sock
```

### 安全性

- Socket 权限设置为 `0600`（仅所有者可访问）
- 可在配置中设置 `auth_token` 进行 Bearer 认证
- 如果同一 socket 上已有实例运行，守护进程将拒绝启动

## 配置

tmuxpulse 从 `~/.config/tmuxpulse/config.toml` 读取配置。使用 `tmuxpulse config init` 生成默认配置文件：

```toml
[general]
theme = "default"              # default | catppuccin-mocha | dracula | nord
poll_interval_ms = 1000        # 回退轮询间隔（毫秒）
capture_lines = 200            # 每面板捕获行数
stale_threshold_secs = 3600    # 闲置多少秒后标记为闲置

[ui]
show_hidden = false            # 启动时显示隐藏会话
default_view = "grid"          # grid | detail
mouse = true                   # 启用鼠标支持
border_style = "rounded"       # rounded | plain | double | thick
show_status_bar = true         # 显示底部状态栏
card_min_width = 40            # 最小卡片宽度（列）
card_min_height = 12           # 最小卡片高度（行）

[keybindings]
quit = "q"
search = "/"
palette = "ctrl+p"
maximize = "z"
collapse = "c"
kill_stale = "X"
next_tab = "shift+right"
prev_tab = "shift+left"

[daemon]
socket_path = "/tmp/tmuxpulse.sock"   # RPC 的 Unix socket 路径
auth_token = "auto"                    # Bearer 令牌（"auto" = 无认证）

[plugins]
enabled = []                   # 启用的插件名称列表
directories = []               # 额外的插件搜索目录
```

所有字段都有合理的默认值 -- **零配置即可开箱即用**。CLI 选项覆盖配置文件值，配置文件值覆盖默认值。

## 主题引擎

tmuxpulse 内置 4 套主题，控制所有边框、文本和 UI 颜色：

| 主题 | 风格 | 适用场景 |
| --- | --- | --- |
| `default` | 256 色终端调色板 | 通用兼容性 |
| `catppuccin-mocha` | 暖色粉彩暗底 | 支持 RGB 的现代终端 |
| `dracula` | 亮色霓虹暗紫底 | 偏好高对比度 |
| `nord` | 冷色北极蓝 | 简约、宁静的审美 |

通过 CLI 或配置切换主题：

```bash
tmuxpulse --theme dracula
```

```toml
# ~/.config/tmuxpulse/config.toml
[general]
theme = "catppuccin-mocha"
```

每套主题定义 20 个色位（边框、背景、前景、强调色、状态指示器）。自定义主题将在未来版本中支持在配置文件中定义。

## 工作原理

1. `tmuxpulse` 启动时调用 `tmux list-sessions`、`list-windows`、`list-panes`，使用格式字符串构建所有会话、窗口和面板的类型化快照。

2. 控制模式客户端（`tmux -C attach`）提供事件流进行实时变更检测。`%session-created`、`%window-add`、`%output`、`%layout-change` 等事件触发针对性刷新，而非全量轮询。

3. 对每个可见的会话卡片，`tmux capture-pane -p -J` 获取活跃面板的最新输出。预算调度器每 tick 限制最多 6 次捕获，优先级为：聚焦会话 > 光标会话 > 其余轮询。

4. 捕获的内容使用 FNV-1a 哈希，与上一次哈希比较。若未变化则跳过渲染；若变化则更新视口并触发 1.5 秒的边框脉冲动画。

5. Ratatui TUI 渲染自适应的会话卡片网格、标签栏和状态栏。Crossterm 处理原始终端 I/O 和鼠标事件。

6. 用户输入通过单一状态的 Elm 式架构处理：`AppState` + `handle_key_event()` + `draw_ui()`。

7. 守护进程（启动后）运行后台快照刷新循环，并通过 Unix socket 提供 JSON-RPC 请求服务，使 AI 代理和脚本能够以编程方式与 tmux 交互。

## 架构

```
                      tmux -C attach（控制模式事件）
+-------------+       tmux list-sessions -F "..."       +--------------+
|  tmuxpulse  | -----> tmux list-windows -F "..."  ----> | tmux server  |
|             | <----- tmux capture-pane -p -J     <---- |              |
| +---------+ |       tmux send-keys                     +--------------+
| | Config  | |       tmux kill-session
| +---------+ |
| | State   | |       Unix Socket (JSON-RPC 2.0)
| +---------+ |       +----------------+
| | Plugins | | <---> |  AI 代理 /     |
| +---------+ |       |  脚本          |
| | Daemon  | |       +----------------+
| +---------+ |
| | Ratatui | |
| +---------+ |
| | Terminal| |
| +---------+ |
+-------------+
```

## 项目结构

```
src/
├── lib.rs                  # 库 crate 根（集成测试的公共 API）
├── main.rs                 # CLI 入口点、命令分发、时长解析
├── cli.rs                  # 命令行参数定义（clap v4 derive）
├── config/
│   ├── mod.rs              # TOML 配置加载、默认值、验证
│   └── theme.rs            # 主题引擎，4 套内置主题（每套 20 色位）
├── mux/
│   ├── mod.rs              # 核心类型：Session, Window, Pane, Snapshot, MuxEvent
│   └── tmux/
│       ├── mod.rs          # tmux 客户端：快照、捕获、发送按键、终止
│       ├── parser.rs       # Tab 分隔格式字符串解析
│       └── control.rs      # tmux 控制模式客户端（事件驱动监控）
├── plugin/
│   └── mod.rs              # 插件系统：TOML 清单、JSON 协议、生命周期钩子
├── daemon/
│   └── mod.rs              # 守护进程 RPC：Unix socket 上的 JSON-RPC 2.0
├── state/
│   └── mod.rs              # Elm 式应用状态、FNV-1a 内容哈希、卡片状态
└── ui/
    ├── mod.rs              # Ratatui 应用循环、输入处理、叠加层（搜索、面板、通知）
    ├── cards.rs            # 会话卡片组件（脉冲、闲置、折叠、聚焦状态）
    ├── layout.rs           # 自适应网格计算（终端大小 -> 列数 x 行数）
    ├── tabs.rs             # 标签栏组件（概览 + 单会话详情标签页）
    └── status.rs           # 状态栏组件（会话/面板计数、闲置计数、快捷键）
tests/
└── snapshots.rs            # Insta 快照测试，确保序列化稳定性（5 个测试）
.github/workflows/
├── ci.yml                  # CI：check, fmt, clippy, test (Linux + macOS), build, MSRV
└── release.yml             # Release：交叉编译 4 个目标平台，GitHub Release 附件
```

## 安全性与威胁模型

tmuxpulse 设计为**单用户、本地使用**的开发工具。以下安全控制已到位：

| 层 | 控制措施 | 详情 |
| --- | --- | --- |
| **tmux 访问** | 仅本地进程 | 通过子进程与 tmux 通信（`tmux list-sessions` 等）；无网络 I/O |
| **RPC 传输** | Unix socket + Bearer 令牌 | Socket 位于 `$XDG_RUNTIME_DIR/tmuxpulse.sock`，权限 `0600`；可选的每请求认证令牌 |
| **配置文件** | 仅所有者路径 | `~/.config/tmuxpulse/config.toml` 遵循 XDG 规范 |
| **工作区快照** | 仅所有者目录 | 保存至 `~/.local/share/tmuxpulse/workspaces/`，使用标准用户权限 |
| **插件系统** | 路径遍历防护 | 插件入口路径相对于插件目录解析；拒绝入口路径中的 `..` 段 |
| **tmux 命令** | 无 shell 注入 | 所有 tmux 参数作为独立 `&str` 传递给 `Command::new()`，绝不拼接为 shell 字符串 |
| **守护进程启动** | 单实例保护 | 守护进程检查现有 socket，如有其他实例运行则拒绝启动 |

### 不建议用于

- **多用户/共享机器** -- 具有 root 或相同 UID 访问权限的其他本地用户可以读取 tmux 会话。请通过操作系统级权限或容器限制访问。
- **不受信任的 tmux 会话** -- tmuxpulse 原样捕获和显示面板输出。面板输出中的恶意终端转义序列可能影响你的终端。
- **生产监控** -- tmuxpulse 是开发工具。对于生产环境，请使用专用的监控基础设施。

## 故障排除

### 找不到 tmux

```
Error: tmux not found in PATH. Install tmux:
  macOS: brew install tmux
  Ubuntu/Debian: sudo apt install tmux
  Fedora: sudo dnf install tmux
```

也可以显式指定二进制路径：

```bash
tmuxpulse --tmux /usr/local/bin/tmux
```

### 没有找到会话

tmuxpulse 需要至少一个正在运行的 tmux 会话：

```bash
tmux new -s dev -d    # 创建一个后台会话
tmuxpulse             # 现在会显示该会话
```

### 终端渲染问题

如果 TUI 渲染不正常，请尝试：

1. 确保终端支持 256 色（`echo $TERM` 应显示 `xterm-256color` 或类似值）
2. 尝试不同主题：`tmuxpulse --theme default`
3. 将终端大小调整到至少 80x24

### 配置文件错误

如果配置文件有语法错误，tmuxpulse 会回退到默认值并在 stderr 输出警告。重置方法：

```bash
rm ~/.config/tmuxpulse/config.toml
tmuxpulse config init
```

### 守护进程问题

```bash
# 检查守护进程是否运行
tmuxpulse daemon status

# 如果 socket 过时（守护进程崩溃），删除它
rm /tmp/tmuxpulse-*.sock
tmuxpulse daemon start
```

## 路线图

| 阶段 | 功能 | 状态 |
| --- | --- | --- |
| 1 | 核心 TUI（网格、卡片、标签页、搜索、命令面板） | 已完成 |
| 1 | tmux 客户端（快照、捕获、发送按键、终止） | 已完成 |
| 1 | TOML 配置 + 4 套内置主题 | 已完成 |
| 1 | 工作区保存 | 已完成 |
| 2 | tmux 控制模式（事件驱动，<10ms 延迟） | 已完成 |
| 2 | 插件系统（TOML 清单 + JSON 协议 + 钩子） | 已完成 |
| 2 | 守护进程模式 + JSON-RPC 2.0 服务器（Unix socket） | 已完成 |
| 2 | GitHub Actions CI/CD（Linux + macOS 矩阵） | 已完成 |
| 2 | Insta 快照测试（序列化稳定性） | 已完成 |
| 3 | 工作区恢复（重建会话/窗口/面板） | 计划中 |
| 3 | 模糊搜索（skim 集成） | 计划中 |
| 3 | AI 代理 SDK（Node.js + Python，零依赖） | 计划中 |
| 3 | Zellij 后端（Multiplexer trait） | 计划中 |
| 4 | Shell 补全（bash/zsh/fish） | 计划中 |
| 4 | Man 手册生成 | 计划中 |
| 4 | Homebrew tap + crates.io 发布 | 计划中 |

## 贡献

欢迎贡献！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

## 更新日志

请参阅 [CHANGELOG.md](CHANGELOG.md) 了解版本历史。

## 致谢

灵感来自 [steipete/tmuxwatch](https://github.com/steipete/tmuxwatch) 和 [justinhuangcode/browsercli](https://github.com/justinhuangcode/browsercli)。

## 许可证

[MIT](LICENSE)
