import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ButtonHTMLAttributes, CSSProperties, MouseEvent, Ref, SyntheticEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LogicalPosition, LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
import { currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
import { Menu } from '@tauri-apps/api/menu';
import type { CodexMeterStatus, Language, LimitWindow, MeterConfig, ThemeMode } from './types';
import { tr } from './i18n';
import UsageLogPage from './UsageLogPage';
import donationQr from './assets/support/donation-qr.png';
import wechatQr from './assets/support/wechat-qr.png';
import logoImg from './assets/logo.png';

const APP_NAME = 'LX Codex Meter';
const APP_VERSION = '0.6.15';
const APP_AUTHOR = 'lttlz';
const GITHUB_URL = 'https://github.com/lttlz/LXCodexMeter';
const GITEE_URL = 'https://gitee.com/lttlz/LXCodexMeter';
const GITHUB_RELEASES = 'https://github.com/lttlz/LXCodexMeter/releases';
const FLOATING_LAYOUT_BASE_WIDTH = 215;
const STRIP_LAYOUT_BASE_WIDTH = 246;
const DEFAULT_WINDOW_WIDTH = FLOATING_LAYOUT_BASE_WIDTH;
const MIN_WINDOW_WIDTH = 150;
const DEFAULT_SETTINGS_HEIGHT = 660;
const FLOATING_OK_LAYOUT_BASE_HEIGHT = 142;
const FLOATING_ERROR_LAYOUT_BASE_HEIGHT = 150;
const STRIP_LAYOUT_BASE_HEIGHT = 30;
const TASKBAR_SAFE_GAP = 0;
const WINDOW_MOVE_DEBOUNCE_MS = 100;

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
  theme: 'system',
};

function clamp(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, n));
}

function normalizeWindowWidth(width: number, fallback = DEFAULT_WINDOW_WIDTH): number {
  const next = Number.isFinite(width) ? Math.round(width) : fallback;
  return Math.max(MIN_WINDOW_WIDTH, next > 1 ? next : fallback);
}

function getInitialUserWindowWidth(): number {
  return normalizeWindowWidth(Math.round(window.innerWidth || DEFAULT_WINDOW_WIDTH));
}

function getLayoutBaseWidth(strip: boolean): number {
  return strip ? STRIP_LAYOUT_BASE_WIDTH : FLOATING_LAYOUT_BASE_WIDTH;
}

function getLayoutBaseHeight(
  strip: boolean,
  open: boolean,
  statusOk: boolean | null | undefined,
  settingsHeight: number | null,
): number {
  if (open) return settingsHeight ?? DEFAULT_SETTINGS_HEIGHT;
  if (strip) return STRIP_LAYOUT_BASE_HEIGHT;
  return statusOk === false ? FLOATING_ERROR_LAYOUT_BASE_HEIGHT : FLOATING_OK_LAYOUT_BASE_HEIGHT;
}

function getAvailableWindowHeight(): number {
  const availableHeight = Math.round(window.screen?.availHeight || 0);
  if (availableHeight <= 0) return Number.MAX_SAFE_INTEGER;
  const top = Number.isFinite(window.screenY) ? Math.max(0, Math.round(window.screenY)) : 0;
  return Math.max(1, availableHeight - top - 8);
}

function getTargetWindowSize(
  userWindowWidth: number,
  layoutBaseHeight: number,
  contentScale: number,
  maxHeight: number,
): WindowBaseSize {
  const targetHeight = Math.max(1, Math.ceil(layoutBaseHeight * contentScale));
  return {
    width: normalizeWindowWidth(userWindowWidth),
    height: Math.min(targetHeight, Math.max(1, maxHeight)),
  };
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

// Data refresh time: HH:mm:ss. Empty string when not refreshed.
function formatUpdated(ms: number): string {
  if (!ms) return '';
  try {
    return new Date(ms).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch {
    return '';
  }
}

function pct(n: number | null): string {
  if (n === null || Number.isNaN(n)) return '--';
  return `${Math.max(0, Math.min(100, Math.round(n)))}%`;
}

type TFunc = (key: string) => string;

function LimitRow({
  limit,
  title,
  resetDateMode = 'auto',
  t,
}: {
  limit: LimitWindow | null;
  title: string;
  resetDateMode?: 'auto' | 'date' | 'time';
  t: TFunc;
}) {
  const remaining = limit?.remaining_percent ?? null;
  const used = limit?.used_percent ?? null;
  return (
    <div className="row">
      <span className="label">{title}</span>
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
  const [settingsOpen, setSettingsOpenState] = useState(false);
  const [viewportSize, setViewportSize] = useState<WindowBaseSize>(() => getViewportSize());
  const [userWindowWidth, setUserWindowWidth] = useState(() => getInitialUserWindowWidth());
  const [settingsContentHeight, setSettingsContentHeight] = useState<number | null>(null);
  const programmaticResizeRef = useRef(false);
  const programmaticResizeTimerRef = useRef<number | undefined>(undefined);
  const settingsOpenRef = useRef(settingsOpen);
  const lastBaseKeyRef = useRef('');
  const userWindowWidthRef = useRef<number | null>(null);
  const titlebarRef = useRef<HTMLElement | null>(null);
  const stripPanelRef = useRef<HTMLElement | null>(null);
  const settingsPanelRef = useRef<HTMLDivElement | null>(null);
  const startupDoneRef = useRef(false);
  const refreshInFlightRef = useRef(false);
  const setSettingsOpen = useCallback((open: boolean) => {
    settingsOpenRef.current = open;
    setSettingsOpenState(open);
  }, []);
  if (userWindowWidthRef.current === null) {
    userWindowWidthRef.current = userWindowWidth;
  }

  const lang = config.language;
  const t = useMemo<TFunc>(() => (key: string) => tr(lang, key), [lang]);

  const layoutBaseWidth = getLayoutBaseWidth(config.taskbar_strip);
  const layoutBaseHeight = getLayoutBaseHeight(
    config.taskbar_strip,
    settingsOpen,
    status?.ok,
    settingsContentHeight,
  );
  const contentScale = clamp(userWindowWidth / layoutBaseWidth, 0.1, 10);
  const maxHeight = settingsOpen ? getAvailableWindowHeight() : Number.MAX_SAFE_INTEGER;
  const targetWindowSize = getTargetWindowSize(
    userWindowWidth,
    layoutBaseHeight,
    contentScale,
    maxHeight,
  );
  const contentViewportHeight = Math.max(
    1,
    Math.min(targetWindowSize.height, viewportSize.height) / contentScale,
  );

  const clampNormalWindowAboveTaskbar = useCallback(async () => {
    if (settingsOpenRef.current) return;

    const win = getCurrentWindow();
    const visible = await win.isVisible().catch(() => false);
    if (!visible || settingsOpenRef.current) return;

    try {
      const [outerPosition, innerPosition, innerSize, monitor] = await Promise.all([
        win.outerPosition(),
        win.innerPosition(),
        win.innerSize(),
        currentMonitor(),
      ]);

      if (settingsOpenRef.current) return;
      if (monitor) {
        const workAreaBottom = monitor.workArea.position.y + monitor.workArea.size.height;
        const targetVisibleBottom = workAreaBottom - TASKBAR_SAFE_GAP;
        const visibleBottom = innerPosition.y + innerSize.height;
        if (visibleBottom > targetVisibleBottom && !settingsOpenRef.current) {
          const overflow = visibleBottom - targetVisibleBottom;
          const nextOuterY = outerPosition.y - overflow;
          await win
            .setPosition(new PhysicalPosition(outerPosition.x, nextOuterY))
            .catch(() => undefined);
        }
        return;
      }
    } catch {
      // Fall back to WebView screen coordinates when monitor information is unavailable.
    }

    if (settingsOpenRef.current) return;
    const fallbackScreen = window.screen as Screen & { availTop?: number };
    const availableTop = Math.round(fallbackScreen.availTop || 0);
    const availableHeight = Math.round(fallbackScreen.availHeight || 0);
    const currentX = Math.round(window.screenX);
    const currentY = Math.round(window.screenY);
    const windowHeight = Math.round(window.outerHeight || window.innerHeight || 0);
    if (
      availableHeight <= 0
      || windowHeight <= 0
      || !Number.isFinite(currentX)
      || !Number.isFinite(currentY)
    ) return;

    const maxBottom = availableTop + availableHeight - TASKBAR_SAFE_GAP;
    if (currentY + windowHeight > maxBottom && !settingsOpenRef.current) {
      await win
        .setPosition(new LogicalPosition(currentX, maxBottom - windowHeight))
        .catch(() => undefined);
    }
  }, []);

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

    // Windows/WebView2 occasionally keeps one old transparent frame after shrinking.
    // Apply the same logical size once more after the webview has observed the new viewport.
    programmaticResizeTimerRef.current = window.setTimeout(async () => {
      await win.setSize(new LogicalSize(width, height)).catch(() => undefined);
      setViewportSize(getViewportSize());
      programmaticResizeRef.current = false;
      programmaticResizeTimerRef.current = undefined;
      await clampNormalWindowAboveTaskbar();
    }, 90);
  }, [clampNormalWindowAboveTaskbar]);

  const syncUserWindowWidth = useCallback((width: number) => {
    const nextWidth = normalizeWindowWidth(width, userWindowWidthRef.current ?? DEFAULT_WINDOW_WIDTH);
    if (Math.abs((userWindowWidthRef.current ?? 0) - nextWidth) <= 1) return;
    userWindowWidthRef.current = nextWidth;
    setUserWindowWidth(nextWidth);
  }, []);

  const measureSettingsHeight = useCallback(() => {
    if (!settingsOpen) return;
    const settings = settingsPanelRef.current;
    if (!settings) return;
    const header = config.taskbar_strip ? stripPanelRef.current : titlebarRef.current;
    const headerHeight = Math.ceil(header?.offsetHeight || (config.taskbar_strip ? 40 : 0));
    const settingsHeight = Math.ceil(settings.scrollHeight || settings.offsetHeight || 0);
    const nextHeight = Math.max(1, headerHeight + settingsHeight);
    setSettingsContentHeight((current) => (current !== null && Math.abs(current - nextHeight) <= 1 ? current : nextHeight));
  }, [config.taskbar_strip, settingsOpen]);

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
    if (refreshInFlightRef.current) return;
    refreshInFlightRef.current = true;

    const visible = await getCurrentWindow().isVisible().catch(() => false);
    if (!visible) {
      refreshInFlightRef.current = false;
      return;
    }

    setLoading(true);
    try {
      const next = await invoke<CodexMeterStatus | null>('get_status', {
        mode: 'app_server',
        clientText: '',
      });
      if (next !== null) {
        setStatus(next);
      }
    } catch (error) {
      setStatus({
        ok: false,
        message: error instanceof Error ? error.message : String(error),
        source_mode: 'app_server',
        auth_mode: null,
        plan_type: null,
        primary: null,
        secondary: null,
        five_hour: null,
        weekly: null,
        credit_balance: null,
        credit_limit: null,
        reset_credits_available: null,
        updated_at_ms: Date.now(),
      });
    } finally {
      setLoading(false);
      refreshInFlightRef.current = false;
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
        merged.theme = merged.theme === 'light' || merged.theme === 'dark' ? merged.theme : 'system';
        setConfig(merged);
      })
      .catch(() => setConfig(DEFAULT_CONFIG))
      .finally(() => setConfigReady(true));
  }, []);

  useEffect(() => {
    const onResize = () => {
      const nextViewportSize = getViewportSize();
      setViewportSize(nextViewportSize);
      if (!programmaticResizeRef.current) {
        syncUserWindowWidth(nextViewportSize.width);
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
  }, [syncUserWindowWidth]);

  useEffect(() => {
    if (settingsOpen) return;

    const win = getCurrentWindow();
    let disposed = false;
    let moveTimer: number | undefined;
    let unlisten: (() => void) | undefined;

    void win.onMoved(() => {
      if (disposed || settingsOpenRef.current) return;
      if (moveTimer !== undefined) window.clearTimeout(moveTimer);
      moveTimer = window.setTimeout(() => {
        moveTimer = undefined;
        if (!disposed) void clampNormalWindowAboveTaskbar();
      }, WINDOW_MOVE_DEBOUNCE_MS);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch(() => undefined);

    return () => {
      disposed = true;
      if (moveTimer !== undefined) window.clearTimeout(moveTimer);
      unlisten?.();
    };
  }, [clampNormalWindowAboveTaskbar, settingsOpen]);

  useEffect(() => {
    if (!settingsOpen) {
      setSettingsContentHeight(null);
      return undefined;
    }

    const frameId = window.requestAnimationFrame(measureSettingsHeight);
    const timeoutId = window.setTimeout(measureSettingsHeight, 0);
    const observer = typeof ResizeObserver === 'undefined' ? undefined : new ResizeObserver(measureSettingsHeight);
    if (observer) {
      const header = config.taskbar_strip ? stripPanelRef.current : titlebarRef.current;
      if (settingsPanelRef.current) observer.observe(settingsPanelRef.current);
      if (header) observer.observe(header);
    }
    window.addEventListener('resize', measureSettingsHeight);

    return () => {
      window.cancelAnimationFrame(frameId);
      window.clearTimeout(timeoutId);
      observer?.disconnect();
      window.removeEventListener('resize', measureSettingsHeight);
    };
  }, [config.taskbar_strip, measureSettingsHeight, settingsOpen]);

  // Always-on-top sync plus one-time autostart hiding. Normal launches are
  // already visible from tauri.conf.json; this effect must not re-show a window
  // the user has just hidden while the async startup check is still pending.
  useEffect(() => {
    if (!configReady) return;
    const win = getCurrentWindow();
    const wantTop = config.taskbar_strip || config.always_on_top;
    win.setAlwaysOnTop(wantTop).catch(() => undefined);
    if (!startupDoneRef.current) {
      startupDoneRef.current = true;
      invoke<boolean>('should_start_hidden')
        .then((hidden) => {
          if (hidden) {
            void win.hide();
          }
        })
        .catch(() => undefined);
      return;
    }
  }, [configReady, config.always_on_top, config.taskbar_strip]);

  // Re-assert topmost right after the settings panel toggles, because resizing
  // the window can occasionally drop the always-on-top flag on Windows.
  useEffect(() => {
    if (!config.taskbar_strip) return;
    const win = getCurrentWindow();
    win.setAlwaysOnTop(true).catch(() => undefined);
  }, [config.taskbar_strip, settingsOpen]);

  useEffect(() => {
    if (!configReady) return;
    const baseKey = `${targetWindowSize.width}x${targetWindowSize.height}:${config.taskbar_strip}:${settingsOpen}:${status?.ok === false}`;
    if (lastBaseKeyRef.current === baseKey) return;

    lastBaseKeyRef.current = baseKey;
    void resizeWindowToBase(targetWindowSize);
  }, [config.taskbar_strip, configReady, resizeWindowToBase, settingsOpen, status?.ok, targetWindowSize]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    let cleanups: Array<() => void> = [];
    listen('meter-refresh-requested', () => refresh()).then((unlisten) => cleanups.push(unlisten));
    listen<CodexMeterStatus>('meter-status-updated', (event) => {
      setStatus(event.payload);
    }).then((unlisten) => cleanups.push(unlisten));
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

  const hideToTray = useCallback(() => {
    void invoke('hide_to_tray').catch((error) => {
      console.error('hide_to_tray failed', error);
    });
  }, []);

  const showNativeContextMenu = useCallback(async (event?: MouseEvent) => {
    event?.preventDefault();
    event?.stopPropagation();
    try {
      const menu = await Menu.new({
        items: [
          { id: 'refresh', text: t('menuRefresh'), action: () => void refresh() },
          { id: 'settings', text: settingsOpen ? t('menuCloseSettings') : t('menuSettings'), action: () => { if (settingsOpen) closeSettingsNow(); else openSettings(); } },
          { id: 'strip', text: config.taskbar_strip ? t('menuStripOff') : t('menuStripOn'), action: () => toggleStripMode() },
          { id: 'hideTray', text: t('menuHideTray'), action: hideToTray },
          { id: 'quit', text: t('menuQuit'), action: () => void invoke('exit_app') },
        ],
      });
      await menu.popup(undefined, getCurrentWindow());
    } catch {
      // Fallback: if native popup is blocked by permissions/runtime, open settings instead of showing a clipped web menu.
      setSettingsOpen(true);
    }
  }, [config.taskbar_strip, refresh, settingsOpen, toggleStripMode, hideToTray, openSettings, closeSettingsNow, t]);

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
    ['--base-width' as string]: `${layoutBaseWidth}px`,
    ['--base-height' as string]: `${layoutBaseHeight}px`,
    ['--content-viewport-height' as string]: `${contentViewportHeight}px`,
    ['--content-scale' as string]: contentScale,
  } as CSSProperties;
  const themeClass = config.theme === 'light' ? 'theme-light' : config.theme === 'dark' ? 'theme-dark' : 'theme-system';

  if (config.taskbar_strip) {
    return (
      <main
        className={`meter strip ${themeClass} ${settingsOpen ? 'settings-open' : ''}`}
        style={meterStyle}
        onContextMenu={showNativeContextMenu}
      >
        <div className="meter-content">
          <section className="strip-panel" ref={stripPanelRef}>
            <div className="strip-drag" onMouseDown={(event) => startWindowDrag(event)}>
              <div className="strip-lines">
                <div>
                  <b>{t('strip5h')}</b>: {pct(status?.five_hour?.remaining_percent ?? null)} <span>{t('stripWeek')}</span>: {pct(status?.weekly?.remaining_percent ?? null)} <span>{t('stripVoucher')}</span>: {status?.reset_credits_available ?? 0}
                  {config.show_reset_time && <span>{t('stripData')}</span>}
                  {config.show_reset_time && `: ${formatUpdated(status?.updated_at_ms ?? 0) || '--'}`}
                </div>
                <div>
                  <b>{t('stripReset')}</b>: {formatReset(status?.five_hour?.resets_at ?? null, status?.five_hour?.reset_text, 'auto', t('unknown'))} <span>{t('stripWeek')}</span>: {formatReset(status?.weekly?.resets_at ?? null, status?.weekly?.reset_text, 'date', t('unknown'))}
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
                rootRef={settingsPanelRef}
              />
            </section>
          )}
        </div>
      </main>
    );
  }

  return (
    <main
      className={`meter normal ${themeClass} ${settingsOpen ? 'settings-open' : ''}`}
      style={meterStyle}
      onContextMenu={showNativeContextMenu}
    >
      <div className="meter-content">
        <section className="panel">
          <header className="titlebar" ref={titlebarRef} onMouseDown={(event) => startWindowDrag(event)}>
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
              rootRef={settingsPanelRef}
            />
          ) : !status ? (
            <div className="message drag-zone" onMouseDown={(event) => startWindowDrag(event)}>{t('loading')}</div>
          ) : status.ok ? (
            <div className="content drag-zone" onMouseDown={(event) => startWindowDrag(event)}>
              <LimitRow limit={status.five_hour} title={t('strip5h')} resetDateMode="auto" t={t} />
              <LimitRow limit={status.weekly} title={t('secondary')} resetDateMode="date" t={t} />
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
  rootRef,
}: {
  config: MeterConfig;
  saveConfig: (next: MeterConfig) => Promise<void>;
  onClose: () => void;
  lang: Language;
  rootRef?: Ref<HTMLDivElement>;
}) {
  const t = useMemo<TFunc>(() => (key: string) => tr(lang, key), [lang]);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [updateState, setUpdateState] = useState<UpdateState>('idle');
  const [updateVersion, setUpdateVersion] = useState('');
  const [section, setSection] = useState<'settings' | 'usage'>('settings');

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
    <div className="settings" ref={rootRef}>
      <nav className="settings-tabs" aria-label={t('settingsTab')}>
        <button className={section === 'settings' ? 'active' : ''} type="button" onClick={() => setSection('settings')}>
          {t('settingsTab')}
        </button>
        <button className={section === 'usage' ? 'active' : ''} type="button" onClick={() => setSection('usage')}>
          {t('usageLogTab')}
        </button>
      </nav>
      {section === 'usage' ? (
        <UsageLogPage lang={lang} />
      ) : (
        <>
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
      <label>
        {t('theme')}
        <select
          value={config.theme}
          onChange={(e) => saveConfig({ ...config, theme: e.target.value as ThemeMode, source_mode: 'app_server' })}
        >
          <option value="system">{t('themeSystem')}</option>
          <option value="light">{t('themeLight')}</option>
          <option value="dark">{t('themeDark')}</option>
        </select>
      </label>
      <button className="settings-button secondary" type="button" onClick={onClose}>
        {t('closeSettings')}
      </button>
      {config.taskbar_strip && (
        <button className="settings-button" type="button" onClick={() => saveConfig({ ...config, taskbar_strip: false, source_mode: 'app_server' })}>
          {t('backToFloat')}
        </button>
      )}
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
        </>
      )}
    </div>
  );
}
