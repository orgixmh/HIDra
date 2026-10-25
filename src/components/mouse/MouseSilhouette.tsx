import { useTranslation } from "../../i18n";
import type { Rgb } from "../../types";
import { rgbCss } from "../../hooks";

/** The RGB strip that wraps the base of the mouse. */
const STRIP = "M46 236 C58 268 78 282 100 282 C122 282 142 268 154 236";

interface MouseSilhouetteProps {
  color: Rgb;
  /** 0..255 — scales the glow intensity. */
  brightness?: number;
  /** When false, the RGB strip is drawn dark (effect "off"). */
  lit?: boolean;
}

export function MouseSilhouette({
  color,
  brightness = 255,
  lit = true,
}: MouseSilhouetteProps) {
  const { t } = useTranslation();
  const glow = lit ? Math.max(0.15, brightness / 255) : 0;
  const stripColor = lit ? rgbCss(color) : "#1a1c22";

  return (
    <svg
      className="mouse-svg"
      viewBox="0 0 200 300"
      role="img"
      aria-label={t("Pré-visualização do mouse")}
    >
      <defs>
        <linearGradient id="mouseBody" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#23262d" />
          <stop offset="1" stopColor="#141519" />
        </linearGradient>
        {/* The LED glow is a gradient halo rather than a feGaussianBlur filter:
            WebKitGTK composites filtered subtrees onto the wrong surface on
            some Linux GPU setups, painting black rectangles over the page. */}
        <radialGradient id="ledHalo">
          <stop offset="0" stopColor={stripColor} stopOpacity="0.5" />
          <stop offset="0.55" stopColor={stripColor} stopOpacity="0.16" />
          <stop offset="1" stopColor={stripColor} stopOpacity="0" />
        </radialGradient>
      </defs>

      {/* body */}
      <path
        d="M100 8
           C58 8 40 42 36 92
           C33 132 30 168 34 214
           C39 268 66 292 100 292
           C134 292 161 268 166 214
           C170 168 167 132 164 92
           C160 42 142 8 100 8 Z"
        fill="url(#mouseBody)"
        stroke="#2b2f37"
        strokeWidth="2"
      />

      {/* button split */}
      <path d="M100 12 L100 120" stroke="#2b2f37" strokeWidth="3" />
      <path
        d="M40 96 C60 108 140 108 160 96"
        fill="none"
        stroke="#2b2f37"
        strokeWidth="2"
      />

      {/* scroll wheel */}
      <rect x="90" y="52" width="20" height="40" rx="10" fill="#0e0f12" stroke="#33373f" strokeWidth="2" />
      {lit && <ellipse cx="100" cy="72" rx="38" ry="50" fill="url(#ledHalo)" opacity={glow} />}
      <rect x="93" y="56" width="14" height="32" rx="7" fill={stripColor} opacity={glow} />

      {/* RGB accent strip around the base — stacked strokes fake the bloom */}
      {lit &&
        [26, 16].map((w, i) => (
          <path
            key={w}
            d={STRIP}
            fill="none"
            stroke={stripColor}
            strokeWidth={w}
            strokeLinecap="round"
            opacity={glow * (i === 0 ? 0.12 : 0.26)}
          />
        ))}
      <path
        d={STRIP}
        fill="none"
        stroke={stripColor}
        strokeWidth="8"
        strokeLinecap="round"
        opacity={glow}
      />
      {/* logo dot */}
      {lit && <circle cx="100" cy="188" r="42" fill="url(#ledHalo)" opacity={glow} />}
      <circle cx="100" cy="188" r="15" fill={stripColor} opacity={glow} />
    </svg>
  );
}
