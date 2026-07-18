import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  createUsageCsvRows,
  filterAndSortUsageTasks,
  formatUsageTimeRange,
  usageCsvFileName,
} from '../src/usageLogUtils.js';
import { getThemedSelectOpeningIndex, moveThemedSelectIndex } from '../src/themedSelectUtils.js';

const now = new Date('2026-07-15T12:00:00+08:00');

function task(id, weekly, startedAtMs = now.getTime(), durationSeconds = 60) {
  return {
    id,
    startedAtMs,
    endedAtMs: startedAtMs + durationSeconds * 1_000,
    durationSeconds,
    weeklyConsumedPercent: weekly,
    fiveHourConsumedPercent: weekly === null ? null : weekly * 2,
    endWeeklyRemainingPercent: weekly === null ? null : 95,
    endFiveHourRemainingPercent: null,
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

test('unknown weekly usage is included only by the all filter and sorts last', () => {
  const tasks = [task('unknown', null), task('known', 3)];
  assert.deepEqual(
    filterAndSortUsageTasks(tasks, preferences(), now).map((item) => item.id),
    ['known'],
  );
  assert.deepEqual(
    filterAndSortUsageTasks(tasks, preferences({ weeklyFilter: 'all', sortMode: 'weekly' }), now).map((item) => item.id),
    ['known', 'unknown'],
  );
});

test('time ranges label same-day, cross-day, active, and English records explicitly', () => {
  const start = new Date(2026, 6, 16, 1, 35).getTime();
  const sameDay = { ...task('same', 3, start), endedAtMs: new Date(2026, 6, 16, 1, 44).getTime() };
  const crossDay = { ...task('cross', 3, start), endedAtMs: new Date(2026, 6, 17, 2, 4).getTime() };
  const active = { ...sameDay, isActive: true };
  assert.equal(formatUsageTimeRange(sameDay, { time: '时间', recording: '记录中' }), '时间: 2026/07/16 01:35 - 01:44');
  assert.equal(formatUsageTimeRange(crossDay, { time: '时间', recording: '记录中' }), '时间: 2026/07/16 01:35 - 2026/07/17 02:04');
  assert.equal(formatUsageTimeRange(active, { time: '时间', recording: '记录中' }), '时间: 2026/07/16 01:35 - 记录中');
  assert.equal(formatUsageTimeRange(sameDay, { time: 'Time', recording: 'Recording' }), 'Time: 2026/07/16 01:35 - 01:44');
});

test('CSV payload uses every filtered sorted row without the 100-card page limit', () => {
  const tasks = Array.from({ length: 150 }, (_, index) => task(String(index), 3, now.getTime() - index));
  const filtered = filterAndSortUsageTasks(tasks, preferences({ weeklyFilter: 'all' }), now);
  const rows = createUsageCsvRows(filtered);
  assert.equal(rows.length, 150);
  assert.equal(rows[0].startTime, '2026/07/15 12:00');
  assert.equal(rows[0].weeklyConsumedPercent, 3);
  assert.equal(rows[0].endWeeklyRemainingPercent, 95);
  assert.equal(rows[0].endFiveHourRemainingPercent, null);
  assert.deepEqual(filtered.map((item) => item.id), tasks.map((item) => item.id));
});

test('active CSV rows leave the end time empty and filenames are localized', () => {
  const active = { ...task('active', 3), isActive: true };
  assert.equal(createUsageCsvRows([active])[0].endTime, null);
  const stamp = new Date(2026, 6, 16, 3, 1, 6);
  assert.equal(usageCsvFileName('zh', stamp), 'LXCodexMeter_消耗日志_20260716_030106.csv');
  assert.equal(usageCsvFileName('en', stamp), 'LXCodexMeter_usage_log_20260716_030106.csv');
});

test('usage cards show time and quota balance without a visible status field or window.confirm', () => {
  const page = readFileSync(new URL('../src/UsageLogPage.tsx', import.meta.url), 'utf8');
  assert.match(page, /formatUsageTimeRange/);
  assert.match(page, /endWeeklyRemainingPercent/);
  assert.match(page, /endFiveHourRemainingPercent/);
  assert.doesNotMatch(page, /window\.confirm/);
  assert.doesNotMatch(page, /t\('usageStatus'\)/);
  assert.match(page, /createUsageCsvRows\(filtered\)/);
  assert.match(page, /confirmBusyRef\.current/);
});

test('theme classes own strip and themed select variables independent of system media', () => {
  const css = readFileSync(new URL('../src/styles.css', import.meta.url), 'utf8');
  assert.match(css, /\.meter\.theme-light\s*\{[^}]*--strip-primary-text:/s);
  assert.match(css, /\.meter\.theme-dark\s*\{[^}]*--strip-primary-text:/s);
  assert.match(css, /\.meter\.theme-system\s*\{[^}]*--strip-primary-text:/s);
  assert.match(css, /\.strip-lines\s*\{[^}]*color:\s*var\(--strip-primary-text\)/s);
  assert.match(css, /\.strip-lines span\s*\{[^}]*color:\s*var\(--strip-secondary-text\)/s);
  assert.match(css, /\.meter\.theme-light\s*\{[^}]*--text-primary:/s);
  assert.match(css, /\.meter\.theme-dark\s*\{[^}]*--text-primary:/s);
  assert.match(css, /\.meter\.theme-dark\s*\{[^}]*color-scheme:\s*dark;[^}]*--select-control-background:\s*#4a4d53;[^}]*--select-menu-background:\s*#767676;[^}]*--select-option-text:\s*#ffffff;/s);
  assert.match(css, /\.meter\.theme-light\s*\{[^}]*color-scheme:\s*light;[^}]*--select-control-background:\s*#ffffff;[^}]*--select-menu-background:\s*#f2f2f2;[^}]*--select-option-text:\s*#172033;/s);
  assert.match(css, /@media \(prefers-color-scheme:\s*dark\)\s*\{\s*\.meter\.theme-system\s*\{[^}]*color-scheme:\s*dark;/s);
  assert.match(css, /@media \(prefers-color-scheme:\s*light\)\s*\{\s*\.meter\.theme-system\s*\{[^}]*color-scheme:\s*light;/s);
  assert.match(css, /\.settings \.themed-select-button\s*\{[^}]*appearance:\s*none;[^}]*-webkit-appearance:\s*none;[^}]*border:[^}]*var\(--select-control-border\);[^}]*color:\s*var\(--select-control-text\);[^}]*background:\s*var\(--select-control-background\)/s);
  const triggerStateBlock = css.match(/\.settings \.themed-select-button:hover,[^{]*\.settings \.themed-select-button\[aria-expanded="true"\]\s*\{([^}]*)\}/s)?.[1] ?? '';
  assert.match(triggerStateBlock, /border-color:\s*var\(--select-control-border\)/);
  assert.match(triggerStateBlock, /color:\s*var\(--select-control-text\)/);
  assert.match(triggerStateBlock, /background:\s*var\(--select-control-background\)/);
  assert.match(triggerStateBlock, /outline:\s*none/);
  assert.match(triggerStateBlock, /box-shadow:\s*none/);
  assert.match(triggerStateBlock, /filter:\s*none/);
  assert.doesNotMatch(triggerStateBlock, /select-selected|active-background|#1677ff/i);
  assert.match(css, /\.settings \.themed-select-button:focus-visible\s*\{[^}]*border-color:\s*var\(--select-control-border\);[^}]*box-shadow:\s*none/s);
  assert.match(css, /\.settings \.themed-select-option\[aria-selected="true"\]\s*\{[^}]*color:\s*var\(--select-option-text\);[^}]*background:\s*var\(--select-menu-background\)/s);
  assert.match(css, /\.settings \.themed-select-option\.is-active,[^{]*\.settings \.themed-select-option:hover\s*\{[^}]*color:\s*var\(--select-selected-text\);[^}]*background:\s*var\(--select-selected-background\)/s);
  assert.equal((css.match(/background:\s*var\(--select-selected-background\)/g) ?? []).length, 1);
  assert.match(css, /\.confirm-overlay\s*\{[^}]*background:\s*var\(--overlay-background\)/s);
});

test('all five dropdowns use ThemedSelect with keyboard and dismissal support', () => {
  const app = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
  const page = readFileSync(new URL('../src/UsageLogPage.tsx', import.meta.url), 'utf8');
  const component = readFileSync(new URL('../src/ThemedSelect.tsx', import.meta.url), 'utf8');
  assert.equal((app.match(/<ThemedSelect/g) ?? []).length, 3);
  assert.equal((page.match(/<ThemedSelect/g) ?? []).length, 2);
  assert.doesNotMatch(`${app}\n${page}`, /<select\b/);
  assert.match(component, /ArrowDown/);
  assert.match(component, /ArrowUp/);
  assert.match(component, /event\.key === 'Enter'/);
  assert.match(component, /event\.key === 'Escape'/);
  assert.match(component, /event\.key === 'Tab'/);
  assert.match(component, /tabIndex=\{-1\}/);
  assert.match(component, /document\.addEventListener\('pointerdown'/);
  assert.match(component, /aria-selected=/);
  assert.match(component, /useState<number \| null>\(null\)/);
  assert.match(component, /openMenu\('pointer'\)/);
  assert.match(component, /setActiveIndex\(null\)/);
  assert.match(component, /index === activeIndex \? 'is-active'/);
  assert.doesNotMatch(component, /index === selectedIndex \? 'is-active'/);
});

test('ThemedSelect arrow movement wraps without changing an empty list', () => {
  assert.equal(moveThemedSelectIndex(0, 1, 3), 1);
  assert.equal(moveThemedSelectIndex(2, 1, 3), 0);
  assert.equal(moveThemedSelectIndex(0, -1, 3), 2);
  assert.equal(moveThemedSelectIndex(0, 1, 0), -1);
});

test('ThemedSelect opening keeps pointer and neutral keyboard opens inactive', () => {
  assert.equal(getThemedSelectOpeningIndex(1, 'pointer', 4), null);
  assert.equal(getThemedSelectOpeningIndex(1, 'keyboard-neutral', 4), null);
  assert.equal(getThemedSelectOpeningIndex(1, 'ArrowDown', 4), 2);
  assert.equal(getThemedSelectOpeningIndex(1, 'ArrowUp', 4), 0);
  assert.equal(getThemedSelectOpeningIndex(0, 'ArrowUp', 4), 3);
  assert.equal(getThemedSelectOpeningIndex(0, 'ArrowDown', 0), null);
});

test('quota UI reads normalized fields and keeps fixed semantic titles', () => {
  const app = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
  const messages = readFileSync(new URL('../src/i18n.ts', import.meta.url), 'utf8');
  assert.doesNotMatch(app, /status\?*\.primary|status\?*\.secondary/);
  assert.match(app, /status\?\.five_hour/);
  assert.match(app, /status\?\.weekly/);
  assert.doesNotMatch(messages, /主额度|副额度|'Primary'/);
});

test('close settings action follows the theme selector and strip mode changes use the close transaction', () => {
  const app = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
  const css = readFileSync(new URL('../src/styles.css', import.meta.url), 'utf8');
  const theme = app.indexOf('value={config.theme}');
  const close = app.indexOf("{t('closeSettings')}", theme);
  const backToFloat = app.indexOf("{t('backToFloat')}", theme);
  assert.ok(theme >= 0 && close > theme && backToFloat > close);
  assert.match(app, /onChange=\{\(e\) => onTaskbarStripChange\(e\.target\.checked\)\}/);
  assert.match(app, /onClick=\{\(\) => onTaskbarStripChange\(false\)\}/);
  assert.equal(app.indexOf("{t('closeSettings')}", close + 1), -1);
  assert.match(app.slice(Math.max(0, close - 180), close), /className="settings-button"/);
  assert.doesNotMatch(app.slice(Math.max(0, close - 180), close), /secondary/);
  assert.match(css, /\.settings-button:hover:not\(:disabled\)/);
  assert.match(css, /\.settings-button:active:not\(:disabled\)/);
  assert.match(css, /\.settings-button:focus-visible/);
  assert.match(css, /\.settings-button:disabled/);
});

test('runtime and package versions are consistently upgraded to 0.6.15', () => {
  const expected = '0.6.15';
  const packageJson = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
  const packageLock = JSON.parse(readFileSync(new URL('../package-lock.json', import.meta.url), 'utf8'));
  const tauri = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
  const cargo = readFileSync(new URL('../src-tauri/Cargo.toml', import.meta.url), 'utf8');
  const app = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');
  assert.equal(packageJson.version, expected);
  assert.equal(packageLock.version, expected);
  assert.equal(packageLock.packages[''].version, expected);
  assert.equal(tauri.version, expected);
  assert.match(cargo, /^version = "0\.6\.15"$/m);
  assert.match(app, /APP_VERSION = '0\.6\.15'/);
});
