import { useTranslation } from "../../i18n";
import type { MouseState } from "../../types";
import { useLocalStorage } from "../../hooks";
import { Tabs } from "../ui/Tabs";
import type { TabItem } from "../ui/Tabs";
import { MouseLighting } from "./MouseLighting";
import { MouseDpi } from "./MouseDpi";
import { MouseButtons } from "./MouseButtons";
import { MousePolling } from "./MousePolling";

type MouseTab = "led" | "dpi" | "buttons" | "polling";

const TABS: TabItem<MouseTab>[] = [
  { id: "led", label: "Iluminação" },
  { id: "dpi", label: "DPI" },
  { id: "buttons", label: "Botões" },
  { id: "polling", label: "Polling" },
];

interface MousePanelProps {
  state: MouseState | null;
  /** Model of the mouse that was found, or a generic label when none is. */
  name: string;
  present: boolean;
  applied: string;
  onApplied: () => void;
}

export function MousePanel({ state, name, present, applied, onApplied }: MousePanelProps) {
  const { t } = useTranslation();
  const [tab, setTab] = useLocalStorage<MouseTab>("hidra.mouseTab", "led");

  return (
    <div className="workspace">
      <div className="workspace__top">
        <div className="workspace__heading">
          <h1>{t(name)}</h1>
          <span className={`chip${present ? " chip--on" : " chip--off"}`}>
            {present ? t("Conectado") : t("Desconectado")}
          </span>
        </div>
        <Tabs tabs={TABS} active={tab} onChange={setTab} />
      </div>

      {!present && (
        <p className="disconnected-note">
          {t("Mouse não detectado. Conecte o receptor 2.4G ou o cabo para aplicar alterações — os controles abaixo continuam editáveis.")}</p>
      )}

      <div className="workspace__body">
        {tab === "led" && (
          <MouseLighting state={state} applied={applied} onApplied={onApplied} />
        )}
        {tab === "dpi" && (
          <MouseDpi state={state} applied={applied} onApplied={onApplied} />
        )}
        {tab === "buttons" && <MouseButtons state={state} applied={applied} />}
        {tab === "polling" && <MousePolling state={state} applied={applied} />}
      </div>
    </div>
  );
}
