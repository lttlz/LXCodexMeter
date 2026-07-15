export function moveThemedSelectIndex(
  currentIndex: number,
  direction: -1 | 1,
  optionCount: number,
): number;

export function getThemedSelectOpeningIndex(
  selectedIndex: number,
  action: 'pointer' | 'keyboard-neutral' | 'ArrowDown' | 'ArrowUp',
  optionCount: number,
): number | null;
