import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ButtonHTMLAttributes, CSSProperties, MouseEvent, SyntheticEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Menu } from '@tauri-apps/api/menu';
import type { CodexMeterStatus, LimitWindow, MeterConfig } from './types';

const APP_NAME = 'LX Codex Meter';
const APP_VERSION = '0.6.11';
const APP_AUTHOR = 'lttlz';
const GITHUB_URL = 'https://github.com/lttlz/LXCodexMeter';

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
};

function clamp(n: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, n));
}

function getBaseWindowSize(
  strip: boolean,
  open: boolean,
  statusOk: boolean | null | undefined,
  stripWidth = 292,
): WindowBaseSize {
  if (strip) {
    return {
      width: open ? 215 : stripWidth,
      height: open ? 430 : 34,
    };
  }
  return {
    width: open ? 215 : 215,
    height: open ? 470 : statusOk === false ? 150 : 178,
  };
}

function getViewportSize(): WindowBaseSize {
  return {
    width: Math.max(1, Math.round(window.innerWidth || 1)),
    height: Math.max(1, Math.round(window.innerHeight || 1)),
  };
}

function formatReset(seconds: number | null, text?: string | null, dateMode: 'auto' | 'date' | 'time' = 'auto'): string {
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
  return '未知';
}

function formatUpdated(ms: number): string {
  if (!ms) return '未刷新';
  try {
    return new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return '未刷新';
  }
}

function pct(n: number | null): string {
  if (n === null || Number.isNaN(n)) return '--';
  return `${Math.max(0, Math.min(100, Math.round(n)))}%`;
}

function limitTitle(limit: LimitWindow | null, fallback: string): string {
  if (!limit) return fallback;
  const mins = limit.window_duration_mins;
  if (mins === 300) return '5小时';
  if (mins === 10080) return '周额度';
  if (mins === 60) return '1小时';
  if (mins === 15) return '15分钟';
  if (mins && mins > 60 && mins < 1440) return `${Math.round(mins / 60)}小时`;
  if (mins && mins >= 1440) return `${Math.round(mins / 1440)}天`;
  return limit.label || fallback;
}

function LimitRow({
  limit,
  fallback,
  resetDateMode = 'auto',
}: {
  limit: LimitWindow | null;
  fallback: string;
  resetDateMode?: 'auto' | 'date' | 'time';
}) {
  const remaining = limit?.remaining_percent ?? null;
  const used = limit?.used_percent ?? null;
  return (
    <div className="row">
      <span className="label">{limitTitle(limit, fallback)}</span>
      <span className="value">{pct(remaining)}</span>
      <span className="sub">用 {pct(used)} · 重置 {formatReset(limit?.resets_at ?? null, limit?.reset_text, resetDateMode)}</span>
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
  const [stripContentWidth, setStripContentWidth] = useState(292);
  const programmaticResizeRef = useRef(false);
  const programmaticResizeTimerRef = useRef<number | undefined>(undefined);
  const lastBaseKeyRef = useRef('');
  const stripLinesRef = useRef<HTMLDivElement | null>(null);

  const baseSize = useMemo(
    () => getBaseWindowSize(config.taskbar_strip, settingsOpen, status?.ok, stripContentWidth),
    [config.taskbar_strip, settingsOpen, status?.ok, stripContentWidth],
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

    // Windows/WebView2 occasionally keeps one old transparent frame after shrinking.
    // Apply the same logical size once more after the webview has observed the new viewport.
    programmaticResizeTimerRef.current = window.setTimeout(() => {
      void win.setSize(new LogicalSize(width, height)).catch(() => undefined);
      setViewportSize(getViewportSize());
      programmaticResizeRef.current = false;
      programmaticResizeTimerRef.current = undefined;
    }, 90);
  }, []);

  const openSettings = useCallback(() => {
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
        setConfig(merged);
      })
      .catch(() => setConfig(DEFAULT_CONFIG))
      .finally(() => setConfigReady(true));
  }, []);

  useEffect(() => {
    const onResize = () => {
      setViewportSize(getViewportSize());
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

  useEffect(() => {
    const win = getCurrentWindow();
    win.setAlwaysOnTop(config.always_on_top).catch(() => undefined);
    if (config.show_floating_window) {
      win.show().catch(() => undefined);
    } else {
      win.hide().catch(() => undefined);
    }
  }, [config.always_on_top, config.show_floating_window]);

  useEffect(() => {
    if (!configReady) return;
    const baseKey = `${baseSize.width}x${baseSize.height}:${config.taskbar_strip}:${settingsOpen}:${status?.ok === false}`;
    if (lastBaseKeyRef.current === baseKey) return;

    lastBaseKeyRef.current = baseKey;
    void resizeWindowToBase(baseSize);
  }, [baseSize, config.taskbar_strip, configReady, resizeWindowToBase, settingsOpen, status?.ok]);

  useEffect(() => {
    if (!config.taskbar_strip || settingsOpen) return;
    const frameId = window.requestAnimationFrame(() => {
      const lines = stripLinesRef.current;
      if (!lines) return;
      // Use scrollWidth first: getBoundingClientRect() includes parent transform scale,
      // which made the measured strip width chase the old transparent window width.
      const contentWidth = Math.ceil(lines.scrollWidth || lines.offsetWidth || lines.getBoundingClientRect().width || 0);
      const measured = contentWidth + 12;
      const nextWidth = Math.max(150, measured);
      setStripContentWidth((current) => (Math.abs(current - nextWidth) > 1 ? nextWidth : current));
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [config.show_reset_time, config.taskbar_strip, settingsOpen, status]);

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
          { id: 'refresh', text: '刷新', action: () => void refresh() },
          { id: 'settings', text: settingsOpen ? '关闭设置' : '设置', action: () => { if (settingsOpen) closeSettingsNow(); else openSettings(); } },
          { id: 'strip', text: config.taskbar_strip ? '切回默认悬浮窗' : '切换任务栏条模式', action: () => toggleStripMode() },
          { id: 'quit', text: '退出', action: () => void invoke('exit_app') },
        ],
      });
      await menu.popup(undefined, getCurrentWindow());
    } catch {
      // Fallback: if native popup is blocked by permissions/runtime, open settings instead of showing a clipped web menu.
      setSettingsOpen(true);
    }
  }, [config.taskbar_strip, refresh, settingsOpen, toggleStripMode, openSettings, closeSettingsNow]);

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
              <div className="strip-lines" ref={stripLinesRef}>
                <div>
                  <b>5h</b>: {pct(status?.primary?.remaining_percent ?? null)} <span>周</span>: {pct(status?.secondary?.remaining_percent ?? null)}
                  {config.show_reset_time && <span>刷新</span>}
                  {config.show_reset_time && `: ${formatUpdated(status?.updated_at_ms ?? 0)}`}
                </div>
                <div>
                  <b>重置</b>: {formatReset(status?.primary?.resets_at ?? null, status?.primary?.reset_text, 'auto')} <span>周</span>: {formatReset(status?.secondary?.resets_at ?? null, status?.secondary?.reset_text, 'date')}
                  {config.show_reset_time && <span>Credits</span>}
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
              <div className="name">LX Codex Meter</div>
            </div>
            <div className="title-spacer-drag" />
            <div className="actions" onMouseDown={stopDragEvent} onPointerDown={stopDragEvent}>
              <ActionButton title="刷新" onClick={() => refresh()} disabled={loading}>{loading ? '…' : '↻'}</ActionButton>
              <ActionButton title="切换任务栏条模式" onClick={toggleStripMode}>▭</ActionButton>
              <ActionButton title="设置" onClick={toggleSettings}>⚙</ActionButton>
            </div>
          </header>

          {!status ? (
            <div className="message drag-zone" onMouseDown={(event) => startWindowDrag(event)}>正在读取 Codex 额度…</div>
          ) : status.ok ? (
            <div className="content drag-zone" onMouseDown={(event) => startWindowDrag(event)}>
              <LimitRow limit={status.primary} fallback="主额度" resetDateMode="auto" />
              <LimitRow limit={status.secondary} fallback="副额度" resetDateMode="date" />
              <div className="row">
                <span className="label">Credits</span>
                <span className="value">{status.credit_balance ?? '--'}</span>
                <span className="sub" />
              </div>
              <div className="footer">重置券 {status.reset_credits_available ?? 0} · 刷新 {formatUpdated(status.updated_at_ms)}</div>
            </div>
          ) : (
            <div className="message error drag-zone" onMouseDown={(event) => startWindowDrag(event)}>{status.message || '读取失败'}</div>
          )}

          {settingsOpen && (
            <SettingsPanel
              config={config}
              saveConfig={saveConfig}
              onClose={closeSettings}
            />
          )}
        </section>
      </div>
    </main>
  );
}

function SettingsPanel({
  config,
  saveConfig,
  onClose,
}: {
  config: MeterConfig;
  saveConfig: (next: MeterConfig) => Promise<void>;
  onClose: () => void;
}) {
  return (
    <div className="settings">
      <label>
        刷新间隔
        <select
          value={config.refresh_interval_secs}
          onChange={(e) => saveConfig({ ...config, refresh_interval_secs: Number(e.target.value), source_mode: 'app_server' })}
        >
          <option value={60}>1 分钟</option>
          <option value={180}>3 分钟</option>
          <option value={300}>5 分钟</option>
          <option value={600}>10 分钟</option>
        </select>
      </label>
      <label>
        透明度
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
        置顶显示
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.taskbar_strip}
          onChange={(e) => saveConfig({ ...config, taskbar_strip: e.target.checked, source_mode: 'app_server' })}
        />
        任务栏条模式，伪嵌入
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.auto_update}
          onChange={(e) => saveConfig({ ...config, auto_update: e.target.checked, source_mode: 'app_server' })}
        />
        自动更新
      </label>
      <label className="check">
        <input
          type="checkbox"
          checked={config.show_reset_time}
          onChange={(e) => saveConfig({ ...config, show_reset_time: e.target.checked, source_mode: 'app_server' })}
        />
        显示 Credits / 刷新
      </label>
      {config.taskbar_strip && (
        <button className="settings-button" type="button" onClick={() => saveConfig({ ...config, taskbar_strip: false, source_mode: 'app_server' })}>
          切回默认悬浮窗
        </button>
      )}
      <button className="settings-button secondary" type="button" onClick={onClose}>
        关闭设置
      </button>
      <div className="about">
        <div><strong>{APP_NAME}</strong> <span>v{APP_VERSION}</span></div>
        <div>作者：{APP_AUTHOR}</div>
        <div>GitHub: <a href={GITHUB_URL} onClick={(e) => { e.preventDefault(); void invoke('open_project_url').catch(() => undefined); }}>{GITHUB_URL}</a></div>
      </div>
    </div>
  );
}
