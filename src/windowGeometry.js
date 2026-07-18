function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function normalizeRect(rect) {
  return {
    x: Math.round(rect.x),
    y: Math.round(rect.y),
    width: Math.max(1, Math.round(rect.width)),
    height: Math.max(1, Math.round(rect.height)),
  };
}

export function logicalSizeToPhysical(size, scaleFactor) {
  const scale = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
  return {
    width: Math.max(1, Math.round(size.width * scale)),
    height: Math.max(1, Math.round(size.height * scale)),
  };
}

export function physicalSizeToLogical(size, scaleFactor) {
  const scale = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
  return {
    width: Math.max(1, Math.round(size.width / scale)),
    height: Math.max(1, Math.round(size.height / scale)),
  };
}

export function calculateAnchoredSettingsGeometry(originalWindow, workArea, requestedSize) {
  const original = normalizeRect(originalWindow);
  const area = normalizeRect(workArea);
  const requested = normalizeRect({ x: 0, y: 0, ...requestedSize });
  const width = Math.min(requested.width, area.width);
  const height = Math.min(requested.height, area.height);
  const workAreaRight = area.x + area.width;
  const workAreaBottom = area.y + area.height;
  const originalBottom = clamp(original.y + original.height, area.y, workAreaBottom);

  return {
    x: clamp(original.x, area.x, workAreaRight - width),
    y: clamp(originalBottom - height, area.y, workAreaBottom - height),
    width,
    height,
  };
}

function intersects(left, right) {
  return left.x < right.x + right.width
    && left.x + left.width > right.x
    && left.y < right.y + right.height
    && left.y + left.height > right.y;
}

export function calculateRestoredWindowGeometry(originalWindow, workAreas, fallbackWorkArea) {
  const original = normalizeRect(originalWindow);
  if (workAreas.some((area) => intersects(original, normalizeRect(area)))) return original;

  const fallback = normalizeRect(fallbackWorkArea ?? workAreas[0] ?? original);
  const width = Math.min(original.width, fallback.width);
  const height = Math.min(original.height, fallback.height);
  return {
    x: clamp(original.x, fallback.x, fallback.x + fallback.width - width),
    y: clamp(original.y, fallback.y, fallback.y + fallback.height - height),
    width,
    height,
  };
}

export function preserveInitialWindowSnapshot(existingSnapshot, nextSnapshot) {
  return existingSnapshot ?? nextSnapshot;
}
