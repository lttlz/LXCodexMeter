function weeklyThreshold(preferences) {
  switch (preferences.weeklyFilter) {
    case 'all': return null;
    case 'gte1': return 1;
    case 'gte5': return 5;
    case 'custom': return preferences.customThreshold;
    default: return 3;
  }
}

function timeCutoff(filter, now) {
  if (filter === 'all') return 0;
  if (filter === 'today') {
    return new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  }
  const days = filter === '7d' ? 7 : 30;
  return now.getTime() - days * 24 * 60 * 60 * 1_000;
}

export function filterAndSortUsageTasks(tasks, preferences, now = new Date()) {
  const threshold = weeklyThreshold(preferences);
  const cutoff = timeCutoff(preferences.timeFilter, now);
  return tasks
    .filter((task) => threshold === null || (
      typeof task.weeklyConsumedPercent === 'number'
      && task.weeklyConsumedPercent + Number.EPSILON >= threshold
    ))
    .filter((task) => task.startedAtMs >= cutoff)
    .sort((left, right) => {
      if (preferences.sortMode === 'weekly') {
        return (right.weeklyConsumedPercent ?? Number.NEGATIVE_INFINITY)
          - (left.weeklyConsumedPercent ?? Number.NEGATIVE_INFINITY)
          || right.startedAtMs - left.startedAtMs;
      }
      if (preferences.sortMode === 'duration') {
        return right.durationSeconds - left.durationSeconds
          || right.startedAtMs - left.startedAtMs;
      }
      return right.startedAtMs - left.startedAtMs;
    });
}

function pad(value) {
  return String(value).padStart(2, '0');
}

export function formatLocalDateTime(timestamp) {
  const date = new Date(timestamp);
  return `${date.getFullYear()}/${pad(date.getMonth() + 1)}/${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function formatLocalTime(timestamp) {
  const date = new Date(timestamp);
  return `${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function isSameLocalDay(leftTimestamp, rightTimestamp) {
  const left = new Date(leftTimestamp);
  const right = new Date(rightTimestamp);
  return left.getFullYear() === right.getFullYear()
    && left.getMonth() === right.getMonth()
    && left.getDate() === right.getDate();
}

export function formatUsageTimeRange(task, labels) {
  const start = formatLocalDateTime(task.startedAtMs);
  const end = task.isActive
    ? labels.recording
    : isSameLocalDay(task.startedAtMs, task.endedAtMs)
      ? formatLocalTime(task.endedAtMs)
      : formatLocalDateTime(task.endedAtMs);
  return `${labels.time}: ${start} - ${end}`;
}

export function createUsageCsvRows(tasks) {
  return tasks.map((task) => ({
    startTime: formatLocalDateTime(task.startedAtMs),
    endTime: task.isActive ? null : formatLocalDateTime(task.endedAtMs),
    durationSeconds: task.durationSeconds,
    weeklyConsumedPercent: task.weeklyConsumedPercent,
    fiveHourConsumedPercent: task.fiveHourConsumedPercent,
    endWeeklyRemainingPercent: task.endWeeklyRemainingPercent,
    endFiveHourRemainingPercent: task.endFiveHourRemainingPercent,
    isEstimated: task.isEstimated,
    isComplete: task.isComplete,
  }));
}

export function usageCsvFileName(language, now = new Date()) {
  const stamp = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}_${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return language === 'zh'
    ? `LXCodexMeter_消耗日志_${stamp}.csv`
    : `LXCodexMeter_usage_log_${stamp}.csv`;
}
