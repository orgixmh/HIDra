import { useEffect, useId, useRef, useState } from "react";
import { callBackend, isTauri } from "../backend";
import { useTranslation } from "../i18n";
import { OSDSettings } from "./OSDSettings";
import { LanguageSelect } from "./LanguageSelect";

type StartupStatus = { enabled: boolean; supported: boolean; reason: string | null };
export function Settings() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<StartupStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const title = useId();
  const hint = useId();
  useEffect(() => {
    if (open && dialog.current && !dialog.current.open) dialog.current.showModal();
    else if (!open && dialog.current?.open) dialog.current.close();
    if (!open) return;
    let disposed = false;
    setError(null); setStatus(null);
    if (!isTauri()) {
      setStatus({ enabled: false, supported: false, reason: "Automatic startup is available in the desktop app." });
    } else {
      void callBackend<StartupStatus>("autostart_status").then((next) => { if (!disposed) setStatus(next); })
        .catch((err: unknown) => { if (!disposed) setError(String(err)); });
    }
    return () => { disposed = true; };
  }, [open]);
  const toggleStartup = async (enabled: boolean) => {
    setBusy(true); setError(null);
    try { setStatus(await callBackend<StartupStatus>("autostart_set", { enabled })); }
    catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  };
  return <>
    <button className="btn settings-button" type="button" onClick={() => setOpen(true)} aria-haspopup="dialog">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
        <path d="M9 3h6l.7 3 2.6 1.5 3 .9v6l-3 .9-2.6 1.5-.7 3H9l-.7-3-2.6-1.5-3-.9v-6l3-.9L8.3 6 9 3Z" /><circle cx="12" cy="11.4" r="3" />
      </svg>{t("Settings")}
    </button>
    <dialog className="close-dialog settings-dialog" ref={dialog} aria-labelledby={title} onClose={() => setOpen(false)}
      onCancel={(event) => { if (busy) event.preventDefault(); }}>
      <h2 id={title}>{t("Settings")}</h2>
      <LanguageSelect />
      <section className="settings-startup">
        <h3>{t("Startup")}</h3>
        <label className="settings-checkbox">
          <input type="checkbox" checked={status?.enabled ?? false} aria-describedby={hint}
            disabled={busy || !status || (!status.supported && !status.enabled)}
            onChange={(event) => void toggleStartup(event.target.checked)} />
          <span>{t("Start HIDra automatically when I sign in")}</span>
        </label>
        <p className="note" id={hint}>{t("HIDra will start in the top bar without opening the main window. Smart Notifications resumes if it was left on.")}</p>
        {!status && !error && <p className="note">{t("Loading startup settings…")}</p>}
        {status?.reason && <p className="note">{t(status.reason)}</p>}
        {error && <p className="smart-error" role="alert">{t(error)}</p>}
      </section>
      {open && <OSDSettings open={open} />}
      <div className="close-dialog__actions"><button className="btn btn--primary" type="button" disabled={busy} onClick={() => setOpen(false)}>{t("Done")}</button></div>
    </dialog>
  </>;
}
