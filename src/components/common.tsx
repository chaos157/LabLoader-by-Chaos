import { getCurrentWindow } from "@tauri-apps/api/window";
import { AlertTriangle, CheckCircle2, Info, Loader2, Minus, Square, X } from "lucide-react";
import {
  useEffect,
  type CSSProperties,
  type ReactNode,
} from "react";
import { useStore } from "../lib/store";

export function Titlebar({ subtitle }: { subtitle?: string }) {
  const win = getCurrentWindow();
  const isAndroid = /Android/i.test(navigator.userAgent);
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar-brand" data-tauri-drag-region>
        <span className="brand-mark" data-tauri-drag-region>
          L
        </span>
        <span className="brand-name" data-tauri-drag-region>
          LabLoader
        </span>
        <span className="brand-sub" data-tauri-drag-region>
          {subtitle ?? "BONELAB mod manager"}
        </span>
      </div>
      {!isAndroid && (
        <div className="titlebar-controls">
          <button
            className="win-btn"
            title="Minimize"
            onClick={() => void win.minimize()}
          >
            <Minus size={15} />
          </button>
          <button
            className="win-btn"
            title="Maximize"
            onClick={() => void win.toggleMaximize()}
          >
            <Square size={12} />
          </button>
          <button
            className="win-btn win-close"
            title="Close"
            onClick={() => void win.close()}
          >
            <X size={15} />
          </button>
        </div>
      )}
    </header>
  );
}

export function Modal({
  open,
  title,
  subtitle,
  onClose,
  children,
  footer,
  width,
}: {
  open: boolean;
  title: string;
  subtitle?: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  width?: number;
}) {
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div
        className="modal"
        style={{ maxWidth: width ?? 520 }}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="modal-head">
          <div>
            <h2>{title}</h2>
            {subtitle && <p>{subtitle}</p>}
          </div>
          <button className="icon-btn" onClick={onClose} title="Close">
            <X size={16} />
          </button>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-foot">{footer}</div>}
      </div>
    </div>
  );
}

export function Toaster() {
  const { toasts, dismissToast } = useStore();
  return (
    <div className="toast-stack">
      {toasts.map((entry) => (
        <div
          key={entry.id}
          className={`toast toast-${entry.kind}`}
          onClick={() => dismissToast(entry.id)}
          role="status"
        >
          {entry.kind === "success" && <CheckCircle2 size={17} />}
          {entry.kind === "error" && <AlertTriangle size={17} />}
          {entry.kind === "info" && <Info size={17} />}
          <span>{entry.message}</span>
        </div>
      ))}
    </div>
  );
}

export function Switch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label ?? "Toggle"}
      className={`switch ${checked ? "on" : ""}`}
      onClick={() => onChange(!checked)}
    >
      <span className="switch-knob" />
    </button>
  );
}

export function Spinner({ size = 16, label }: { size?: number; label?: string }) {
  return (
    <span className="spinner">
      <Loader2 size={size} className="spin" />
      {label && <span>{label}</span>}
    </span>
  );
}

export function EmptyState({
  icon,
  title,
  body,
  action,
}: {
  icon: ReactNode;
  title: string;
  body: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <div className="empty-icon">{icon}</div>
      <h3>{title}</h3>
      <p>{body}</p>
      {action}
    </div>
  );
}

export function Select({
  value,
  onChange,
  children,
  disabled,
  style,
  ariaLabel,
}: {
  value: string;
  onChange: (value: string) => void;
  children: ReactNode;
  disabled?: boolean;
  style?: CSSProperties;
  ariaLabel?: string;
}) {
  return (
    <div className="select-wrap" style={style}>
      <select
        aria-label={ariaLabel}
        className="select"
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      >
        {children}
      </select>
    </div>
  );
}

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      {children}
      {hint && <span className="field-hint">{hint}</span>}
    </label>
  );
}
