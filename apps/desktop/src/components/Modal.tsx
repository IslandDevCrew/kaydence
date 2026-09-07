import { useLayoutEffect, useRef, type ReactNode } from "react";
import "./Modal.css";

/** Native modality keeps the background inert and restores the opener's focus. */
export function Modal({
  children, className, labelledBy, onDismiss,
}: {
  children: ReactNode;
  className: string;
  labelledBy: string;
  onDismiss: () => void;
}): JSX.Element {
  const ref = useRef<HTMLDialogElement>(null);
  useLayoutEffect(() => {
    const dialog = ref.current;
    dialog?.showModal();
    return () => dialog?.close();
  }, []);

  return (
    <dialog
      aria-labelledby={labelledBy}
      className={`modal-backdrop ${className}`}
      onCancel={(event) => { event.preventDefault(); onDismiss(); }}
      onClick={(event) => { if (event.target === event.currentTarget) onDismiss(); }}
      onKeyDown={(event) => {
        if (event.key !== "Tab") return;
        const controls = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(
          "button, [href], input, select, textarea, [tabindex]",
        )).filter((element) => element.tabIndex >= 0 && !element.matches(":disabled")
          && element.getClientRects().length > 0);
        const first = controls[0];
        const last = controls[controls.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }}
      ref={ref}
    >{children}</dialog>
  );
}
