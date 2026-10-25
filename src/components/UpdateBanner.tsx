import { useTranslation } from "../i18n";
import type { UpdateState } from "../hooks";
import type { AvailableUpdate, UpdateSupport } from "../updater";
import { downloadUrl, releaseNotesUrl } from "../updater";

interface UpdateBannerProps {
  state: UpdateState;
  support: UpdateSupport;
  onInstall: (update: AvailableUpdate) => void;
  onDismiss: () => void;
}

/**
 * The strip that appears above the workspace when a release is out.
 *
 * It offers to install only where HIDra can actually do it. A build we did not
 * package — a source checkout, a distro's own package — gets a download link
 * instead of a button that would fail.
 */
export function UpdateBanner({
  state,
  support,
  onInstall,
  onDismiss,
}: UpdateBannerProps) {
  const { t } = useTranslation();
  if (state.kind === "idle" || state.kind === "checking") return null;

  if (state.kind === "uptodate") {
    return (
      <div className="updatebar updatebar--quiet" role="status">
        <span className="updatebar__text">{t("Você está na versão mais recente.")}</span>
        <button type="button" className="btn btn--sm" onClick={onDismiss}>
          {t("Fechar")}</button>
      </div>
    );
  }

  if (state.kind === "error") {
    return (
      <div className="updatebar updatebar--error" role="alert">
        <span className="updatebar__text">
          {t("Não foi possível atualizar: {message}", { message: t(state.message) })}
        </span>
        <a
          className="btn btn--sm"
          href={`${downloadUrl(support, "latest")}`}
          target="_blank"
          rel="noreferrer"
        >
          {t("Baixar no site")}</a>
        <button type="button" className="btn btn--sm" onClick={onDismiss}>
          {t("Fechar")}</button>
      </div>
    );
  }

  if (state.kind === "installing") {
    const pct = state.progress === null ? null : Math.round(state.progress * 100);
    return (
      <div className="updatebar" role="status" aria-live="polite">
        <span className="updatebar__text">
          {pct === null ? t("Instalando atualização…") : t("Baixando atualização… {percent}%", { percent: pct })}
        </span>
        <span className="updatebar__progress" aria-hidden="true">
          <span
            className={`updatebar__bar${pct === null ? " is-indeterminate" : ""}`}
            style={pct === null ? undefined : { width: `${pct}%` }}
          />
        </span>
      </div>
    );
  }

  const { update } = state;
  const canInstall = support.can_install;
  return (
    <div className="updatebar" role="status">
      <span className="updatebar__text">
        <strong>{t("HIDra {version} disponível.", { version: update.version })}</strong>
      </span>
      <a
        className="updatebar__link"
        href={releaseNotesUrl(update.version)}
        target="_blank"
        rel="noreferrer"
      >
        {t("Ver novidades")}</a>
      {canInstall ? (
        <button
          type="button"
          className="btn btn--primary btn--sm"
          onClick={() => onInstall(update)}
          title={
            support.bundle === "deb" || support.bundle === "rpm"
              ? t("Baixa o pacote assinado e instala pelo gerenciador do sistema")
              : t("Baixa a nova versão assinada e reinicia o app")
          }
        >
          {t("Atualizar agora")}</button>
      ) : (
        <a
          className="btn btn--primary btn--sm"
          href={downloadUrl(support, update.version)}
          target="_blank"
          rel="noreferrer"
        >
          {t("Baixar")}</a>
      )}
      <button type="button" className="btn btn--sm" onClick={onDismiss}>
        {t("Depois")}</button>
    </div>
  );
}
