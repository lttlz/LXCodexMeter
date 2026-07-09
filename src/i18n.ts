import type { Language } from './types';

// Minimal hand-rolled i18n. No framework dependency.
// Keys are stable string identifiers; each language must provide the same keys.
type Dict = Record<string, string>;

const zh: Dict = {
  appTitle: 'LX Codex Meter',
  loading: '正在读取 Codex 额度…',
  failed: '读取失败',

  primary: '主额度',
  secondary: '副额度',
  credits: 'Credits',
  used: '用',
  resetLabel: '重置',
  resetCredits: '重置券',
  data: '数据',
  unknown: '未知',

  strip5h: '5h',
  stripWeek: '周',
  stripReset: '重置',
  stripVoucher: '重置券',
  stripData: '数据',
  stripCredits: 'Credits',

  refreshInterval: '刷新间隔',
  opacity: '透明度',
  alwaysOnTop: '置顶显示',
  taskbarStrip: '任务栏条模式，伪嵌入',
  autoUpdate: '自动更新',
  checkUpdate: '立即检查更新',
  checking: '正在检查...',
  showCreditsData: '显示 Credits / 数据',
  backToFloat: '切回默认悬浮窗',
  closeSettings: '关闭设置',
  autostart: '开机自动启动 LX Codex Meter',
  startHidden: '启动后隐藏到托盘',
  autoShowCodex: 'Codex 运行时自动显示',
  autoHideCodex: 'Codex 关闭后自动隐藏到托盘',
  language: '语言',
  langZh: '中文',
  langEn: 'English',
  minute: '分钟',

  supportTitle: '支持与联系',
  supportDesc: '本软件完全免费开放全部功能，无付费限制、无强制打赏。若您觉得工具实用，可自愿小额赞赏支持后续维护更新，支持与否不影响任何使用权限。',
  donation: '自愿赞赏',
  addFriend: '添加好友',

  author: '作者',

  updateChecking: '正在检查更新...',
  updateUpToDate: '当前已是最新版本',
  updateFound: '发现新版本',
  updateCheckFailed: '检查更新失败，请稍后重试',
  downloadInstall: '下载并安装',
  updatingDownload: '正在下载更新...',
  updatingInstall: '正在安装更新...',
  updateDone: '更新安装完成，请重启应用',
  updateFailed: '更新失败，请前往发布页手动下载',
  releasePage: '发布页',
  restart: '重启',
  manualDownload: '手动下载',

  menuRefresh: '刷新',
  menuSettings: '设置',
  menuCloseSettings: '关闭设置',
  menuStripOn: '切换任务栏条模式',
  menuStripOff: '切回默认悬浮窗',
  menuQuit: '退出',

  titleRefresh: '刷新',
  titleStrip: '切换任务栏条模式',
  titleSettings: '设置',
};

const en: Dict = {
  appTitle: 'LX Codex Meter',
  loading: 'Reading Codex usage…',
  failed: 'Failed to read',

  primary: 'Primary',
  secondary: 'Weekly',
  credits: 'Credits',
  used: 'Used',
  resetLabel: 'Reset',
  resetCredits: 'Voucher',
  data: 'Data',
  unknown: 'Unknown',

  strip5h: '5h',
  stripWeek: 'Wk',
  stripReset: 'Reset',
  stripVoucher: 'Vch',
  stripData: 'Data',
  stripCredits: 'Credits',

  refreshInterval: 'Refresh interval',
  opacity: 'Opacity',
  alwaysOnTop: 'Always on top',
  taskbarStrip: 'Taskbar strip mode (pseudo-embed)',
  autoUpdate: 'Auto update',
  checkUpdate: 'Check for updates',
  checking: 'Checking...',
  showCreditsData: 'Show Credits / Data',
  backToFloat: 'Back to floating window',
  closeSettings: 'Close settings',
  autostart: 'Launch LX Codex Meter on system startup',
  startHidden: 'Start hidden to tray',
  autoShowCodex: 'Auto-show when Codex is running',
  autoHideCodex: 'Auto-hide to tray when Codex closes',
  language: 'Language',
  langZh: 'Chinese',
  langEn: 'English',
  minute: 'min',

  supportTitle: 'Support & Contact',
  supportDesc: 'This software is completely free with no paid limits and no mandatory tips. If you find it useful, you may optionally support future development with a small donation. Tipping does not affect any usage rights.',
  donation: 'Donate',
  addFriend: 'Add friend',

  author: 'Author',

  updateChecking: 'Checking for updates...',
  updateUpToDate: 'You are on the latest version',
  updateFound: 'New version available',
  updateCheckFailed: 'Update check failed, please try again later',
  downloadInstall: 'Download and install',
  updatingDownload: 'Downloading update...',
  updatingInstall: 'Installing update...',
  updateDone: 'Update installed, please restart the app',
  updateFailed: 'Update failed, please download manually from the release page',
  releasePage: 'Release page',
  restart: 'Restart',
  manualDownload: 'Manual download',

  menuRefresh: 'Refresh',
  menuSettings: 'Settings',
  menuCloseSettings: 'Close settings',
  menuStripOn: 'Toggle strip mode',
  menuStripOff: 'Back to floating window',
  menuQuit: 'Quit',

  titleRefresh: 'Refresh',
  titleStrip: 'Toggle strip mode',
  titleSettings: 'Settings',
};

const dicts: Record<Language, Dict> = { zh, en };

export function tr(lang: Language, key: string): string {
  return dicts[lang]?.[key] ?? zh[key] ?? key;
}
