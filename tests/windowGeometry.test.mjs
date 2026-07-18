import test from 'node:test';
import assert from 'node:assert/strict';
import {
  calculateAnchoredSettingsGeometry,
  calculateRestoredWindowGeometry,
  logicalSizeToPhysical,
  preserveInitialWindowSnapshot,
} from '../src/windowGeometry.js';

const workArea = { x: 0, y: 0, width: 1920, height: 1040 };

test('settings geometry stays in the work area when the window starts in the middle', () => {
  assert.deepEqual(
    calculateAnchoredSettingsGeometry(
      { x: 500, y: 300, width: 215, height: 142 },
      workArea,
      { width: 300, height: 660 },
    ),
    { x: 500, y: 0, width: 300, height: 660 },
  );
});

test('settings opens upward when the original bottom touches the work-area bottom', () => {
  assert.deepEqual(
    calculateAnchoredSettingsGeometry(
      { x: 500, y: 898, width: 215, height: 142 },
      workArea,
      { width: 300, height: 660 },
    ),
    { x: 500, y: 380, width: 300, height: 660 },
  );
});

test('right-bottom placement is clamped at both work-area edges', () => {
  assert.deepEqual(
    calculateAnchoredSettingsGeometry(
      { x: 1800, y: 900, width: 215, height: 140 },
      workArea,
      { width: 300, height: 660 },
    ),
    { x: 1620, y: 380, width: 300, height: 660 },
  );
});

test('settings taller than the work area is capped to the full work-area height', () => {
  assert.deepEqual(
    calculateAnchoredSettingsGeometry(
      { x: 500, y: 898, width: 215, height: 142 },
      workArea,
      { width: 300, height: 1400 },
    ),
    { x: 500, y: 0, width: 300, height: 1040 },
  );
});

test('negative monitor coordinates remain valid', () => {
  assert.deepEqual(
    calculateAnchoredSettingsGeometry(
      { x: -1800, y: 860, width: 215, height: 140 },
      { x: -1920, y: 0, width: 1920, height: 1000 },
      { width: 300, height: 660 },
    ),
    { x: -1800, y: 340, width: 300, height: 660 },
  );
});

test('a non-zero work-area top is respected', () => {
  assert.deepEqual(
    calculateAnchoredSettingsGeometry(
      { x: 100, y: 100, width: 215, height: 140 },
      { x: 0, y: 40, width: 1280, height: 960 },
      { width: 300, height: 660 },
    ),
    { x: 100, y: 40, width: 300, height: 660 },
  );
});

test('logical settings sizes convert correctly at common Windows scale factors', () => {
  assert.deepEqual(logicalSizeToPhysical({ width: 300, height: 660 }, 1.25), { width: 375, height: 825 });
  assert.deepEqual(logicalSizeToPhysical({ width: 300, height: 660 }, 1.5), { width: 450, height: 990 });
  assert.deepEqual(logicalSizeToPhysical({ width: 300, height: 660 }, 2), { width: 600, height: 1320 });
});

test('an original bottom below the work area is corrected before anchoring', () => {
  assert.deepEqual(
    calculateAnchoredSettingsGeometry(
      { x: 500, y: 1000, width: 215, height: 142 },
      workArea,
      { width: 300, height: 660 },
    ),
    { x: 500, y: 380, width: 300, height: 660 },
  );
});

test('closing restores the exact original geometry while its monitor still exists', () => {
  const original = { x: -1800, y: 850, width: 215, height: 142 };
  assert.deepEqual(
    calculateRestoredWindowGeometry(original, [{ x: -1920, y: 0, width: 1920, height: 1040 }], workArea),
    original,
  );
});

test('closing clamps a stale off-screen position into the current work area', () => {
  assert.deepEqual(
    calculateRestoredWindowGeometry(
      { x: -1800, y: 850, width: 215, height: 142 },
      [workArea],
      workArea,
    ),
    { x: 0, y: 850, width: 215, height: 142 },
  );
});

test('rapid opens preserve the first window snapshot', () => {
  const first = { x: 10, y: 20, width: 215, height: 142 };
  const later = { x: 10, y: 0, width: 300, height: 660 };
  assert.equal(preserveInitialWindowSnapshot(null, first), first);
  assert.equal(preserveInitialWindowSnapshot(first, later), first);
});
