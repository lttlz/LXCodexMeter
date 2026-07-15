import test from 'node:test';
import assert from 'node:assert/strict';
import { filterAndSortUsageTasks } from '../src/usageLogUtils.js';

const now = new Date('2026-07-15T12:00:00+08:00');

function task(id, weekly, startedAtMs = now.getTime(), durationSeconds = 60) {
  return {
    id,
    startedAtMs,
    endedAtMs: startedAtMs + durationSeconds * 1_000,
    durationSeconds,
    weeklyConsumedPercent: weekly,
    fiveHourConsumedPercent: weekly * 2,
    recordMode: 'automatic',
    isComplete: true,
    isEstimated: false,
    createdAtMs: startedAtMs,
    updatedAtMs: startedAtMs,
    isActive: false,
  };
}

function preferences(overrides = {}) {
  return {
    weeklyFilter: 'gte3',
    customThreshold: 3,
    timeFilter: '30d',
    sortMode: 'latest',
    ...overrides,
  };
}

test('gte3 includes exactly 3.0 percent', () => {
  const tasks = [0.5, 1.0, 2.9, 3.0, 3.1, 5.0].map((weekly, index) => task(String(index), weekly));
  const result = filterAndSortUsageTasks(tasks, preferences(), now);
  assert.deepEqual(result.map((item) => item.weeklyConsumedPercent), [3.0, 3.1, 5.0]);
});

test('quick and custom weekly filters use inclusive thresholds', () => {
  const tasks = [task('one', 1), task('two-five', 2.5), task('five', 5)];
  assert.equal(filterAndSortUsageTasks(tasks, preferences({ weeklyFilter: 'all' }), now).length, 3);
  assert.equal(filterAndSortUsageTasks(tasks, preferences({ weeklyFilter: 'gte1' }), now).length, 3);
  assert.deepEqual(filterAndSortUsageTasks(tasks, preferences({ weeklyFilter: 'gte5' }), now).map((item) => item.id), ['five']);
  assert.deepEqual(filterAndSortUsageTasks(tasks, preferences({ weeklyFilter: 'custom', customThreshold: 2.5 }), now).map((item) => item.id), ['two-five', 'five']);
});

test('time filter combines with weekly filter', () => {
  const recent = task('recent', 3, now.getTime() - 6 * 24 * 60 * 60 * 1_000);
  const old = task('old', 5, now.getTime() - 8 * 24 * 60 * 60 * 1_000);
  assert.deepEqual(filterAndSortUsageTasks([old, recent], preferences({ timeFilter: '7d' }), now).map((item) => item.id), ['recent']);
});

test('sort modes order by latest, weekly consumption, and duration', () => {
  const tasks = [
    task('old-high', 5, now.getTime() - 2_000, 10),
    task('new-low', 3, now.getTime() - 1_000, 20),
    task('long', 4, now.getTime() - 3_000, 30),
  ];
  assert.deepEqual(filterAndSortUsageTasks(tasks, preferences(), now).map((item) => item.id), ['new-low', 'old-high', 'long']);
  assert.deepEqual(filterAndSortUsageTasks(tasks, preferences({ sortMode: 'weekly' }), now).map((item) => item.id), ['old-high', 'long', 'new-low']);
  assert.deepEqual(filterAndSortUsageTasks(tasks, preferences({ sortMode: 'duration' }), now).map((item) => item.id), ['long', 'new-low', 'old-high']);
});
