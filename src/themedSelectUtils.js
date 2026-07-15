export function moveThemedSelectIndex(currentIndex, direction, optionCount) {
  if (optionCount <= 0) return -1;
  const normalized = currentIndex < 0 ? 0 : currentIndex;
  return (normalized + direction + optionCount) % optionCount;
}

export function getThemedSelectOpeningIndex(selectedIndex, action, optionCount) {
  if (action !== 'ArrowDown' && action !== 'ArrowUp') return null;
  const next = moveThemedSelectIndex(selectedIndex, action === 'ArrowDown' ? 1 : -1, optionCount);
  return next < 0 ? null : next;
}
