import type { UsageLogPreferences, UsageTask } from './types';

export function filterAndSortUsageTasks(
  tasks: UsageTask[],
  preferences: UsageLogPreferences,
  now?: Date,
): UsageTask[];
