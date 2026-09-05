import { memo } from "react";
import { Bell, FolderKanban, Layers, Search, Settings } from "lucide-react";
import type { TFunction } from "i18next";

type HeaderProps = {
  language: string;
  loading: boolean;
  activeView:
    | "myskills"
    | "explore"
    | "detail"
    | "settings"
    | "projects"
    | "explore-detail";
  /** Errors and warnings not yet seen in the notification panel. */
  unreadNotifications: number;
  onToggleLanguage: () => void;
  onOpenNotifications: () => void;
  onOpenSettings: () => void;
  onViewChange: (view: "myskills" | "explore" | "projects") => void;
  t: TFunction;
};

/** Past this the badge stops counting; the panel has the real number. */
const BADGE_MAX = 99;

const Header = ({
  language,
  activeView,
  unreadNotifications,
  onToggleLanguage,
  onOpenNotifications,
  onOpenSettings,
  onViewChange,
  t,
}: HeaderProps) => {
  const bellLabel =
    unreadNotifications > 0
      ? t("notifications.bellUnread", { count: unreadNotifications })
      : t("notifications.bell");
  return (
    <header className="skills-header">
      <div className="header-left">
        <div className="brand-area">
          <img className="logo-icon" src="/logo.png" alt="" />
          <div className="brand-text-wrap">
            <div className="brand-text">{t("appName")}</div>
          </div>
        </div>
        <nav className="nav-tabs">
          <button
            className={`nav-tab${activeView === "myskills" || activeView === "detail" ? " active" : ""}`}
            type="button"
            onClick={() => onViewChange("myskills")}
          >
            <Layers size={16} />
            {t("navMySkills")}
          </button>
          <button
            className={`nav-tab${activeView === "explore" || activeView === "explore-detail" ? " active" : ""}`}
            type="button"
            onClick={() => onViewChange("explore")}
          >
            <Search size={16} />
            {t("navExplore")}
          </button>
          <button
            className={`nav-tab${activeView === "projects" ? " active" : ""}`}
            type="button"
            onClick={() => onViewChange("projects")}
          >
            <FolderKanban size={16} />
            {t("navProjects")}
          </button>
        </nav>
      </div>
      <div className="header-actions">
        <button className="lang-btn" type="button" onClick={onToggleLanguage}>
          {language === "en" ? t("languageShort.en") : t("languageShort.zh")}
        </button>
        <button
          className="icon-btn notif-btn"
          type="button"
          onClick={onOpenNotifications}
          aria-label={bellLabel}
          title={bellLabel}
        >
          <Bell size={18} />
          {unreadNotifications > 0 ? (
            <span className="notif-badge" aria-hidden="true">
              {unreadNotifications > BADGE_MAX
                ? `${BADGE_MAX}+`
                : unreadNotifications}
            </span>
          ) : null}
        </button>
        <button
          className={`icon-btn${activeView === "settings" ? " active" : ""}`}
          type="button"
          onClick={onOpenSettings}
        >
          <Settings size={18} />
        </button>
      </div>
    </header>
  );
};

export default memo(Header);
