import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { tr } from './i18n';
import { filterAndSortUsageTasks } from './usageLogUtils.js';
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

function formatPercent(value: number): string {
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

function formatStartedAt(timestamp: number, lang: Language): string {
  return new Date(timestamp).toLocaleString(lang === 'zh' ? 'zh-CN' : 'en-US', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}

function taskStatus(task: UsageTask, lang: Language): string {
  if (task.isActive) return tr(lang, 'usageStatusActive');
  if (!task.isComplete) return tr(lang, 'usageStatusIncomplete');
  if (task.isEstimated) return tr(lang, 'usageStatusEstimated');
  return tr(lang, 'usageStatusComplete');
}

export default function UsageLogPage({ lang }: { lang: Language }) {
  const t = useMemo(() => (key: string) => tr(lang, key), [lang]);
  const [view, setView] = useState<UsageLogView | null>(null);
  const [preferences, setPreferences] = useState(DEFAULT_PREFERENCES);
  const [customInput, setCustomInput] = useState('3.0');
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setError('');
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
    const weeklyTotal = filtered.reduce((total, task) => total + task.weeklyConsumedPercent, 0);
    const longest = filtered.reduce((maximum, task) => Math.max(maximum, task.durationSeconds), 0);
    return {
      count: filtered.length,
      weeklyTotal,
      weeklyAverage: filtered.length ? weeklyTotal / filtered.length : 0,
      longest,
    };
  }, [filtered]);

  const deleteTask = useCallback(async (task: UsageTask) => {
    if (task.isActive) return;
    try {
      await invoke('delete_usage_task', { id: task.id });
      setView((current) => current ? {
        ...current,
        tasks: current.tasks.filter((item) => item.id !== task.id),
      } : current);
    } catch {
      setError(t('usageDeleteFailed'));
    }
  }, [t]);

  const clearHistory = useCallback(async () => {
    if (!window.confirm(t('usageClearConfirm'))) return;
    try {
      await invoke('clear_usage_tasks');
      setView((current) => current ? {
        ...current,
        tasks: current.tasks.filter((task) => task.isActive),
      } : current);
    } catch {
      setError(t('usageClearFailed'));
    }
  }, [t]);

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
              <div className="usage-task-time">{formatStartedAt(task.startedAtMs, lang)}</div>
              <button
                className="usage-delete"
                type="button"
                disabled={task.isActive}
                title={t('usageDelete')}
                onClick={() => void deleteTask(task)}
              >×</button>
              <dl>
                <div><dt>{t('usageDuration')}</dt><dd>{formatDuration(task.durationSeconds, lang)}</dd></div>
                <div className="usage-primary-value"><dt>{t('usageWeeklyConsumed')}</dt><dd>{formatPercent(task.weeklyConsumedPercent)}</dd></div>
                <div><dt>{t('usageFiveHourConsumed')}</dt><dd>{formatPercent(task.fiveHourConsumedPercent)}</dd></div>
                <div><dt>{t('usageStatus')}</dt><dd>{taskStatus(task, lang)}</dd></div>
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
        <button type="button" onClick={() => void load()}>{t('usageReload')}</button>
        <button className="danger" type="button" onClick={() => void clearHistory()}>{t('usageClearAll')}</button>
      </div>
    </section>
  );
}
