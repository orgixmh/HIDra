import { useEffect, useId, useRef, useState } from "react";
import { isTauri } from "../../backend";
import { useTranslation } from "../../i18n";
import { KEYBOARD_ASPECT, KEYBOARD_LAYOUT } from "../../keyboardLayout";
import type { KeyCell } from "../../keyboardLayout";
import { rgbCss } from "../../hooks";
import type { NotificationRule, Rgb } from "../../types";
import type { SmartController } from "../../useSmartNotifications";
import { useToast } from "../Toast";
import { ColorPicker } from "../ui/ColorPicker";
import { Field, Panel } from "../ui/Panel";
import { Toggle } from "../ui/Toggle";

interface Props {
  controller: SmartController;
  layout?: KeyCell[];
  aspect?: number;
  supported: boolean;
  present: boolean;
}

export function SmartNotifications({ controller, layout, aspect, supported, present }: Props) {
  const { t } = useTranslation();
  const notify = useToast();
  const { view, error, save, test, clear, setBackground: applyBackground } = controller;
  const [rules, setRules] = useState<NotificationRule[]>([]);
  const [background, setBackground] = useState<Rgb>([24, 24, 32]);
  const pendingBackground = useRef<Rgb | null>(null);
  const sendingBackground = useRef(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [recentOpen, setRecentOpen] = useState(false);
  const recentDialog = useRef<HTMLDialogElement>(null);
  const recentId = useId();
  const recentTitle = useId();
  const recentDescription = useId();
  useEffect(() => {
    if (recentOpen && recentDialog.current && !recentDialog.current.open) recentDialog.current.showModal();
    else if (!recentOpen && recentDialog.current?.open) recentDialog.current.close();
  }, [recentOpen]);
  const saved = JSON.stringify(view?.settings.rules ?? []);
  useEffect(() => {
    const next = JSON.parse(saved) as NotificationRule[];
    setRules(next);
    setSelected((id) => next.some((rule) => rule.id === id) ? id : next[0]?.id ?? null);
  }, [saved]);
  const savedBackground = JSON.stringify(view?.settings.background ?? [24, 24, 32]);
  useEffect(() => {
    if (!sendingBackground.current) setBackground(JSON.parse(savedBackground) as Rgb);
  }, [savedBackground]);
  const keys = layout ?? KEYBOARD_LAYOUT;
  const draft = rules.find((rule) => rule.id === selected);
  const enabled = view?.settings.enabled ?? false;
  const changeBackground = (color: Rgb) => {
    setBackground(color);
    if (!enabled) return;
    pendingBackground.current = color;
    if (sendingBackground.current) return;
    sendingBackground.current = true;
    // Keep only the latest drag position while a frame is being transmitted.
    // Requests stay ordered without queuing every intermediate colour.
    void (async () => {
      try {
        while (pendingBackground.current) {
          const next = pendingBackground.current;
          pendingBackground.current = null;
          await applyBackground(next);
        }
      } catch (err) {
        pendingBackground.current = null;
        notify(String(err), "error");
      } finally {
        sendingBackground.current = false;
      }
    })();
  };
  const source = draft?.source === "script" ? "script" : draft?.app.trim() === "" && draft?.text.trim().toLowerCase() === "mail.google.com" ? "gmail" : "custom";
  const dirty = JSON.stringify(rules) !== saved || JSON.stringify(background) !== savedBackground;
  const valid = rules.length <= 64 && rules.every((rule) => rule.name.trim() &&
    (rule.source === "script" ? rule.script_path.startsWith("/") && rule.interval_seconds >= 5 && rule.interval_seconds <= 86400 : (rule.app.trim() || rule.text.trim())) && rule.keys.length > 0 && [0, 60, 300, 600].includes(rule.duration_seconds));
  const available = !isTauri() || (supported && present);
  const apps = [...new Set(["Viber", "Telegram", "Discord", "Slack", ...(view?.recent.map((event) => event.app) ?? [])])];
  const update = (change: Partial<NotificationRule>) => {
    setRules((previous) => previous.map((rule) => rule.id === selected ? { ...rule, ...change } : rule));
  };
  const action = async (run: () => Promise<unknown>, message: string) => {
    setBusy(true);
    try { await run(); notify(message, "success"); }
    catch (err) { notify(String(err), "error"); }
    finally { setBusy(false); }
  };
  const add = () => {
    const id = crypto.randomUUID();
    setRules((previous) => [...previous, { id, name: t("Rule {number}", { number: previous.length + 1 }),
      enabled: true, app: "", text: "", keys: [], color: [140, 80, 255], duration_seconds: 0, blinking: false, source: "notification", script_path: "", interval_seconds: 30 }]);
    setSelected(id);
  };
  const remove = () => {
    const next = rules.filter((rule) => rule.id !== selected);
    setRules(next); setSelected(next[0]?.id ?? null);
  };
  const toggle = (on: boolean) => {
    if (!view) return;
    // Off must always work, even if the unsaved rule being edited is incomplete.
    const settings = on ? { enabled: true, rules, background } : { ...view.settings, enabled: false };
    void action(() => save(settings), on ? "Smart Notifications enabled" : "Previous lighting restored");
  };
  const selectedNames = keys.filter((key) => draft?.keys.includes(key.off)).map((key) => t(key.label));

  return (
    <div className="smart-notifications">
      <Panel title={t("Smart Notifications")} subtitle={t("Assign desktop notifications to keyboard keys.")}
        actions={<Toggle checked={enabled} onChange={toggle} disabled={busy || !view || (!enabled && (!available || !valid))}
          label={enabled ? t("Ligado") : t("Desligado")} />}>
        <p className="note">{t("Smart Notifications replaces normal lighting effects with a solid background and notification colours. Turning it off restores your previous lighting.")}</p>
        <Field label={t("Background colour")} hint={t("All keys use this colour between alerts. Applied immediately while Smart Notifications is on.")}>
          <ColorPicker value={background} onChange={changeBackground} />
        </Field>
        <div className="smart-status" role="status">
          <span className={`led${view?.status === "listening" ? " led--on" : ""}`} />
          {t(view?.status === "listening" ? "Listening for desktop notifications" : view?.status === "starting" ? "Starting Smart Notifications…" : enabled ? "Notification listener needs attention" : "Smart Notifications is off")}
          {dirty && <span className="chip chip--muted">{t("Unsaved changes")}</span>}
        </div>
        <div className="smart-recent-toolbar">
        <button type="button" className="btn" disabled={busy || !view?.active || !view.lit_keys.length}
          onClick={() => void action(clear, "Alerts cleared")}>{t("Clear alerts")}</button>
          <button type="button" className="btn" aria-haspopup="dialog" aria-expanded={recentOpen}
            aria-controls={recentId} onClick={() => setRecentOpen(true)}>{t("Recent notifications")}</button>
        </div>
        {!available && <p className="note">{t("Smart Notifications requires a supported keyboard connected by USB or RF on Linux.")}</p>}
        {!isTauri() && <p className="note">{t("Browser preview: notification capture and keyboard output are simulated.")}</p>}
        {(error || view?.error) && <p className="smart-error" role="alert">{t(error ?? view?.error ?? "")}</p>}
        {view?.status === "error" && <button className="btn" type="button" disabled={busy || !view}
          onClick={() => void action(() => save(view.settings), "Smart Notifications reconnected")}>{t("Retry connection / recovery")}</button>}
      </Panel>

      <div className="smart-grid">
        <Panel title={t("Notification rules")} subtitle={t("Select a rule, then click the keys that should light up.")}
          actions={<button type="button" className="btn" disabled={busy || rules.length >= 64} onClick={add}>{t("Add rule")}</button>}>
          <div className="smart-rule-list">
            {rules.map((rule) => <button type="button" key={rule.id} className={`smart-rule${rule.id === selected ? " is-active" : ""}`}
              onClick={() => setSelected(rule.id)}>
              <span className="smart-colour" style={{ background: rgbCss(rule.color) }} />
              <span><strong>{rule.name}</strong><small>{rule.source === "script" ? t("Custom script") : rule.app.trim() === "" && rule.text.trim().toLowerCase() === "mail.google.com" ? t("Gmail (Browser)") : rule.app || rule.text || t("Configure a filter")}</small></span>
              <span className="chip chip--muted">{t("{count} keys", { count: rule.keys.length })}</span>
            </button>)}
            {!rules.length && <p className="note">{t("Add your first rule to assign notifications to keys.")}</p>}
          </div>
          <div className="smart-save">
            <button className="btn btn--primary" type="button" disabled={busy || !view || !valid || !dirty}
              onClick={() => void action(() => save({ enabled, rules, background }), "Notification rules saved")}>{t("Save rules")}</button>
            {!valid && <p className="note">{t("Each rule needs a name, a notification filter or script path, at least one key and a valid duration / interval.")}</p>}
          </div>
        </Panel>

        <Panel title={draft?.name || t("Rule settings")} subtitle={draft ? t("Choose keys for {name}", { name: draft.name }) : t("Select or add a rule first.")}>
          {draft && <>
          <div className="smart-keyboard-frame">
          <div className="kbd-stage smart-keyboard" style={{ aspectRatio: String(aspect ?? KEYBOARD_ASPECT) }}>
            {keys.map((key) => {
              const selectedKey = draft?.keys.includes(key.off) ?? false;
              return <button type="button" key={key.off} className={`kbd-key${selectedKey ? " kbd-key--lit smart-key--selected" : ""}`}
                aria-pressed={selectedKey} disabled={!draft || busy} title={t(key.label)}
                style={{ left: `${key.x}%`, top: `${key.y}%`, width: `${key.w}%`, height: `${key.h}%`, background: rgbCss(selectedKey ? draft.color : background) }}
                onClick={() => draft && update({ keys: selectedKey ? draft.keys.filter((off) => off !== key.off) : [...draft.keys, key.off] })}>
                <span>{t(key.label)}</span>
              </button>;
            })}
          </div>
          </div>
          <p className="note">{selectedNames.length ? t("Assigned keys: {keys}", { keys: selectedNames.join(", ") }) : t("Click keys to select or remove them from this rule.")}</p>
          <p className="note">{t("Keys return to the background colour after alerts. When rules overlap, the newest alert takes priority.")}</p>
          </>}
          {draft && <div className="smart-editor">
            <Field label={t("Rule name")}><input className="input" maxLength={100} value={draft.name} onChange={(event) => update({ name: event.target.value })} /></Field>
            <Toggle checked={draft.enabled} onChange={(on) => update({ enabled: on })} label={t("Rule enabled")} />
            <Field label={t("Notification source")}>
              <select className="input" value={source} onChange={(event) => update(event.target.value === "gmail"
                ? { source: "notification", app: "", text: "mail.google.com" } : event.target.value === "script"
                ? { source: "script", app: "", text: "", duration_seconds: 0 } : { source: "notification", app: "", text: "" })}>
                <option value="custom">{t("Custom application or website")}</option>
                <option value="gmail">{t("Gmail (Browser)")}</option>
                <option value="script">{t("Custom script")}</option>
              </select>
            </Field>
            {source === "script" ? <>
              <Field label={t("Script path")} hint={t("Enter the absolute path to an executable script with a shebang. No terminal is opened.")}>
                <input className="input" value={draft.script_path} placeholder="/home/user/scripts/check-vpn.sh" maxLength={4096}
                  onChange={(event) => update({ script_path: event.target.value })} />
              </Field>
              <Field label={t("Check interval (seconds)")} hint={t("Runs immediately, then waits this interval after each run. Maximum run time: 10 seconds.")}>
                <input className="input" type="number" min={5} max={86400} step={1} value={draft.interval_seconds}
                  onChange={(event) => update({ interval_seconds: Number(event.target.value) })} />
              </Field>
              <p className="note">{t("Exit code 1 triggers the alert. Exit code 0 clears it. Other exit codes or timeouts show an error and keep the last alert state.")}</p>
              <div className="smart-status" role="status">{(() => {
                const status = view?.scripts?.find((entry) => entry.id === draft.id);
                return status?.running ? t("Running script…") : status?.error ? <span className="smart-error">{t(status.error)}</span>
                  : status?.last_exit != null ? t("Last exit code: {code}", { code: status.last_exit }) : t("Script has not run yet");
              })()}</div>
            </> : source === "gmail" ? <p className="note">{t("Matches mail.google.com in the notification title or description, from any browser. No manual filter is needed.")}</p> : <>
            <Field label={t("Application contains")} hint={t("Leave empty to match any application or browser.")}>
              <input className="input" list="smart-apps" maxLength={200} value={draft.app} placeholder={t("Example: Viber or Telegram")}
                onChange={(event) => update({ app: event.target.value })} />
              <datalist id="smart-apps">{apps.map((app) => <option key={app} value={app} />)}</datalist>
            </Field>
            <Field label={t("Title or description contains")} hint={t("For websites, match text such as Facebook or Gmail. This works with any browser.")}>
              <input className="input" maxLength={300} value={draft.text} placeholder={t("Example: Facebook")}
                onChange={(event) => update({ text: event.target.value })} />
            </Field>
            <p className="note">{t("Matching ignores letter case. If both filters are filled in, both must match.")}</p>
            </>}
            <Field label={t("Alert colour")}><ColorPicker value={draft.color} onChange={(color) => update({ color })} /></Field>
            <Field label={t("Keep alert active")} hint={source === "script" ? t("The default keeps the key active until the script returns 0.") : undefined}>
              <select className="input" value={draft.duration_seconds} onChange={(event) => update({ duration_seconds: Number(event.target.value) })}>
                <option value={0}>{t(source === "script" ? "Until script returns 0" : "Until notification is dismissed")}</option>
                <option value={60}>{t("1 minute")}</option>
                <option value={300}>{t("5 minutes")}</option>
                <option value={600}>{t("10 minutes")}</option>
              </select>
            </Field>
            <Toggle checked={draft.blinking} onChange={(blinking) => update({ blinking })} label={t("Blink notification keys")} />
            <div className="btnrow">
              <button className="btn" type="button" disabled={busy} onClick={remove}>{t("Delete rule")}</button>
              <button className="btn" type="button" disabled={busy || dirty || !enabled || !view?.active}
                onClick={() => void action(() => test(draft.id), "Test notification sent")}>{t("Test rule")}</button>
            </div>
          </div>}
        </Panel>
      </div>

      <dialog className="close-dialog smart-recent-dialog" ref={recentDialog} id={recentId}
        aria-labelledby={recentTitle} aria-describedby={recentDescription} onClose={() => setRecentOpen(false)}>
        <header className="smart-recent-dialog__header">
          <div>
            <h2 id={recentTitle}>{t("Recent notifications")}</h2>
            <p className="note" id={recentDescription}>{t("Only kept in memory while HIDra is open. Use these details to build a matching rule.")}</p>
            {draft && <span className="chip chip--muted">{draft.name}</span>}
          </div>
          <button type="button" className="btn" onClick={() => setRecentOpen(false)}>{t("Fechar")}</button>
        </header>
        <div className="smart-recent-dialog__content">
        {!view?.recent.length && <p className="note">{t("Turn on Smart Notifications, then send a notification from an app or website.")}</p>}
        <div className="smart-recent">
          {view?.recent.map((event, index) => <div className="smart-recent-row" key={`${index}:${event.app}:${event.title}`}>
            <div><strong>{event.app || t("Unknown application")}</strong><span>{event.title}</span><p>{event.body}</p></div>
            <div className="smart-recent-actions">
              <span className={`chip${event.matched ? " chip--on" : " chip--muted"}`}>{t(event.matched ? "Matched" : "No matching rule")}</span>
              <button className="btn btn--sm" type="button" disabled={!draft || busy} onClick={() => update({ source: "notification", app: event.app, text: source === "gmail" || source === "script" ? "" : draft?.text ?? "" })}>{t("Use application")}</button>
              <button className="btn btn--sm" type="button" disabled={!draft || busy} onClick={() => update({ source: "notification", text: event.title })}>{t("Use title")}</button>
              <button className="btn btn--sm" type="button" disabled={!draft || busy || !event.body} onClick={() => update({ source: "notification", text: event.body.slice(0, 300) })}>{t("Use description")}</button>
            </div>
          </div>)}
        </div>

        </div>
      </dialog>
    </div>
  );
}
