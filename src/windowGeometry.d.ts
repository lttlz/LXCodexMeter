export type WindowRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type WindowSize = {
  width: number;
  height: number;
};

export function logicalSizeToPhysical(size: WindowSize, scaleFactor: number): WindowSize;
export function physicalSizeToLogical(size: WindowSize, scaleFactor: number): WindowSize;
export function calculateAnchoredSettingsGeometry(
  originalWindow: WindowRect,
  workArea: WindowRect,
  requestedSize: WindowSize,
): WindowRect;
export function calculateRestoredWindowGeometry(
  originalWindow: WindowRect,
  workAreas: WindowRect[],
  fallbackWorkArea?: WindowRect,
): WindowRect;
export function preserveInitialWindowSnapshot<T>(existingSnapshot: T | null, nextSnapshot: T): T;
