import type { UsageLogPreferences, UsageTask } from './types';

export function filterAndSortUsageTasks(
  tasks: UsageTask[],
  preferences: UsageLogPreferences,
  now?: Date,
): UsageTask[];
export function formatLocalDateTime(timestamp: number): string;
export function formatUsageTimeRange(
  task: UsageTask,
  labels: { time: string; recording: string },
): string;
export function createUsageCsvRows(tasks: UsageTask[]): import('./types').UsageCsvRow[];
export function usageCsvFileName(language: import('./types').Language, now?: Date): string;
