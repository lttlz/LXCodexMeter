import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent,
} from 'react';
import { moveThemedSelectIndex } from './themedSelectUtils.js';

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
  const [activeIndex, setActiveIndex] = useState(selectedIndex);

  const close = useCallback(() => setOpen(false), []);
  const openMenu = useCallback(() => {
    const rect = buttonRef.current?.getBoundingClientRect();
    const menuHeight = Math.min(options.length * 29 + 8, 184);
    setOpenAbove(Boolean(rect && rect.bottom + menuHeight > window.innerHeight - 8 && rect.top > menuHeight));
    setActiveIndex(selectedIndex);
    setOpen(true);
  }, [options.length, selectedIndex]);

  const selectAt = useCallback((index: number) => {
    const option = options[index];
    if (!option) return;
    onChange(option.value);
    setOpen(false);
    window.setTimeout(() => buttonRef.current?.focus(), 0);
  }, [onChange, options]);

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
        openMenu();
      } else {
        setActiveIndex((current) => moveThemedSelectIndex(current, event.key === 'ArrowDown' ? 1 : -1, options.length));
      }
      return;
    }
    if (event.key === 'Home' && open) {
      event.preventDefault();
      setActiveIndex(0);
      return;
    }
    if (event.key === 'End' && open) {
      event.preventDefault();
      setActiveIndex(options.length - 1);
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (open) selectAt(activeIndex);
      else openMenu();
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
        aria-activedescendant={open ? `${listboxId}-option-${activeIndex}` : undefined}
        onClick={() => { if (open) close(); else openMenu(); }}
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
