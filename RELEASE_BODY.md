# LX Codex Meter v0.6.14

v0.6.14 是一次较大的桌面体验、自动化、安装和隐私升级。

这一版本让 LX Codex Meter 不再只是一个额度显示窗口，而是能够跟随 Codex / ChatGPT 工作状态自动出现和隐藏的 Windows 桌面工具。

## 本版本亮点

### 跟随 Codex 自动显示和隐藏

- Codex / ChatGPT 启动时可以自动显示 Meter
- Codex / ChatGPT 关闭后可以自动隐藏到托盘
- 自动显示不主动调用窗口聚焦
- 自动显示后仍可立即再次隐藏
- 修复了必须先打开设置页才能再次隐藏的问题

### 隐藏状态网络静默

当 Meter 窗口隐藏时：

- 不发起新的 Codex 额度读取
- 不启动 Meter 自有 Codex App Server
- 不执行定时额度刷新
- 延后后台自动更新检查
- 保留上一次额度数据显示

窗口重新显示后会立即刷新额度，并恢复正常更新检查。

### 更完整的 Windows 桌面体验

- 新增开机自启动
- 新增启动后隐藏到托盘
- 新增右键隐藏到托盘
- 新增中文 / 英文界面切换
- 新增中文 / 英文安装程序语言选择
- 改进任务栏条模式和窗口贴合
- 改进窗口宽度、缩放和设置页布局
- 更新应用、托盘、安装程序和桌面图标

### 应用内更新

- 检测到新版本后可以直接下载并安装
- 保留 GitHub / Gitee 手动下载入口
- 安装包继续使用签名校验
- 不执行静默后台安装

## 隐私设计

LX Codex Meter 不读取：

- 浏览器 Cookie
- Token
- auth.json
- Codex 对话内容
- 用户项目文件
- 网页内容

额度信息仅通过本机 Codex App Server 获取。

Codex / ChatGPT 生命周期检测只使用进程名和父进程关系，不读取完整命令行、文件或会话内容。

窗口隐藏时不会发起新的额度读取和自动更新请求。

## 推荐下载

LX.Codex.Meter_0.6.14_x64-setup.exe

## 备用安装包

LX.Codex.Meter_0.6.14_x64_en-US.msi

---

# LX Codex Meter v0.6.14

v0.6.14 is a major desktop experience, automation, installer, and privacy update.

LX Codex Meter is no longer only a usage display window. It can now follow the Codex / ChatGPT lifecycle and appear only when needed.

## Highlights

### Lifecycle-aware auto show and hide

- Automatically show when Codex or ChatGPT starts
- Automatically hide to tray after they close
- Automatic show does not explicitly focus the window
- The Meter can be hidden again immediately after auto-show
- Fixes the issue where Settings had to be opened before the window could be hidden again

### Network quiet while hidden

While the Meter window is hidden:

- No new Codex usage requests are started
- The Meter does not start its own Codex App Server child
- Scheduled usage refreshes are skipped
- Automatic update checks are deferred
- The last displayed usage data is kept

Usage is refreshed immediately after the window becomes visible again.

### Improved Windows desktop experience

- Optional autostart
- Optional start hidden
- Right-click hide-to-tray
- Chinese / English UI
- Chinese / English NSIS installer language selector
- Improved taskbar strip positioning
- Improved width, scale, and Settings layout
- Updated app, tray, installer, and desktop icons

### In-app updates

- Download and install updates inside the app
- GitHub / Gitee manual download fallback
- Signed update verification
- No silent background installation

## Privacy

LX Codex Meter does not read browser cookies, tokens, auth.json, Codex conversations, project files, or web page contents.

Usage information is retrieved only through the local Codex App Server.

Lifecycle detection uses process names and parent process relationships without reading full command lines, files, or session contents.

No new usage retrieval or automatic update request is started while the main window is hidden.

## Recommended download

LX.Codex.Meter_0.6.14_x64-setup.exe

## Alternative installer

LX.Codex.Meter_0.6.14_x64_en-US.msi
