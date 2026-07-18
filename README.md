# LX Codex Meter

[English](#english) | [中文](#中文)

**免费 · 开源 · 无广告 | Free · Open Source · Ad-Free**

[▶ 观看 1 分钟功能演示 / Watch the 1-minute demo](docs/media/LXCodexMeter_v0.6.15_demo.mp4)

---

<a id="中文"></a>

## 中文

**免费、开源、无广告，本地、轻量、注重隐私的 Windows Codex 额度桌面监控工具。**

无需反复打开 Codex Usage 页面，即可在桌面悬浮窗或任务栏条中查看：

- 5 小时额度与重置时间
- 周额度与重置时间
- Credits 余额
- 最近刷新时间

LX Codex Meter 可以跟随 Codex / ChatGPT 的运行状态自动显示或隐藏，让额度信息在需要时出现，不需要时安静留在托盘。

> 只显示你需要的额度信息，不读取你的对话、项目、网页或浏览器数据。

## 为什么选择 LX Codex Meter

### 一眼看到额度，不打断工作

不需要切换到 Usage 页面，也不需要在浏览器和开发工具之间来回查找。

额度信息始终以紧凑悬浮窗或任务栏条的形式显示在 Windows 桌面上。

### 跟随 Codex 自动出现和隐藏

检测到 Codex 或 ChatGPT 启动时，Meter 可以自动显示。

检测到它们关闭后，Meter 可以自动隐藏到系统托盘。

自动显示不会主动调用窗口聚焦，且显示后仍可以立即再次隐藏。

### 隐藏时保持网络静默

当 LX Codex Meter 主窗口隐藏时：

- 不发起新的 Codex 额度读取
- 不启动 Meter 自有的 Codex App Server
- 延后后台自动更新检查
- 保留上一次已显示的额度数据

窗口重新显示后，才恢复额度刷新和自动更新检查。

本地 Codex / ChatGPT 进程检测仍会继续运行，以保证自动显示功能正常。

### 本地读取，不抓取网页

额度信息通过本机 Codex App Server 获取。

LX Codex Meter 不会读取：

- 浏览器 Cookie
- Token
- auth.json
- Codex 对话内容
- 用户项目文件
- 浏览器页面内容

### 为 Windows 桌面使用而设计

- 紧凑悬浮窗
- 任务栏条模式
- 窗口缩放和宽度调节
- 始终置顶
- 系统托盘菜单
- 开机自启动
- 启动后隐藏
- 中文 / 英文界面
- 中文 / 英文安装程序

### 更完整的更新体验

- GitHub / Gitee 双更新源
- 安装包签名校验
- 应用内检查更新
- 应用内下载并安装
- 保留手动下载入口

## 核心功能

- Codex 5 小时额度显示
- 5 小时额度重置时间
- Codex 周额度显示
- 周额度重置时间
- Credits 信息显示
- 最近刷新时间
- 悬浮窗模式
- 任务栏条模式
- Codex / ChatGPT 启动时自动显示
- Codex / ChatGPT 关闭后自动隐藏
- 手动隐藏到托盘
- 开机自启动
- 启动时隐藏
- 中英文界面
- 中英文 NSIS 安装程序
- GitHub / Gitee 双更新源
- 签名验证
- 应用内下载并安装更新

## 工作方式

LX Codex Meter 通过本机 Codex App Server 读取账户额度信息。

Codex / ChatGPT 运行状态检测使用本机进程名和父进程关系：

- ChatGPT.exe 视为用户正在运行 ChatGPT / Codex
- codex.exe 只有在直接父进程是 LX Codex Meter 时，才会被识别为 Meter 自有额度读取子进程并排除
- 不读取完整命令行
- 不扫描 Codex 文件
- 不读取会话内容

## 隐私说明

LX Codex Meter 不收集用户数据。

它不会读取或上传：

- 浏览器 Cookie
- Token
- auth.json
- Codex 会话
- 项目文件
- 提示词
- 网页内容

主窗口隐藏时，不会发起新的额度读取或后台自动更新请求。

已经发起但尚未完成的请求不会被强制中止。隐藏状态的网络静默仅针对 LX Codex Meter 自身发起的额度读取和自动更新检查，不影响 WebView2 运行时或系统其他组件的网络活动。

## 安装与下载

推荐安装包：

```
LX.Codex.Meter_0.6.16_x64-setup.exe
```

备用安装包：

```
LX.Codex.Meter_0.6.16_x64_en-US.msi
```

下载地址：

- 国内用户（Gitee）：https://gitee.com/lttlz/LXCodexMeter
- 国际用户（GitHub）：https://github.com/lttlz/LXCodexMeter

## 自动更新

窗口可见时，LX Codex Meter 会连接官方 GitHub / Gitee 发布源检查更新。

如果程序以隐藏状态启动，后台自动更新检查会等待到窗口首次显示后再执行。

用户主动点击"检查更新"时仍会立即执行检查。

更新安装包经过签名校验。

---

<a id="english"></a>

## English

**A free, open-source, ad-free, lightweight, privacy-focused Windows desktop meter for Codex usage.**

View the following without repeatedly opening the Codex Usage page:

- 5-hour usage limits and reset time
- Weekly usage limits and reset time
- Credit balance
- Last refresh time

LX Codex Meter can automatically appear when Codex or ChatGPT starts and hide to the system tray when they close.

> See the usage information you need without reading your conversations, projects, browser data, or web pages.

## Why LX Codex Meter

### Usage at a glance

Keep Codex limits visible in a compact floating window or taskbar strip without repeatedly opening the Usage page.

### Lifecycle-aware auto show and hide

The Meter can automatically show when Codex or ChatGPT starts and hide when they close.

Automatic show does not explicitly focus the window, and the Meter can be hidden again immediately.

### Network quiet while hidden

While the main window is hidden, LX Codex Meter:

- Does not start new usage retrieval requests
- Does not start its own Codex App Server child
- Defers automatic update checks
- Keeps the last displayed usage data

Usage refresh and the deferred update check resume when the window becomes visible.

Local Codex / ChatGPT process detection continues to run so that auto show remains functional.

### Local retrieval without web scraping

Usage data is retrieved through the local Codex App Server.

LX Codex Meter does not read:

- Browser cookies
- Tokens
- auth.json
- Codex conversations
- User project files
- Browser page contents

### Built for the Windows desktop

- Compact floating window
- Taskbar strip mode
- Window scale and width controls
- Always-on-top support
- System tray integration
- Optional autostart
- Optional start hidden
- Chinese / English UI
- Chinese / English NSIS installer

### Signed in-app updates

- GitHub / Gitee update sources
- Signed installer verification
- In-app update checks
- In-app download and installation
- Manual download fallback

## Features

- Codex 5-hour usage display
- 5-hour usage reset time
- Codex weekly usage display
- Weekly usage reset time
- Credits information display
- Last refresh time
- Floating window mode
- Taskbar strip mode
- Auto show when Codex / ChatGPT starts
- Auto hide when Codex / ChatGPT closes
- Manual hide to tray
- Autostart
- Start hidden
- Chinese / English UI
- Chinese / English NSIS installer
- GitHub / Gitee dual update sources
- Signature verification
- In-app download and install updates

## How it works

LX Codex Meter retrieves account usage information through the local Codex App Server.

Codex / ChatGPT lifecycle detection uses local process names and parent process relationships:

- ChatGPT.exe is treated as the user running ChatGPT / Codex
- codex.exe is recognized as the Meter's own usage-retrieval child and excluded only when its direct parent process is LX Codex Meter
- Full command lines are not read
- Codex files are not scanned
- Session contents are not read

## Privacy

LX Codex Meter does not collect user data.

It does not read or upload:

- Browser cookies
- Tokens
- auth.json
- Codex conversations
- Project files
- Prompts
- Web page contents

No new usage retrieval or automatic update request is started while the main window is hidden.

Requests that have already been started but not yet completed are not forcibly aborted. The network-quiet behavior applies only to requests initiated by LX Codex Meter itself, and does not affect network activity from the WebView2 runtime or other system components.

## Installation and downloads

Recommended installer:

```
LX.Codex.Meter_0.6.16_x64-setup.exe
```

Alternative installer:

```
LX.Codex.Meter_0.6.16_x64_en-US.msi
```

Downloads:

- Mainland China users (Gitee): https://gitee.com/lttlz/LXCodexMeter
- Global users (GitHub): https://github.com/lttlz/LXCodexMeter

## Automatic updates

When the window is visible, LX Codex Meter contacts the official GitHub / Gitee release endpoints to check for updates.

If the app starts in a hidden state, the background update check is deferred until the window is first shown.

Clicking "Check for updates" manually still performs the check immediately.

Update packages are signature-verified.
