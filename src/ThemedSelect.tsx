import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent,
} from 'react';
import { getThemedSelectOpeningIndex, moveThemedSelectIndex } from './themedSelectUtils.js';

export type ThemedSelectOption = {
  value: string;
  label: string;
};

type ThemedSelectProps = {
  ariaLabel: string;
  value: string;
  options: readonly ThemedSelectOption[];
  onChange: (value: string) => void;
};

export default function ThemedSelect({ ariaLabel, value, options, onChange }: ThemedSelectProps) {
  const listboxId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const [openAbove, setOpenAbove] = useState(false);
  const selectedIndex = Math.max(0, options.findIndex((option) => option.value === value));
  const [activeIndex, setActiveIndex] = useState<number | null>(null);

  const close = useCallback(() => {
    setOpen(false);
    setActiveIndex(null);
  }, []);
  const openMenu = useCallback((action: 'pointer' | 'keyboard-neutral' | 'ArrowDown' | 'ArrowUp') => {
    const rect = buttonRef.current?.getBoundingClientRect();
    const menuHeight = Math.min(options.length * 29 + 8, 184);
    setOpenAbove(Boolean(rect && rect.bottom + menuHeight > window.innerHeight - 8 && rect.top > menuHeight));
    setActiveIndex(getThemedSelectOpeningIndex(selectedIndex, action, options.length));
    setOpen(true);
  }, [options.length, selectedIndex]);

  const selectAt = useCallback((index: number) => {
    const option = options[index];
    if (!option) return;
    onChange(option.value);
    close();
    window.setTimeout(() => buttonRef.current?.focus(), 0);
  }, [close, onChange, options]);

  useEffect(() => {
    if (!open) return undefined;
    const handleOutsidePress = (event: Event) => {
      if (!rootRef.current?.contains(event.target as Node)) close();
    };
    document.addEventListener('pointerdown', handleOutsidePress, true);
    document.addEventListener('mousedown', handleOutsidePress, true);
    return () => {
      document.removeEventListener('pointerdown', handleOutsidePress, true);
      document.removeEventListener('mousedown', handleOutsidePress, true);
    };
  }, [close, open]);

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (!open) {
        openMenu(event.key);
      } else {
        setActiveIndex((current) => {
          const next = moveThemedSelectIndex(
            current ?? selectedIndex,
            event.key === 'ArrowDown' ? 1 : -1,
            options.length,
          );
          return next < 0 ? null : next;
        });
      }
      return;
    }
    if (event.key === 'Home' && open) {
      event.preventDefault();
      setActiveIndex(options.length > 0 ? 0 : null);
      return;
    }
    if (event.key === 'End' && open) {
      event.preventDefault();
      setActiveIndex(options.length > 0 ? options.length - 1 : null);
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (open) selectAt(activeIndex ?? selectedIndex);
      else openMenu('keyboard-neutral');
      return;
    }
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      close();
      return;
    }
    if (event.key === 'Tab' && open) close();
  };

  const selected = options[selectedIndex];
  return (
    <div className="themed-select" ref={rootRef}>
      <button
        ref={buttonRef}
        className="themed-select-button"
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listboxId}
        aria-activedescendant={open && activeIndex !== null ? `${listboxId}-option-${activeIndex}` : undefined}
        onClick={() => { if (open) close(); else openMenu('pointer'); }}
        onKeyDown={handleKeyDown}
      >
        <span>{selected?.label ?? value}</span>
        <span className="themed-select-chevron" aria-hidden="true">▾</span>
      </button>
      {open && (
        <div
          id={listboxId}
          className={`themed-select-menu ${openAbove ? 'open-above' : ''}`}
          role="listbox"
          aria-label={ariaLabel}
        >
          {options.map((option, index) => (
            <button
              className={`themed-select-option ${index === activeIndex ? 'is-active' : ''}`}
              type="button"
              role="option"
              id={`${listboxId}-option-${index}`}
              tabIndex={-1}
              aria-selected={index === selectedIndex}
              key={option.value}
              onMouseEnter={() => setActiveIndex(index)}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => selectAt(index)}
            >
              {option.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
