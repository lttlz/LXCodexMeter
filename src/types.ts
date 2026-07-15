export type SourceMode = 'app_server' | 'auto' | 'client_window' | 'client_watch';
export type Language = 'zh' | 'en';
export type ThemeMode = 'system' | 'light' | 'dark';

export type LimitWindow = {
  label: string;
  used_percent: number | null;
  remaining_percent: number | null;
  window_duration_mins: number | null;
  resets_at: number | null;
  reset_text: string | null;
  reached_type: string | null;
};

export type CodexMeterStatus = {
  ok: boolean;
  message: string;
  source_mode: SourceMode | string;
  auth_mode: string | null;
  plan_type: string | null;
  primary: LimitWindow | null;
  secondary: LimitWindow | null;
  five_hour: LimitWindow | null;
  weekly: LimitWindow | null;
  credit_balance: number | null;
  credit_limit: number | null;
  reset_credits_available: number | null;
  updated_at_ms: number;
};

export type MeterConfig = {
  refresh_interval_secs: number;
  show_floating_window: boolean;
  opacity: number;
  always_on_top: boolean;
  compact: boolean;
  ui_scale: number;
  taskbar_strip: boolean;
  show_reset_time: boolean;
  auto_update: boolean;
  source_mode: SourceMode;
  autostart: boolean;
  start_hidden: boolean;
  auto_show_on_codex: boolean;
  auto_hide_on_codex_close: boolean;
  language: Language;
  theme: ThemeMode;
};

export type WeeklyUsageFilter = 'all' | 'gte1' | 'gte3' | 'gte5' | 'custom';
export type UsageTimeFilter = 'today' | '7d' | '30d' | 'all';
export type UsageSortMode = 'latest' | 'weekly' | 'duration';

export type UsageLogPreferences = {
  weeklyFilter: WeeklyUsageFilter;
  customThreshold: number;
  timeFilter: UsageTimeFilter;
  sortMode: UsageSortMode;
};

export type UsageTask = {
  id: string;
  startedAtMs: number;
  endedAtMs: number;
  durationSeconds: number;
  weeklyConsumedPercent: number | null;
  fiveHourConsumedPercent: number | null;
  recordMode: 'automatic' | string;
  isComplete: boolean;
  isEstimated: boolean;
  createdAtMs: number;
  updatedAtMs: number;
  isActive: boolean;
};

export type UsageLogView = {
  tasks: UsageTask[];
  preferences: UsageLogPreferences;
  warning: string | null;
};
