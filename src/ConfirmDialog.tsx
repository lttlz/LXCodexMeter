import { useEffect, type ReactNode } from 'react';

type ConfirmDialogProps = {
  title: string;
  description: string;
  details?: ReactNode;
  confirmLabel: string;
  cancelLabel: string;
  loadingLabel: string;
  busy: boolean;
  onConfirm: () => void;
  onCancel: () => void;
};

export default function ConfirmDialog({
  title,
  description,
  details,
  confirmLabel,
  cancelLabel,
  loadingLabel,
  busy,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !busy) onCancel();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [busy, onCancel]);

  return (
    <div className="confirm-overlay" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget && !busy) onCancel();
    }}>
      <section className="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title">
        <strong id="confirm-title">{title}</strong>
        <p>{description}</p>
        {details && <div className="confirm-details">{details}</div>}
        <div className="confirm-actions">
          <button type="button" disabled={busy} onClick={onCancel}>{cancelLabel}</button>
          <button className="danger" type="button" disabled={busy} onClick={onConfirm}>
            {busy ? loadingLabel : confirmLabel}
          </button>
        </div>
      </section>
    </div>
  );
}
