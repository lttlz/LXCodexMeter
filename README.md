# LX Codex Meter

[English](#english) | [中文](#中文)

<a id="中文"></a>

## 中文简介

LX Codex Meter 是一个本地 Windows 桌面小工具，用于通过 Codex App Server 实时显示 Codex 额度状态。它提供悬浮窗、系统托盘和任务栏条模式，适合需要持续关注 Codex 使用额度的用户。

### 核心特性

- Codex App Server 自动实时模式
- 显示 5 小时额度、5 小时重置时间
- 显示周额度、周额度重置时间
- 显示 Credits 信息（如接口可用）
- 显示当前数据刷新时间
- 默认悬浮窗
- 任务栏条模式
- 系统托盘菜单
- 设置页
- GitHub / Gitee 项目链接
- Windows 安装包
- 应用内自动更新（Tauri Updater，签名校验）

### 隐私说明

LX Codex Meter：

- 不读取浏览器 Cookie
- 不读取 token
- 不读取 `auth.json`
- 不抓取网页
- 不扫描用户项目
- 不读取 Codex 会话内容
- 仅通过本机 Codex App Server 获取账户额度信息

自动更新功能仅连接官方 GitHub / Gitee 发布源下载更新清单与安装包，不收集任何用户数据。

### 安装

推荐安装包：

```
LX Codex Meter_0.6.12_x64-setup.exe
```

备用安装包：

```
LX Codex Meter_0.6.12_x64_en-US.msi
```

### 下载地址

- 国内用户（Gitee）：https://gitee.com/lttlz/LXCodexMeter
- 国际用户（GitHub）：https://github.com/lttlz/LXCodexMeter

### 自动更新

应用启动后会自动检查更新（仅连接官方发布源）。当有新版本时，会通过应用内通知提示。更新包经过签名校验，确保完整性。

---

<a id="english"></a>

## English

LX Codex Meter is a local Windows desktop utility for displaying Codex usage limits in real time through Codex App Server. It provides a compact floating window, system tray integration, and a taskbar strip mode for users who need quick visibility into Codex usage status.

### Features

- Automatic real-time mode via Codex App Server
- Displays 5-hour usage and 5-hour reset time
- Displays weekly usage and weekly reset time
- Displays Credits information when available
- Displays current data refresh time
- Compact floating window
- Taskbar strip mode
- System tray menu
- Settings page
- GitHub / Gitee project links
- Windows installer packages
- In-app automatic updates (Tauri Updater, signature verified)

### Privacy

LX Codex Meter does not read browser cookies, tokens, `auth.json`, web pages, user projects, or Codex conversation content. It only communicates with the local Codex App Server to retrieve account usage and rate limit information.

The automatic update feature only contacts the official GitHub / Gitee release endpoints to download the update manifest and installer. No user data is collected.

### Installation

Recommended installer:

```
LX Codex Meter_0.6.12_x64-setup.exe
```

Alternative installer:

```
LX Codex Meter_0.6.12_x64_en-US.msi
```

### Downloads

- Mainland China users (Gitee): https://gitee.com/lttlz/LXCodexMeter
- Global users (GitHub): https://github.com/lttlz/LXCodexMeter

### Automatic Updates

The app checks for updates automatically on startup (official release endpoints only). When a newer version is available, an in-app notification is shown. Update packages are signature-verified for integrity.
