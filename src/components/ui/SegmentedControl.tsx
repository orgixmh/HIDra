import { useTranslation } from "../../i18n";
export interface SegmentOption<T extends string | number> {
  value: T;
  label: string;
}

interface SegmentedControlProps<T extends string | number> {
  options: SegmentOption<T>[];
  value: T;
  onChange: (value: T) => void;
  disabled?: boolean;
  size?: "sm" | "md";
}

export function SegmentedControl<T extends string | number>({
  options,
  value,
  onChange,
  disabled,
  size = "md",
}: SegmentedControlProps<T>) {
  const { t } = useTranslation();
  return (
    <div
      className={`segmented segmented--${size}${disabled ? " segmented--disabled" : ""}`}
      role="tablist"
    >
      {options.map((opt) => (
        <button
          key={String(opt.value)}
          type="button"
          role="tab"
          aria-selected={opt.value === value}
          className={`segmented__item${opt.value === value ? " is-active" : ""}`}
          disabled={disabled}
          onClick={() => onChange(opt.value)}
        >
          {t(opt.label)}
        </button>
      ))}
    </div>
  );
}
