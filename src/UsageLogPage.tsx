import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import ConfirmDialog from './ConfirmDialog';
import { tr } from './i18n';
import {
  createUsageCsvRows,
  filterAndSortUsageTasks,
  formatUsageTimeRange,
  usageCsvFileName,
} from './usageLogUtils.js';
import type {
  Language,
  UsageLogPreferences,
  UsageLogView,
  UsageSortMode,
  UsageTask,
  UsageTimeFilter,
  WeeklyUsageFilter,
} from './types';

const DEFAULT_PREFERENCES: UsageLogPreferences = {
  weeklyFilter: 'gte3',
  customThreshold: 3,
  timeFilter: '30d',
  sortMode: 'latest',
};
const PAGE_SIZE = 100;

function formatPercent(value: number | null): string {
  if (value === null) return '--';
  if (value > 0 && value < 0.05) return '<0.1%';
  return `${value.toFixed(1)}%`;
}

function formatDuration(seconds: number, lang: Language): string {
  const total = Math.max(0, Math.floor(seconds));
  if (total < 60) return lang === 'zh' ? `${total}秒` : `${total}s`;
  const minutes = Math.floor(total / 60);
  const remainSeconds = total % 60;
  if (minutes < 60) {
    return lang === 'zh' ? `${minutes}分${remainSeconds}秒` : `${minutes}m ${remainSeconds}s`;
  }
  const hours = Math.floor(minutes / 60);
  const remainMinutes = minutes % 60;
  return lang === 'zh' ? `${hours}小时${remainMinutes}分` : `${hours}h ${remainMinutes}m`;
}

type PendingConfirmation =
  | { type: 'delete'; task: UsageTask }
  | { type: 'clear' };

export default function UsageLogPage({ lang }: { lang: Language }) {
  const t = useMemo(() => (key: string) => tr(lang, key), [lang]);
  const [view, setView] = useState<UsageLogView | null>(null);
  const [preferences, setPreferences] = useState(DEFAULT_PREFERENCES);
  const [customInput, setCustomInput] = useState('3.0');
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [exporting, setExporting] = useState(false);
  const [confirmation, setConfirmation] = useState<PendingConfirmation | null>(null);
  const [confirmBusy, setConfirmBusy] = useState(false);
  const confirmBusyRef = useRef(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError('');
    setNotice('');
    try {
      const next = await invoke<UsageLogView>('get_usage_log');
      setView(next);
      setPreferences(next.preferences);
      setCustomInput(next.preferences.customThreshold.toFixed(1));
    } catch {
      setError(t('usageUnavailable'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setVisibleCount(PAGE_SIZE);
  }, [preferences]);

  const savePreferences = useCallback((next: UsageLogPreferences) => {
    setPreferences(next);
    void invoke('save_usage_log_preferences', { preferences: next }).catch(() => {
      setError(t('usagePreferenceSaveFailed'));
    });
  }, [t]);

  const chooseWeeklyFilter = useCallback((weeklyFilter: WeeklyUsageFilter) => {
    savePreferences({ ...preferences, weeklyFilter });
  }, [preferences, savePreferences]);

  const applyCustom = useCallback(() => {
    const parsed = Number(customInput.trim() || '3');
    const customThreshold = Number.isFinite(parsed) ? Math.min(100, Math.max(0, parsed)) : 3;
    setCustomInput(customThreshold.toFixed(1));
    savePreferences({ ...preferences, weeklyFilter: 'custom', customThreshold });
  }, [customInput, preferences, savePreferences]);

  const setTimeFilter = useCallback((timeFilter: UsageTimeFilter) => {
    savePreferences({ ...preferences, timeFilter });
  }, [preferences, savePreferences]);

  const setSortMode = useCallback((sortMode: UsageSortMode) => {
    savePreferences({ ...preferences, sortMode });
  }, [preferences, savePreferences]);

  const filtered = useMemo(
    () => filterAndSortUsageTasks(view?.tasks ?? [], preferences),
    [preferences, view?.tasks],
  );
  const summary = useMemo(() => {
    const weeklyValues = filtered
      .map((task) => task.weeklyConsumedPercent)
      .filter((value): value is number => value !== null);
    const weeklyTotal = weeklyValues.length
      ? weeklyValues.reduce((total, value) => total + value, 0)
      : null;
    const longest = filtered.reduce((maximum, task) => Math.max(maximum, task.durationSeconds), 0);
    return {
      count: filtered.length,
      weeklyTotal,
      weeklyAverage: weeklyTotal === null ? null : weeklyTotal / weeklyValues.length,
      longest,
    };
  }, [filtered]);
  const historicalCount = useMemo(
    () => (view?.tasks ?? []).filter((task) => !task.isActive).length,
    [view?.tasks],
  );

  const exportCsv = useCallback(async () => {
    if (filtered.length === 0 || exporting) return;
    setExporting(true);
    setError('');
    setNotice('');
    try {
      const saved = await invoke<boolean>('export_usage_csv', {
        rows: createUsageCsvRows(filtered),
        language: lang,
        fileName: usageCsvFileName(lang),
      });
      if (saved) setNotice(t('usageExportSuccess'));
    } catch {
      setError(t('usageExportFailed'));
    } finally {
      setExporting(false);
    }
  }, [exporting, filtered, lang, t]);

  const confirmAction = useCallback(async () => {
    if (!confirmation || confirmBusyRef.current) return;
    confirmBusyRef.current = true;
    setConfirmBusy(true);
    setError('');
    setNotice('');
    try {
      if (confirmation.type === 'delete') {
        const { task } = confirmation;
        if (task.isActive) return;
        await invoke('delete_usage_task', { id: task.id });
        setView((current) => current ? {
          ...current,
          tasks: current.tasks.filter((item) => item.id !== task.id),
        } : current);
      } else {
        await invoke('clear_usage_tasks');
        setView((current) => current ? {
          ...current,
          tasks: current.tasks.filter((task) => task.isActive),
        } : current);
      }
      setConfirmation(null);
    } catch {
      setError(t(confirmation.type === 'delete' ? 'usageDeleteFailed' : 'usageClearFailed'));
    } finally {
      confirmBusyRef.current = false;
      setConfirmBusy(false);
    }
  }, [confirmation, t]);

  return (
    <section className="usage-log-page">
      <header className="usage-log-heading">
        <strong>{t('usageLogTitle')}</strong>
        <span>{t('usageLogDescription')}</span>
      </header>

      <div className="usage-filter-card">
        <div className="usage-filter-label">{t('usageWeeklyFilter')}</div>
        <div className="usage-quick-filters">
          {(['all', 'gte1', 'gte3', 'gte5', 'custom'] as WeeklyUsageFilter[]).map((filter) => (
            <button
              className={preferences.weeklyFilter === filter ? 'active' : ''}
              key={filter}
              type="button"
              onClick={() => chooseWeeklyFilter(filter)}
            >
              {t(`usageFilter_${filter}`)}
            </button>
          ))}
        </div>
        {preferences.weeklyFilter === 'custom' && (
          <div className="usage-custom-filter">
            <span>{t('usageCustomPrefix')}</span>
            <input
              aria-label={t('usageCustomThreshold')}
              type="number"
              min="0"
              max="100"
              step="0.1"
              value={customInput}
              onChange={(event) => setCustomInput(event.target.value)}
              onKeyDown={(event) => { if (event.key === 'Enter') applyCustom(); }}
            />
            <span>%</span>
            <button type="button" onClick={applyCustom}>{t('usageApply')}</button>
          </div>
        )}
        <div className="usage-select-row">
          <label>
            <span>{t('usageTimeFilter')}</span>
            <select value={preferences.timeFilter} onChange={(event) => setTimeFilter(event.target.value as UsageTimeFilter)}>
              <option value="today">{t('usageTimeToday')}</option>
              <option value="7d">{t('usageTime7d')}</option>
              <option value="30d">{t('usageTime30d')}</option>
              <option value="all">{t('usageTimeAll')}</option>
            </select>
          </label>
          <label>
            <span>{t('usageSort')}</span>
            <select value={preferences.sortMode} onChange={(event) => setSortMode(event.target.value as UsageSortMode)}>
              <option value="latest">{t('usageSortLatest')}</option>
              <option value="weekly">{t('usageSortWeekly')}</option>
              <option value="duration">{t('usageSortDuration')}</option>
            </select>
          </label>
        </div>
      </div>

      <div className="usage-summary-grid">
        <div><span>{t('usageTaskCount')}</span><strong>{summary.count}</strong></div>
        <div><span>{t('usageWeeklyTotal')}</span><strong>{formatPercent(summary.weeklyTotal)}</strong></div>
        <div><span>{t('usageWeeklyAverage')}</span><strong>{formatPercent(summary.weeklyAverage)}</strong></div>
        <div><span>{t('usageLongest')}</span><strong>{formatDuration(summary.longest, lang)}</strong></div>
      </div>

      {(view?.warning || error) && <div className="usage-warning">{error || view?.warning}</div>}
      {notice && <div className="usage-notice">{notice}</div>}
      {loading ? (
        <div className="usage-empty">{t('usageLoading')}</div>
      ) : filtered.length === 0 ? (
        <div className="usage-empty">
          <span>{t('usageEmpty')}</span>
          <button type="button" onClick={() => chooseWeeklyFilter('all')}>{t('usageViewAll')}</button>
        </div>
      ) : (
        <div className="usage-task-list">
          {filtered.slice(0, visibleCount).map((task) => (
            <article className="usage-task-row" key={task.id}>
              <div className="usage-task-time">{formatUsageTimeRange(task, {
                time: t('usageTimeLabel'),
                recording: t('usageRecording'),
              })}</div>
              <button
                className="usage-delete"
                type="button"
                disabled={task.isActive}
                title={t('usageDelete')}
                onClick={() => setConfirmation({ type: 'delete', task })}
              >×</button>
              <dl>
                <div><dt>{t('usageDuration')}</dt><dd>{formatDuration(task.durationSeconds, lang)}</dd></div>
                <div className="usage-primary-value"><dt>{t('usageWeeklyConsumed')}</dt><dd>{formatPercent(task.weeklyConsumedPercent)}</dd></div>
                <div><dt>{t('usageFiveHourConsumed')}</dt><dd>{formatPercent(task.fiveHourConsumedPercent)}</dd></div>
                <div><dt>{t('usageQuotaBalance')}</dt><dd>{t('usageWeekShort')} {formatPercent(task.endWeeklyRemainingPercent)} · 5h {formatPercent(task.endFiveHourRemainingPercent)}</dd></div>
              </dl>
            </article>
          ))}
          {visibleCount < filtered.length && (
            <button className="usage-load-more" type="button" onClick={() => setVisibleCount((count) => count + PAGE_SIZE)}>
              {t('usageLoadMore')}
            </button>
          )}
        </div>
      )}

      <div className="usage-management">
        <span>{t('usageManagement')}</span>
        <button type="button" disabled={loading} onClick={() => void load()}>{t('usageReload')}</button>
        <button type="button" disabled={filtered.length === 0 || exporting} onClick={() => void exportCsv()}>
          {exporting ? t('usageExporting') : t('usageExportCsv')}
        </button>
        <button className="danger" type="button" disabled={historicalCount === 0} onClick={() => setConfirmation({ type: 'clear' })}>{t('usageClearAll')}</button>
      </div>

      {confirmation && <ConfirmDialog
        title={t(confirmation.type === 'delete' ? 'usageDeleteConfirmTitle' : 'usageClearConfirmTitle')}
        description={t(confirmation.type === 'delete' ? 'usageDeleteConfirmDescription' : 'usageClearConfirm')}
        details={confirmation.type === 'delete' ? (
          <>
            <span>{formatUsageTimeRange(confirmation.task, { time: t('usageTimeLabel'), recording: t('usageRecording') })}</span>
            <span>{t('usageWeeklyConsumed')}: {formatPercent(confirmation.task.weeklyConsumedPercent)}</span>
          </>
        ) : <span>{t('usageClearCount')}: {historicalCount}</span>}
        confirmLabel={t(confirmation.type === 'delete' ? 'usageConfirmDelete' : 'usageConfirmClear')}
        cancelLabel={t('cancel')}
        loadingLabel={t('usageProcessing')}
        busy={confirmBusy}
        onCancel={() => setConfirmation(null)}
        onConfirm={() => void confirmAction()}
      />}
    </section>
  );
}
