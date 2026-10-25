import { useTranslation } from "../i18n";
import { createContext, useCallback, useContext, useRef, useState } from "react";
import type { ReactNode } from "react";

export type ToastKind = "info" | "success" | "error";

export interface ToastMessage {
  id: number;
  text: string;
  kind: ToastKind;
}

type NotifyFn = (text: string, kind?: ToastKind) => void;

const ToastContext = createContext<NotifyFn>(() => {});

export function useToast(): NotifyFn {
  return useContext(ToastContext);
}

/**
 * Wraps a backend call: shows the success message on resolve, or the plain
 * string error thrown by the command on reject. Returns whether it succeeded.
 */
export function useRun(): (
  fn: () => Promise<unknown>,
  successMessage: string,
) => Promise<boolean> {
  const notify = useToast();
  return useCallback(
    async (fn, successMessage) => {
      try {
        await fn();
        notify(successMessage, "success");
        return true;
      } catch (err) {
        notify(errorText(err), "error");
        return false;
      }
    },
    [notify],
  );
}

function errorText(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  try {
    return String(err);
  } catch {
    return "Erro desconhecido";
  }
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const counter = useRef(0);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const notify = useCallback<NotifyFn>((text, kind = "info") => {
    counter.current += 1;
    setToast({ id: counter.current, text, kind });
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = setTimeout(() => setToast(null), 5000);
  }, []);

  return (
    <ToastContext.Provider value={notify}>
      {children}
      <div className="statusbar" role="status" aria-live="polite">
        {toast ? (
          <span className={`toast toast--${toast.kind}`} key={toast.id}>
            <span className="toast__dot" />
            {t(toast.text)}
          </span>
        ) : (
          <span className="toast toast--idle">{t("Pronto")}</span>
        )}
      </div>
    </ToastContext.Provider>
  );
}
