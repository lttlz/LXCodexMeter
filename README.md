# LX Codex Meter

[English](#english) | [中文](#中文)

**免费 · 开源 · 无广告 | Free · Open Source · Ad-Free**

[▶ 观看 1 分钟功能演示 / Watch the 1-minute demo](https://github.com/lttlz/LXCodexMeter/raw/refs/heads/main/docs/media/LXCodexMeter_v0.6.15_demo.mp4)

---

<a id="中文"></a>

## 中文

**免费、开源、无广告，本地、轻量、注重隐私的 Windows Codex 额度桌面监控工具。**

无需反复打开 Codex Usage 页面，即可在桌面悬浮窗或任务栏条中查看：

- 5 小时额度与重置时间
- 周额度与重置时间
- Credits 余额
- 最近刷新时间
- 每次任务的额度消耗与持续时间估算

LX Codex Meter 可以跟随 Codex / ChatGPT 的运行状态自动显示或隐藏，让额度信息在需要时出现，不需要时安静留在托盘。

> 只显示和统计你需要的额度信息，不读取你的对话、项目、网页或浏览器数据。

## 为什么选择 LX Codex Meter

### 一眼看到额度，不打断工作

额度信息始终以紧凑悬浮窗或任务栏条的形式显示在 Windows 桌面上，无需反复切换到 Usage 页面。

### 额度消耗统计，不再靠猜

LX Codex Meter 会根据本机保存的额度快照变化，估算每次任务的持续时间、周额度消耗和 5 小时额度消耗。

消耗日志支持：

- 查看任务数量、周额度总消耗、平均每次消耗和最长任务
- 按今天、最近 7 天、最近 30 天或全部时间筛选
- 按周额度消耗阈值筛选，并支持自定义阈值
- 按时间、周额度消耗或任务时长排序
- 查看每次任务结束时的周额度和 5 小时额度余额
- 将当前筛选结果导出为 CSV
- 删除单条历史日志或清空全部历史日志

统计依据是本机额度快照变化，因此属于估算结果，不读取 Codex 对话或任务内容。

### 跟随 Codex 自动出现和隐藏

- 检测到 Codex 或 ChatGPT 启动时自动显示
- 检测到它们关闭后自动隐藏到系统托盘
- 自动显示不会主动抢夺窗口焦点

### 隐藏时保持网络静默

当主窗口隐藏时：

- 不发起新的 Codex 额度读取
- 不启动 Meter 自有的 Codex App Server
- 延后后台自动更新检查
- 保留上一次已显示的额度数据

窗口重新显示后，才恢复额度刷新和自动更新检查。本地进程检测仍会继续运行，以保证自动显示功能正常。

### 本地读取，不抓取网页

额度信息通过本机 Codex App Server 获取。LX Codex Meter 不会读取：

- 浏览器 Cookie
- Token
- auth.json
- Codex 对话内容
- 用户项目文件
- 浏览器页面内容

## 核心功能

- Codex 5 小时额度、周额度、重置时间和 Credits 显示
- 本机额度快照与每次任务消耗估算
- 消耗日志筛选、排序、汇总统计和 CSV 导出
- 历史日志单条删除与全部清空
- 悬浮窗模式和任务栏条模式
- 窗口缩放、宽度调节和始终置顶
- Codex / ChatGPT 启停联动显示与隐藏
- 系统托盘、开机自启动和启动后隐藏
- 中文 / 英文界面和安装程序
- GitHub / Gitee 双更新源
- 应用内检查、下载并安装更新

## 工作方式与隐私

LX Codex Meter 通过本机 `codex.exe app-server --stdio` 读取账户额度信息。

Codex / ChatGPT 运行状态检测仅使用本机进程名和父进程关系，不读取完整命令行，不扫描 Codex 文件，也不读取会话内容。

额度消耗统计通过比较本机保存的额度快照变化进行估算，不读取提示词、对话内容或项目文件。

主窗口隐藏时，不会发起新的额度读取或后台自动更新请求。已经发起但尚未完成的请求不会被强制中止。

## 安装与下载

推荐安装包：

```text
LX.Codex.Meter_0.6.16_x64-setup.exe
```

备用安装包：

```text
LX.Codex.Meter_0.6.16_x64_en-US.msi
```

- 国内用户（Gitee）：https://gitee.com/lttlz/LXCodexMeter
- 国际用户（GitHub）：https://github.com/lttlz/LXCodexMeter

## 许可证

允许个人和企业免费使用、修改及免费分发。禁止直接销售本软件，或通过改名、换图标、轻微修改、重新打包等方式，将本软件或实质相同的软件作为收费产品或服务出售。详细条款见 [LICENSE](LICENSE) 和 [NOTICE](NOTICE)。

---

<a id="english"></a>

## English

**A free, open-source, ad-free, lightweight, privacy-focused Windows desktop meter for Codex usage.**

View the following without repeatedly opening the Codex Usage page:

- 5-hour limits and reset time
- Weekly limits and reset time
- Credit balance
- Last refresh time
- Per-task usage and duration estimates

LX Codex Meter can automatically appear when Codex or ChatGPT starts and hide to the system tray when they close.

> See and analyze the usage information you need without reading conversations, projects, browser data, or web pages.

## Why LX Codex Meter

### Usage at a glance

Keep Codex limits visible in a compact floating window or taskbar strip without repeatedly opening the Usage page.

### Per-task usage statistics

LX Codex Meter estimates each task's duration, weekly usage, and 5-hour usage from locally stored quota snapshot changes.

The usage log supports:

- Task count, total weekly usage, average usage per task, and longest-task summaries
- Today, last 7 days, last 30 days, and all-time filters
- Weekly-usage threshold filters, including a custom threshold
- Sorting by time, weekly usage, or task duration
- Remaining weekly and 5-hour quota at the end of each task
- CSV export of the currently filtered results
- Deleting individual records or clearing all history

These values are estimates based on local quota snapshots. The app does not read Codex conversations or task contents.

### Lifecycle-aware display

- Automatically shows when Codex or ChatGPT starts
- Automatically hides to the tray when they close
- Does not explicitly steal window focus when showing

### Network quiet while hidden

While the main window is hidden, LX Codex Meter does not start new usage requests, does not start its own Codex App Server child, and defers automatic update checks. The last displayed data is retained.

### Local retrieval without web scraping

Usage data is retrieved through the local Codex App Server. LX Codex Meter does not read browser cookies, tokens, `auth.json`, Codex conversations, project files, or browser page contents.

## Features

- Codex 5-hour and weekly limits, reset times, and Credits
- Local quota snapshots and per-task usage estimates
- Usage-log filtering, sorting, summary statistics, and CSV export
- Individual history deletion and clear-all history
- Floating window and taskbar strip modes
- Window scaling, width adjustment, and always-on-top support
- Codex / ChatGPT lifecycle-aware show and hide
- System tray, optional autostart, and start hidden
- Chinese / English UI and installers
- GitHub / Gitee update sources
- In-app update checks, download, and installation

## How it works and privacy

LX Codex Meter retrieves account usage through the local `codex.exe app-server --stdio` interface.

Lifecycle detection uses local process names and parent-process relationships only. Full command lines, Codex files, and session contents are not read.

Usage statistics are estimated by comparing locally stored quota snapshots. Prompts, conversations, and project files are not read.

No new usage retrieval or automatic update request is started while the main window is hidden. Requests already in progress are not forcibly aborted.

## Installation and downloads

Recommended installer:

```text
LX.Codex.Meter_0.6.16_x64-setup.exe
```

Alternative installer:

```text
LX.Codex.Meter_0.6.16_x64_en-US.msi
```

- Mainland China users (Gitee): https://gitee.com/lttlz/LXCodexMeter
- Global users (GitHub): https://github.com/lttlz/LXCodexMeter

## License

Free personal and business use, modification, and free redistribution are permitted. Selling LX Codex Meter itself, or a renamed, rebranded, lightly modified, or substantially equivalent repackaged product or service, is not permitted without separate written authorization. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
