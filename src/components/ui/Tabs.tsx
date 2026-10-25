import { useTranslation } from "../../i18n";
export interface TabItem<T extends string> {
  id: T;
  label: string;
}

interface TabsProps<T extends string> {
  tabs: TabItem<T>[];
  active: T;
  onChange: (id: T) => void;
}

export function Tabs<T extends string>({ tabs, active, onChange }: TabsProps<T>) {
  const { t } = useTranslation();
  return (
    <nav className="tabs" role="tablist">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          aria-selected={tab.id === active}
          className={`tabs__item${tab.id === active ? " is-active" : ""}`}
          onClick={() => onChange(tab.id)}
        >
          {t(tab.label)}
        </button>
      ))}
    </nav>
  );
}
