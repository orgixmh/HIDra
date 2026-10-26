import { useEffect, useId, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { callBackend, isTauri } from "../backend";
import { useTranslation } from "../i18n";

type DesktopStatus = { tray_available: boolean; close_requested: boolean; error: string | null };
export function CloseToTray() {
  const { language, t } = useTranslation();
  const [status, setStatus] = useState<DesktopStatus | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useId();
  const description = useId();
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let stop: (() => void) | undefined;
    const receive = (next: DesktopStatus) => {
      if (!disposed) { setStatus(next); if (next.close_requested) setOpen(true); }
    };
    void (async () => {
      try {
        const unlisten = await listen<DesktopStatus>("hidra-close-requested", (event) => receive(event.payload));
        if (disposed) { unlisten(); return; }
        stop = unlisten;
        receive(await callBackend<DesktopStatus>("desktop_lifecycle_status"));
      } catch (err) { if (!disposed) setError(String(err)); }
    })();
    return () => { disposed = true; stop?.(); };
  }, []);
  useEffect(() => {
    if (isTauri()) void callBackend("desktop_lifecycle_language", { language }).catch((err: unknown) => setError(String(err)));
  }, [language]);
  useEffect(() => {
    if (open && dialog.current && !dialog.current.open) dialog.current.showModal();
    else if (!open && dialog.current?.open) dialog.current.close();
  }, [open]);
  const choose = async (choice: "background" | "exit" | "cancel") => {
    if (busy) return;
    setBusy(true); setError(null);
    try {
      await callBackend("desktop_lifecycle_close", { choice });
      setOpen(false);
    } catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  };
  return <dialog className="close-dialog" ref={dialog} aria-labelledby={heading} aria-describedby={description}
    onCancel={(event) => { event.preventDefault(); void choose("cancel"); }}>
    <div className="close-dialog__icon" aria-hidden="true">H</div>
    <h2 id={heading}>{t("Keep Smart Notifications running?")}</h2>
    <p id={description}>{t("Smart Notifications is on. Exiting HIDra will stop notification alerts and script checks, and restore your previous lighting.")}</p>
    <p>{t("Keep HIDra in the top bar to close this window while notifications continue. Open it again from the HIDra icon.")}</p>
    {!status?.tray_available && <p className="smart-error" role="alert">{t(status?.error ?? "The top bar icon is unavailable. Keep this window open to continue Smart Notifications.")}</p>}
    {error && <p className="smart-error" role="alert">{t(error)}</p>}
    <div className="close-dialog__actions">
      <button className="btn" type="button" disabled={busy} onClick={() => void choose("cancel")}>{t("Cancel")}</button>
      <button className="btn close-dialog__exit" type="button" disabled={busy} onClick={() => void choose("exit")}>{t("Exit HIDra")}</button>
      <button className="btn btn--primary" type="button" disabled={busy || !status?.tray_available}
        onClick={() => void choose("background")}>{t("Keep in top bar")}</button>
    </div>
  </dialog>;
}
