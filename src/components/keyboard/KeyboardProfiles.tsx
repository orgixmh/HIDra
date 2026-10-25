import { useTranslation } from "../../i18n";
import type { Profile } from "../../types";
import { Panel } from "../ui/Panel";

interface KeyboardProfilesProps {
  profile: Profile;
  onProfile: (profile: Profile) => void;
  applied: string;
}

const PROFILES: Profile[] = [1, 2, 3];

export function KeyboardProfiles({
  profile,
  onProfile,
}: KeyboardProfilesProps) {
  const { t } = useTranslation();
  // onProfile switches the active profile on the keyboard (see App.selectProfile).
  const select = (p: Profile) => onProfile(p);

  return (
    <div className="tabgrid tabgrid--single">
      <Panel
        title={t("Perfis")}
        subtitle={t("Cada perfil guarda a própria iluminação e o próprio keymap.")}
      >
        <div className="profile-cards">
          {PROFILES.map((p) => (
            <button
              key={p}
              type="button"
              className={`profile-card${p === profile ? " is-active" : ""}`}
              onClick={() => select(p)}
            >
              <span className="profile-card__num">{p}</span>
              <span className="profile-card__name">{t("Perfil {profile}", { profile: p })}</span>
              <span className="profile-card__state">
                {p === profile ? t("Ativo") : t("Ativar")}
              </span>
            </button>
          ))}
        </div>
        <p className="note">
          {t("Ao trocar o perfil ativo, a edição das outras abas passa a apontar para ele.")}</p>
      </Panel>
    </div>
  );
}
