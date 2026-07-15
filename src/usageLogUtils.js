function weeklyThreshold(preferences) {
  switch (preferences.weeklyFilter) {
    case 'all': return 0;
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
    .filter((task) => task.weeklyConsumedPercent + Number.EPSILON >= threshold)
    .filter((task) => task.startedAtMs >= cutoff)
    .sort((left, right) => {
      if (preferences.sortMode === 'weekly') {
        return right.weeklyConsumedPercent - left.weeklyConsumedPercent
          || right.startedAtMs - left.startedAtMs;
      }
      if (preferences.sortMode === 'duration') {
        return right.durationSeconds - left.durationSeconds
          || right.startedAtMs - left.startedAtMs;
      }
      return right.startedAtMs - left.startedAtMs;
    });
}
