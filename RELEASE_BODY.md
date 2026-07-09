LX Codex Meter v0.6.14

LX Codex Meter v0.6.14 新增 Codex / ChatGPT 进程检测与窗口自动显示 / 隐藏功能，同时加入开机自启动、中英文界面切换、右键菜单隐藏到托盘、应用内下载并安装更新、NSIS 安装包语言选择等改进。窗口缩放系统保持 v0.6.13 发行版逻辑不变。

新增功能：
- Codex / ChatGPT 进程检测：通过 Win32_Process 查询进程名与命令行，ChatGPT.exe 存在即认为用户正在运行；codex.exe 排除命令行包含 app-server 或 --stdio 的自有数据读取子进程
- Codex 运行时自动显示：检测到 Codex / ChatGPT 从未运行变为运行时，自动显示窗口一次（使用 SW_SHOWNOACTIVATE，不抢焦点）
- Codex 关闭后自动隐藏：检测到 Codex / ChatGPT 关闭后（连续 2 次检测确认），自动隐藏到托盘一次；用户手动显示后不再自动隐藏，直到下一个完整运行周期
- 开机自启动：基于 Tauri 官方 autostart 插件，写入 HKCU Run 键，无需管理员权限
- 开机启动后隐藏到托盘：仅在 --autostart 参数启动且配置开启时生效，手动启动始终显示窗口
- 中英文界面切换：设置页可切换中文 / 英文，托盘菜单同步切换语言
- 右键菜单增加"隐藏到托盘 / Hide to tray"：仅隐藏当前窗口，不退出应用，不修改显示配置
- 应用内下载并安装更新：检查到新版本后可直接在设置页下载并安装，支持进度提示与重启
- NSIS 安装包中英文语言选择：每次安装显示语言选择器
- 图标更新：替换为透明圆角图标集
- 英文悬浮窗缩写：Weekly → Wk, Used → U, Reset → Rst, Voucher → Vch

隐私说明：
本工具不读取浏览器 Cookie、token、auth.json，不抓取网页，不扫描用户项目，不读取 Codex 会话内容。额度信息仅通过本机 Codex App Server 获取。进程检测仅查询进程名与命令行参数，不读取任何文件或会话内容。自动更新仅连接官方 GitHub / Gitee 发布源，不收集任何用户数据。

推荐下载：
- LX Codex Meter_0.6.14_x64-setup.exe

备用安装包：
- LX Codex Meter_0.6.14_x64_en-US.msi

国内用户：https://gitee.com/lttlz/LXCodexMeter
国际用户：https://github.com/lttlz/LXCodexMeter

---

LX Codex Meter v0.6.14

LX Codex Meter v0.6.14 adds Codex / ChatGPT process detection with auto-show / auto-hide, plus autostart, Chinese / English UI switching, right-click hide-to-tray, in-app download-and-install updates, NSIS installer language selector, and icon refresh. The window scaling system retains the v0.6.13 release logic unchanged.

New features:
- Codex / ChatGPT process detection: queries Win32_Process for process name and command line. ChatGPT.exe presence means the user is running it; codex.exe processes whose command line contains app-server or --stdio (LXCodexMeter's own data-source child) are excluded
- Auto-show when Codex running: when Codex / ChatGPT transitions from not-running to running, the window is shown once using SW_SHOWNOACTIVATE (no focus steal)
- Auto-hide when Codex closes: after Codex / ChatGPT closes (confirmed by 2 consecutive detections), the window hides to tray once; after the user manually shows the window, auto-hide is suppressed until the next full run cycle
- Autostart: based on the official Tauri autostart plugin, writes HKCU Run key, no admin required
- Start hidden after Windows startup: only effective when launched with --autostart arg and config enabled; manual launch always shows the window
- Chinese / English UI switch: settings page can toggle between Chinese and English; tray menu syncs language
- Right-click menu "Hide to tray": hides the current window only, does not exit the app or modify show config
- In-app download and install update: after finding a new version, download and install directly from the settings page with progress and restart support
- NSIS installer Chinese / English language selector: shows language selector on every install
- Icon refresh: replaced with transparent rounded-corner icon set
- English floating window abbreviations: Weekly → Wk, Used → U, Reset → Rst, Voucher → Vch

Privacy:
LX Codex Meter does not read browser cookies, tokens, auth.json, web pages, user projects, or Codex conversation content. Usage data is retrieved only through the local Codex App Server. Process detection queries only process names and command-line arguments, without reading any files or session content. The automatic update feature only contacts the official GitHub / Gitee release endpoints and collects no user data.

Recommended download:
- LX Codex Meter_0.6.14_x64-setup.exe

Alternative installer:
- LX Codex Meter_0.6.14_x64_en-US.msi

Mainland China users: https://gitee.com/lttlz/LXCodexMeter
Global users: https://github.com/lttlz/LXCodexMeter
