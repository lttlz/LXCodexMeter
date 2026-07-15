export function moveThemedSelectIndex(currentIndex, direction, optionCount) {
  if (optionCount <= 0) return -1;
  const normalized = currentIndex < 0 ? 0 : currentIndex;
  return (normalized + direction + optionCount) % optionCount;
}
