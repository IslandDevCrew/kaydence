import { useLayoutEffect, useRef, type ReactNode } from "react";
import "./Modal.css";

function controlsIn(dialog: HTMLDialogElement): HTMLElement[] {
  return Array.from(dialog.querySelectorAll<HTMLElement>(
    "button, [href], input, select, textarea, [tabindex]",
  )).filter((element) => element.tabIndex >= 0 && !element.matches(":disabled")
    && element.getClientRects().length > 0);
}

// A handled Tab move must retain its cue even if the browser's heuristic does not.
function keyboardCue(element: HTMLElement): () => void {
  element.dataset.modalKeyboardFocus = "";
  const clear = () => {
    delete element.dataset.modalKeyboardFocus;
    element.removeEventListener("blur", clear);
    document.removeEventListener("pointerdown", clear, true);
  };
  element.addEventListener("blur", clear, { once: true });
  document.addEventListener("pointerdown", clear, { capture: true, once: true });
  return clear;
}

function revealFocus(element: HTMLElement, block: ScrollLogicalPosition = "nearest"): void {
  element.focus({ preventScroll: true });
  // Refocusing an already-active control does not undo subsequent wheel scrolling.
  element.scrollIntoView({ block, inline: "nearest", behavior: "instant" });
}

/** Native modality keeps the background inert; the actual trigger owns focus return. */
export function Modal({
  children, className, labelledBy, onDismiss, opener, fallbackOpener,
}: {
  children: ReactNode;
  className: string;
  labelledBy: string;
  onDismiss: () => void;
  opener: HTMLButtonElement | null;
  fallbackOpener?: HTMLButtonElement | null;
}): JSX.Element {
  const ref = useRef<HTMLDialogElement>(null);
  const keyboard = useRef(false);
  const clearCue = useRef<(() => void) | null>(null);
  const cue = (element: HTMLElement) => {
    clearCue.current?.();
    clearCue.current = keyboardCue(element);
  };
  useLayoutEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    dialog.showModal();
    revealFocus(controlsIn(dialog)[0] ?? dialog);
    return () => {
      clearCue.current?.();
      dialog.close();
      const target = opener?.isConnected && !opener.disabled ? opener : fallbackOpener;
      if (target?.isConnected && !target.disabled) {
        revealFocus(target, target === opener ? "nearest" : "center");
        if (keyboard.current) keyboardCue(target);
      }
    };
  }, [opener, fallbackOpener]);

  return (
    <dialog
      aria-labelledby={labelledBy}
      className={`modal-backdrop ${className}`}
      onCancel={(event) => { keyboard.current = true; event.preventDefault(); onDismiss(); }}
      onClick={(event) => { if (event.target === event.currentTarget) onDismiss(); }}
      onFocusCapture={(event) => {
        if (keyboard.current && controlsIn(event.currentTarget).includes(event.target)) cue(event.target);
      }}
      onPointerDownCapture={() => { keyboard.current = false; clearCue.current?.(); }}
      onKeyDown={(event) => {
        if (["Shift", "Alt", "Control", "Meta"].includes(event.key)) return;
        keyboard.current = true;
        const controls = controlsIn(event.currentTarget);
        if (document.activeElement instanceof HTMLElement && controls.includes(document.activeElement)) cue(document.activeElement);
        if (event.key !== "Tab") return;
        const first = controls[0];
        const last = controls[controls.length - 1];
        // Browsers may focus a scrolling section that is not in the control list.
        const unmanaged = !controls.includes(document.activeElement as HTMLElement);
        if (event.shiftKey && (document.activeElement === first || unmanaged)) {
          event.preventDefault();
          revealFocus(last ?? event.currentTarget);
        } else if (!event.shiftKey && (document.activeElement === last || unmanaged)) {
          event.preventDefault();
          revealFocus(first ?? event.currentTarget);
        }
      }}
      ref={ref}
    >{children}</dialog>
  );
}
