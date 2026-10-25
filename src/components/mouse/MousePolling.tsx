import { useTranslation } from "../../i18n";
import { useEffect, useState } from "react";
import type { MouseState } from "../../types";
import { mouseSetPolling } from "../../backend";
import { POLLING_RATES } from "../../constants";
import { useRun } from "../Toast";
import { Panel, Field } from "../ui/Panel";
import { SegmentedControl } from "../ui/SegmentedControl";

interface MousePollingProps {
  state: MouseState | null;
  applied: string;
}

export function MousePolling({ state, applied }: MousePollingProps) {
  const { t } = useTranslation();
  const run = useRun();
  const [hz, setHz] = useState<number>(1000);

  useEffect(() => {
    if (state?.polling != null) setHz(state.polling);
  }, [state]);

  const change = (value: number) => {
    setHz(value);
    void run(() => mouseSetPolling(value), applied);
  };

  return (
    <div className="tabgrid tabgrid--single">
      <Panel
        title={t("Taxa de polling")}
        subtitle={t("Frequência de reporte do mouse. Valores maiores reduzem a latência.")}
      >
        <Field label={t("Frequência")}>
          <SegmentedControl<number>
            value={hz}
            onChange={change}
            options={POLLING_RATES.map((r) => ({ value: r, label: `${r} Hz` }))}
          />
        </Field>
        <p className="note">
          {t("Selecionado: {hz} Hz · atualizações por segundo enviadas ao computador.", { hz })}
        </p>
      </Panel>
    </div>
  );
}
