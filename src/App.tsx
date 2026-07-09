import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ButtonHTMLAttributes, CSSProperties, MouseEvent, SyntheticEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Menu } from '@tauri-apps/api/menu';
import type { CodexMeterStatus, Language, LimitWindow, MeterConfig } from './types';
import { tr } from './i18n';
import donationQr from './assets/support/donation-qr.png';
import wechatQr from './assets/support/wechat-qr.png';
import logoImg from './assets/logo.png';

const APP_NAME = 'LX Codex Meter';
const APP_VERSION = '0.6.14';
const APP_AUTHOR = 'lttlz';
const GITHUB_URL = 'https://github.com/lttlz/LXCodexMeter';
const GITEE_URL = 'https://gitee.com/lttlz/LXCodexMeter';
const GITHUB_RELEASES = 'https://github.com/lttlz/LXCodexMeter/releases';

type WindowBaseSize = {
  width: number;
  height: number;
};

const DEFAULT_CONFIG: MeterConfig = {
  refresh_interval_secs: 300,
  show_floating_window: true,
  opacity: 0.92,
  always_on_top: true,
  compact: false,
  ui_scale: 1,
  taskbar_strip: false,
  show_reset_time: true,
  auto_update: false,
  source_mode: 'app_server',
  autostart: false,
  start_hidden: false,
  auto_show_on_codex: false,
  auto_hide_on_codex_close: false,
  language: 'zh',
};

// Default width and aspect ratios used when the user has not resized the window.
// When the user resizes, the current width is preserved across settings open/close
// and the height is recomputed from these ratios (in-session only, not persisted).
const DEFAULT_W = 215;
const SETTINGS_RATIO = 660 / 215;
const NORMAL_RATIO = 178 / 215;
const ERROR_RATIO = 150 / 215;

function clamp(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, n));
}

function getBaseWindowSize(
  strip: boolean,
  open: boolean,
  statusOk: boolean | null | undefined,
  preservedWidth: number,
  userResized: boolean,
): WindowBaseSize {
  // All four states (normal, normal-settings, strip, strip-settings) share the
  // same default width (215). When the user has manually resized, that width is
  // reused across all states so it never snaps back. Strip mode never auto-
  // measures content width; long content is handled by layout compression.
  const w = userResized ? preservedWidth : DEFAULT_W;
  if (strip) {
    if (open) {
      return { width: w, height: Math.round(w * SETTINGS_RATIO) };
    }
    return { width: w, height: 34 };
  }
  if (open) {
    return { width: w, height: Math.round(w * SETTINGS_RATIO) };
  }
  const ratio = statusOk === false ? ERROR_RATIO : NORMAL_RATIO;
  return { width: w, height: Math.round(w * ratio) };
}

function getViewportSize(): WindowBaseSize {
  return {
    width: Math.max(1, Math.round(window.innerWidth || 1)),
    height: Math.max(1, Math.round(window.innerHeight || 1)),
  };
}

function formatReset(
  seconds: number | null,
  text?: string | null,
  dateMode: 'auto' | 'date' | 'time' = 'auto',
  unknown = '未知',
): string {
  if (seconds) {
    try {
      const date = new Date(seconds * 1000);
      const now = new Date();
      const sameDay = date.getFullYear() === now.getFullYear()
        && date.getMonth() === now.getMonth()
        && date.getDate() === now.getDate();
      const time = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      if (dateMode === 'date' || (dateMode === 'auto' && !sameDay)) {
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const day = String(date.getDate()).padStart(2, '0');
        return `${month}/${day} ${time}`;
      }
      return time;
    } catch {
      // Fall through to reset_text below.
    }
  }
  if (text) return text;
  return unknown;
}

// Data time: default HH:mm (no seconds). Empty string when not refreshed.
function formatUpdated(ms: number): string {
  if (!ms) return '';
  try {
    return new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch {
    return '';
  }
}

function pct(n: number | null): string {
  if (n === null || Number.isNaN(n)) return '--';
  return `${Math.max(0, Math.min(100, Math.round(n)))}%`;
}

function limitTitle(limit: LimitWindow | null, fallback: string): string {
  if (!limit) return fallback;
  const mins = limit.window_duration_mins;
  if (mins === 300) return '5h';
  if (mins === 10080) return fallback;
  if (mins === 60) return '1h';
  if (mins === 15) return '15m';
  if (mins && mins > 60 && mins < 1440) return `${Math.round(mins / 60)}h`;
  if (mins && mins >= 1440) return `${Math.round(mins / 1440)}d`;
  return limit.label || fallback;
}

type TFunc = (key: string) => string;

function LimitRow({
  limit,
  fallback,
  resetDateMode = 'auto',
  t,
}: {
  limit: LimitWindow | null;
  fallback: string;
  resetDateMode?: 'auto' | 'date' | 'time';
  t: TFunc;
}) {
  const remaining = limit?.remaining_percent ?? null;
  const used = limit?.used_percent ?? null;
  return (
    <div className="row">
      <span className="label">{limitTitle(limit, fallback)}</span>
      <span className="value">{pct(remaining)}</span>
      <span className="sub">{t('used')} {pct(used)} · {t('resetLabel')} {formatReset(limit?.resets_at ?? null, limit?.reset_text, resetDateMode, t('unknown'))}</span>
    </div>
  );
}

function stopDragEvent(e: SyntheticEvent) {
  e.stopPropagation();
}

function ActionButton({ onPointerDown, onMouseDown, type = 'button', ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type={type}
      {...props}
      onPointerDown={(e) => {
        e.stopPropagation();
        onPointerDown?.(e);
      }}
      onMouseDown={(e) => {
        e.stopPropagation();
        onMouseDown?.(e);
      }}
    />
  );
}

function startWindowDrag(event?: MouseEvent) {
  if (event && event.button !== 0) return;
  getCurrentWindow().startDragging().catch(() => undefined);
}

export default function App() {
  const [status, setStatus] = useState<CodexMeterStatus | null>(null);
  const [config, setConfig] = useState<MeterConfig>(DEFAULT_CONFIG);
  const [configReady, setConfigReady] = useState(false);
  const [loading, setLoading] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [viewportSize, setViewportSize] = useState<WindowBaseSize>(() => getViewportSize());
  const programmaticResizeRef = useRef(false);
  const programmaticResizeTimerRef = useRef<number | undefined>(undefined);
  const lastBaseKeyRef = useRef('');
  const preservedWidthRef = useRef(DEFAULT_W);
  const userResizedRef = useRef(false);
  const startupDoneRef = useRef(false);
  const skipAutoShowRef = useRef(false);

  const lang = config.language;
  const t = useMemo<TFunc>(() => (key: string) => tr(lang, key), [lang]);

  const baseSize = useMemo(
    () => getBaseWindowSize(config.taskbar_strip, settingsOpen, status?.ok, preservedWidthRef.current, userResizedRef.current),
    [config.taskbar_strip, settingsOpen, status?.ok],
  );
  const rawAutoMaxScale = Math.min(viewportSize.width / baseSize.width, viewportSize.height / baseSize.height);
  const contentScale = Number.isFinite(rawAutoMaxScale) && rawAutoMaxScale > 0
    ? Math.max(0.1, rawAutoMaxScale)
    : 1;
  const resizeWindowToBase = useCallback(async (size: WindowBaseSize) => {
    const width = Math.max(1, Math.ceil(size.width));
    const height = Math.max(1, Math.ceil(size.height));
    const win = getCurrentWindow();
    programmaticResizeRef.current = true;
    if (programmaticResizeTimerRef.current) {
      window.clearTimeout(programmaticResizeTimerRef.current);
      programmaticResizeTimerRef.current = undefined;
    }

    await win.setSize(new LogicalSize(width, height)).catch(() => undefined);
    setViewportSize({ width, height });

    programmaticResizeTimerRef.current = window.setTimeout(() => {
      void win.setSize(new LogicalSize(width, height)).catch(() => undefined);
      setViewportSize(getViewportSize());
      programmaticResizeRef.current = false;
      programmaticResizeTimerRef.current = undefined;
    }, 90);
  }, []);

  const openSettings = useCallback(() => {
    // User-initiated open: tell the backend watcher not to auto-hide afterwards.
    void invoke('mark_manual_show').catch(() => undefined);
    setSettingsOpen(true);
    window.setTimeout(() => {
      void getCurrentWindow().show();
      void getCurrentWindow().setFocus();
    }, 0);
  }, []);

  const closeSettingsNow = useCallback(() => {
    setSettingsOpen(false);
  }, []);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const next = await invoke<CodexMeterStatus>('get_status', {
        mode: 'app_server',
        clientText: '',
      });
      setStatus(next);
    } catch (error) {
      setStatus({
        ok: false,
        message: error instanceof Error ? error.message : String(error),
        source_mode: 'app_server',
        auth_mode: null,
        plan_type: null,
        primary: null,
        secondary: null,
        credit_balance: null,
        credit_limit: null,
        reset_credits_available: null,
        updated_at_ms: Date.now(),
      });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    invoke<MeterConfig>('load_config')
      .then((loaded) => {
        const merged = { ...DEFAULT_CONFIG, ...loaded, source_mode: 'app_server' as const };
        merged.ui_scale = clamp(Number(merged.ui_scale || 1), 0.75, 1.5);
        merged.show_reset_time = typeof merged.show_reset_time === 'boolean' ? merged.show_reset_time : true;
        merged.auto_update = typeof merged.auto_update === 'boolean' ? merged.auto_update : false;
        merged.autostart = typeof merged.autostart === 'boolean' ? merged.autostart : false;
        merged.start_hidden = typeof merged.start_hidden === 'boolean' ? merged.start_hidden : false;
        merged.auto_show_on_codex = typeof merged.auto_show_on_codex === 'boolean' ? merged.auto_show_on_codex : false;
        merged.auto_hide_on_codex_close = typeof merged.auto_hide_on_codex_close === 'boolean' ? merged.auto_hide_on_codex_close : false;
        merged.language = merged.language === 'en' ? 'en' : 'zh';
        setConfig(merged);
      })
      .catch(() => setConfig(DEFAULT_CONFIG))
      .finally(() => setConfigReady(true));
  }, []);

  useEffect(() => {
    const onResize = () => {
      const vp = getViewportSize();
      setViewportSize(vp);
      if (!programmaticResizeRef.current) {
        // User-driven resize: remember the width so opening/closing settings
        // does not reset it back to the default width.
        userResizedRef.current = true;
        preservedWidthRef.current = vp.width;
      }
    };
    window.addEventListener('resize', onResize);
    window.setTimeout(onResize, 0);
    return () => {
      window.removeEventListener('resize', onResize);
      if (programmaticResizeTimerRef.current) {
        window.clearTimeout(programmaticResizeTimerRef.current);
      }
    };
  }, []);

  // Show/hide + always-on-top. Runs only after config is loaded so the backend
  // start_hidden hide is not immediately undone. start_hidden hides once on
  // startup; afterwards we don't auto-show from this effect (tray/codex handle it).
  useEffect(() => {
    if (!configReady) return;
    const win = getCurrentWindow();
    const wantTop = config.taskbar_strip || config.always_on_top;
    win.setAlwaysOnTop(wantTop).catch(() => undefined);
    if (!startupDoneRef.current) {
      startupDoneRef.current = true;
      if (config.start_hidden) {
        skipAutoShowRef.current = true;
        win.hide().catch(() => undefined);
        return;
      }
    }
    if (skipAutoShowRef.current) {
      return;
    }
    if (config.show_floating_window) {
      win.show().catch(() => undefined);
    } else {
      win.hide().catch(() => undefined);
    }
  }, [configReady, config.always_on_top, config.show_floating_window, config.taskbar_strip, config.start_hidden]);

  // Taskbar strip keep-alive: reassert always-on-top every 2s so the strip is
  // not occluded by the real Windows taskbar. Only in strip mode; never forces
  // a hidden window visible and never steals focus.
  useEffect(() => {
    if (!config.taskbar_strip) return;
    const win = getCurrentWindow();
    const id = window.setInterval(() => {
      win.isVisible()
        .then((visible) => {
          if (visible) win.setAlwaysOnTop(true).catch(() => undefined);
        })
        .catch(() => undefined);
    }, 2000);
    return () => window.clearInterval(id);
  }, [config.taskbar_strip]);

  // Re-assert topmost after losing focus (e.g. user clicks the Windows taskbar):
  // the taskbar can briefly cover the strip; this pushes the strip back on top.
  useEffect(() => {
    if (!config.taskbar_strip) return;
    const win = getCurrentWindow();
    let timer: number | undefined;
    const unlistenP = win.onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        if (timer) window.clearTimeout(timer);
        timer = window.setTimeout(() => {
          win.isVisible()
            .then((v) => {
              if (v) win.setAlwaysOnTop(true).catch(() => undefined);
            })
            .catch(() => undefined);
        }, 150);
      }
    });
    return () => {
      if (timer) window.clearTimeout(timer);
      unlistenP.then((u) => u()).catch(() => undefined);
    };
  }, [config.taskbar_strip]);

  useEffect(() => {
    if (!config.taskbar_strip) return;
    const win = getCurrentWindow();
    win.setAlwaysOnTop(true).catch(() => undefined);
  }, [config.taskbar_strip, settingsOpen]);

  useEffect(() => {
    if (!configReady) return;
    const baseKey = `${baseSize.width}x${baseSize.height}:${config.taskbar_strip}:${settingsOpen}:${status?.ok === false}:${userResizedRef.current}`;
    if (lastBaseKeyRef.current === baseKey) return;

    lastBaseKeyRef.current = baseKey;
    void resizeWindowToBase(baseSize);
  }, [baseSize, config.taskbar_strip, configReady, resizeWindowToBase, settingsOpen, status?.ok]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    const secs = Math.max(60, config.refresh_interval_secs || DEFAULT_CONFIG.refresh_interval_secs);
    const id = window.setInterval(() => refresh(), secs * 1000);
    return () => window.clearInterval(id);
  }, [config.refresh_interval_secs, refresh]);

  useEffect(() => {
    let cleanups: Array<() => void> = [];
    listen('meter-refresh-requested', () => refresh()).then((unlisten) => cleanups.push(unlisten));
    listen('meter-settings-requested', () => {
      openSettings();
    }).then((unlisten) => cleanups.push(unlisten));
    return () => {
      for (const cleanup of cleanups) cleanup();
    };
  }, [refresh, openSettings]);

  const saveConfig = useCallback(async (next: MeterConfig) => {
    const normalized = {
      ...next,
      source_mode: 'app_server' as const,
      ui_scale: clamp(Number(next.ui_scale || 1), 0.75, 1.5),
    };
    setConfig(normalized);
    await invoke('save_config', { config: normalized }).catch(() => undefined);
  }, []);

  const toggleSettings = useCallback(() => {
    if (settingsOpen) {
      closeSettingsNow();
    } else {
      openSettings();
    }
  }, [settingsOpen, openSettings, closeSettingsNow]);

  const closeSettings = useCallback(() => {
    closeSettingsNow();
  }, [closeSettingsNow]);

  const toggleStripMode = useCallback(() => {
    setSettingsOpen(false);
    void saveConfig({ ...config, taskbar_strip: !config.taskbar_strip, show_floating_window: true, source_mode: 'app_server' });
  }, [config, saveConfig]);

  const showNativeContextMenu = useCallback(async (event?: MouseEvent) => {
    event?.preventDefault();
    event?.stopPropagation();
    try {
      const menu = await Menu.new({
        items: [
          { id: 'refresh', text: t('menuRefresh'), action: () => void refresh() },
          { id: 'settings', text: settingsOpen ? t('menuCloseSettings') : t('menuSettings'), action: () => { if (settingsOpen) closeSettingsNow(); else openSettings(); } },
          { id: 'strip', text: config.taskbar_strip ? t('menuStripOff') : t('menuStripOn'), action: () => toggleStripMode() },
          { id: 'quit', text: t('menuQuit'), action: () => void invoke('exit_app') },
        ],
      });
      await menu.popup(undefined, getCurrentWindow());
    } catch {
      setSettingsOpen(true);
    }
  }, [config.taskbar_strip, refresh, settingsOpen, toggleStripMode, openSettings, closeSettingsNow, t]);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    listen('meter-toggle-strip-requested', () => {
      setSettingsOpen(false);
      void saveConfig({ ...config, taskbar_strip: !config.taskbar_strip, show_floating_window: true, source_mode: 'app_server' });
    }).then((unlisten) => {
      cleanup = unlisten;
    });
    return () => cleanup?.();
  }, [config, saveConfig]);

  const meterStyle = {
    opacity: config.opacity,
    ['--base-width' as string]: `${baseSize.width}px`,
    ['--base-height' as string]: `${baseSize.height}px`,
    ['--content-scale' as string]: contentScale,
  } as CSSProperties;

  if (config.taskbar_strip) {
    return (
      <main
        className={`meter strip ${settingsOpen ? 'settings-open' : ''}`}
        style={meterStyle}
        onContextMenu={showNativeContextMenu}
      >
        <div className="meter-content">
          <section className="strip-panel">
            <div className="strip-drag" onMouseDown={(event) => startWindowDrag(event)}>
              <div className="strip-lines">
                <div>
                  <b>{t('strip5h')}</b>: {pct(status?.primary?.remaining_percent ?? null)} <span>{t('stripWeek')}</span>: {pct(status?.secondary?.remaining_percent ?? null)} <span>{t('stripVoucher')}</span>: {status?.reset_credits_available ?? 0}
                  {config.show_reset_time && <span>{t('stripData')}</span>}
                  {config.show_reset_time && `: ${formatUpdated(status?.updated_at_ms ?? 0) || '--'}`}
                </div>
                <div>
                  <b>{t('stripReset')}</b>: {formatReset(status?.primary?.resets_at ?? null, status?.primary?.reset_text, 'auto', t('unknown'))} <span>{t('stripWeek')}</span>: {formatReset(status?.secondary?.resets_at ?? null, status?.secondary?.reset_text, 'date', t('unknown'))}
                  {config.show_reset_time && <span>{t('stripCredits')}</span>}
                  {config.show_reset_time && `: ${status?.credit_balance ?? '--'}`}
                </div>
              </div>
            </div>
          </section>
          {settingsOpen && (
            <section className="strip-settings" onMouseDown={stopDragEvent} onPointerDown={stopDragEvent}>
              <SettingsPanel
                config={config}
                saveConfig={saveConfig}
                onClose={closeSettings}
                lang={lang}
              />
            </section>
          )}
        </div>
      </main>
    );
  }

  return (
    <main
      className={`meter normal ${settingsOpen ? 'settings-open' : ''}`}
      style={meterStyle}
      onContextMenu={showNativeContextMenu}
    >
      <div className="meter-content">
        <section className="panel">
          <header className="titlebar" onMouseDown={(event) => startWindowDrag(event)}>
            <div className="title-drag" onMouseDown={(event) => startWindowDrag(event)}>
              <div className="name">{APP_NAME}</div>
            </div>
            <div className="title-spacer-drag" />
            <div className="actions" onMouseDown={stopDragEvent} onPointerDown={stopDragEvent}>
              <ActionButton title={t('titleRefresh')} onClick={() => refresh()} disabled={loading}>{loading ? '…' : '↻'}</ActionButton>
              <ActionButton title={t('titleStrip')} onClick={toggleStripMode}>▭</ActionButton>
              <ActionButton title={t('titleSettings')} onClick={toggleSettings}>⚙</ActionButton>
            </div>
          </header>

          {settingsOpen ? (
            <SettingsPanel
              config={config}
              saveConfig={saveConfig}
              onClose={closeSettings}
              lang={lang}
            />
          ) : !status ? (
            <div className="message drag-zone" onMouseDown={(event) => startWindowDrag(event)}>{t('loading')}</div>
          ) : status.ok ? (
            <div className="content drag-zone" onMouseDown={(event) => startWindowDrag(event)}>
              <LimitRow limit={status.primary} fallback={t('primary')} resetDateMode="auto" t={t} />
              <LimitRow limit={status.secondary} fallback={t('secondary')} resetDateMode="date" t={t} />
              <div className="row">
                <span className="label">{t('credits')}</span>
                <span className="value">{status.credit_balance ?? '--'}</span>
                <span className="sub" />
              </div>
              <div className="footer">{t('resetCredits')} {status.reset_credits_available ?? 0} · {t('data')} {formatUpdated(status.updated_at_ms) || '--'}</div>
            </div>
          ) : (
            <div className="message error drag-zone" onMouseDown={(event) => startWindowDrag(event)}>{status.message || t('failed')}</div>
          )}
        </section>
      </div>
    </main>
  );
}

type UpdateState = 'idle' | 'checking' | 'upToDate' | 'available' | 'downloading' | 'installed' | 'failed';

function SettingsPanel({
  config,
  saveConfig,
  onClose,
  lang,
}: {
  config: MeterConfig;
  saveConfig: (next: MeterConfig) => Promise<void>;
  onClose: () => void;
  lang: Language;
}) {
  const t = useMemo<TFunc>(() => (key: string) => tr(lang, key), [lang]);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [updateState, setUpdateState] = useState<UpdateState>('idle');
  const [updateVersion, setUpdateVersion] = useState('');

  useEffect(() => {
    invoke<boolean>('get_autostart_enabled')
      .then(setAutostartEnabled)
      .catch(() => undefined);
  }, []);

  const runUpdateCheck = useCallback(async () => {
    setUpdateState('checking');
    try {
      const result = await invoke<string | null>('check_for_updates');
      if (result) {
        setUpdateVersion(result);
        setUpdateState('available');
      } else {
        setUpdateState('upToDate');
      }
    } catch {
      setUpdateState('failed');
    }
  }, []);

  const downloadInstall = useCallback(async () => {
    setUpdateState('downloading');
    try {
      await invoke('download_and_install_update');
      setUpdateState('installed');
    } catch {
      setUpdateState('failed');
    }
  }, []);

  const onAutostartChange = useCallback(async (checked: boolean) => {
    setAutostartEnabled(checked);
    try {
      await invoke('set_autostart', { enabled: checked });
      void saveConfig({ ...config, autostart: checked, source_mode: 'app_server' });
    } catch {
      setAutostartEnabled(!checked);
    }
  }, [config, saveConfig]);

  const onLanguageChange = useCallback((next: Language) => {
    void saveConfig({ ...config, language: next, source_mode: 'app_server' });
    void invoke('rebuild_tray_menu', { lang: next }).catch(() => undefined);
  }, [config, saveConfig]);

  return (
    <div className="settings">
      <label>
        {t('refreshInterval')}
        <select
          value={config.refresh_interval_secs}
          onChange={(e) => saveConfig({ ...config, refresh_interval_secs: Number(e.target.value), source_mode: 'app_server' })}
        >
          <option value={60}>1 {t('minute')}</option>
          <option value={180}>3 {t('minute')}</option>
          <option value={300}>5 {t('minute')}</option>
          <option value={600}>10 {t('minute')}</option>
        </select>
      </label>
      <label>
        {t('opacity')}
        <input
          type="range"
          min="0.55"
          max="1"
          step="0.05"
          value={config.opacity}
          onChange={(e) => saveConfig({ ...config, opacity: Number(e.target.value), source_mode: 'app_server' })}
        />
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.always_on_top}
          onChange={(e) => saveConfig({ ...config, always_on_top: e.target.checked, source_mode: 'app_server' })}
        />
        {t('alwaysOnTop')}
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.taskbar_strip}
          onChange={(e) => saveConfig({ ...config, taskbar_strip: e.target.checked, source_mode: 'app_server' })}
        />
        {t('taskbarStrip')}
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={autostartEnabled}
          onChange={(e) => onAutostartChange(e.target.checked)}
        />
        {t('autostart')}
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.start_hidden}
          onChange={(e) => saveConfig({ ...config, start_hidden: e.target.checked, source_mode: 'app_server' })}
        />
        {t('startHidden')}
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.auto_show_on_codex}
          onChange={(e) => saveConfig({ ...config, auto_show_on_codex: e.target.checked, source_mode: 'app_server' })}
        />
        {t('autoShowCodex')}
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.auto_hide_on_codex_close}
          onChange={(e) => saveConfig({ ...config, auto_hide_on_codex_close: e.target.checked, source_mode: 'app_server' })}
        />
        {t('autoHideCodex')}
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.auto_update}
          onChange={(e) => saveConfig({ ...config, auto_update: e.target.checked, source_mode: 'app_server' })}
        />
        {t('autoUpdate')}
      </label>
      <div className="update-check-row">
        <button
          className="settings-button"
          type="button"
          onClick={() => void runUpdateCheck()}
          disabled={updateState === 'checking' || updateState === 'downloading'}
        >
          {updateState === 'checking' ? t('checking') : t('checkUpdate')}
        </button>
        <span className="update-status">
          {updateState === 'upToDate' && t('updateUpToDate')}
          {updateState === 'available' && `${t('updateFound')}：v${updateVersion}`}
          {updateState === 'downloading' && t('updatingDownload')}
          {updateState === 'installed' && t('updateDone')}
          {updateState === 'failed' && t('updateFailed')}
        </span>
      </div>
      {updateState === 'available' && (
        <button className="settings-button" type="button" onClick={() => void downloadInstall()}>
          {t('downloadInstall')}
        </button>
      )}
      {updateState === 'installed' && (
        <button className="settings-button" type="button" onClick={() => void invoke('restart_app')}>
          {t('restart')}
        </button>
      )}
      {updateState === 'failed' && (
        <button className="settings-button" type="button" onClick={() => void invoke('open_project_url', { url: GITHUB_RELEASES })}>
          {t('releasePage')}
        </button>
      )}
      <label className="check">
        <input
          type="checkbox"
          checked={config.show_reset_time}
          onChange={(e) => saveConfig({ ...config, show_reset_time: e.target.checked, source_mode: 'app_server' })}
        />
        {t('showCreditsData')}
      </label>
      <label>
        {t('language')}
        <select
          value={config.language}
          onChange={(e) => onLanguageChange(e.target.value as Language)}
        >
          <option value="zh">{t('langZh')}</option>
          <option value="en">{t('langEn')}</option>
        </select>
      </label>
      {config.taskbar_strip && (
        <button className="settings-button" type="button" onClick={() => saveConfig({ ...config, taskbar_strip: false, source_mode: 'app_server' })}>
          {t('backToFloat')}
        </button>
      )}
      <button className="settings-button secondary" type="button" onClick={onClose}>
        {t('closeSettings')}
      </button>
      <div className="about">
        <img className="about-logo" src={logoImg} alt="LX Codex Meter" />
        <div><strong>{APP_NAME}</strong> <span>v{APP_VERSION}</span></div>
        <div>{t('author')}：{APP_AUTHOR}</div>
        <div>GitHub: <a href={GITHUB_URL} onClick={(e) => { e.preventDefault(); void invoke('open_project_url', { url: GITHUB_URL }).catch(() => undefined); }}>{GITHUB_URL}</a></div>
        <div>Gitee: <a href={GITEE_URL} onClick={(e) => { e.preventDefault(); void invoke('open_project_url', { url: GITEE_URL }).catch(() => undefined); }}>{GITEE_URL}</a></div>
      </div>
      <div className="support-section">
        <div className="support-title">{t('supportTitle')}</div>
        <p className="support-desc">{t('supportDesc')}</p>
        <div className="support-qrs">
          <div className="qr-item">
            <img src={donationQr} alt={t('donation')} />
            <span>{t('donation')}</span>
          </div>
          <div className="qr-item">
            <img src={wechatQr} alt={t('addFriend')} />
            <span>{t('addFriend')}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
