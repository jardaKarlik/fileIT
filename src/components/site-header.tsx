import { Link } from "@tanstack/react-router";

import { Logo } from "./logo";
import { LanguageSwitch } from "./language-switch";
import type { Content, Language } from "@/lib/i18n";

export function SiteHeader({
  language,
  onChangeLanguage,
  t,
}: {
  language: Language;
  onChangeLanguage: (next: Language) => void;
  t: Content;
}) {
  return (
    <nav className="relative z-20 mx-auto flex max-w-6xl items-center justify-between px-6 py-6">
      <div>
        <Link to="/" className="block">
          <Logo />
        </Link>
        <span className="mt-1 block pl-13 text-xs font-medium tracking-widest text-primary-foreground/55">
          file-app.uk
        </span>
      </div>
      <div className="flex items-center gap-4">
        <div className="hidden items-center gap-4 text-sm font-semibold sm:flex">
          <Link
            to="/"
            activeProps={{ className: "text-primary-foreground" }}
            className="text-primary-foreground/85 transition-colors hover:text-primary-foreground"
          >
            {t.nav.home}
          </Link>
          <Link
            to="/kontakt"
            activeProps={{ className: "text-primary-foreground" }}
            className="text-primary-foreground/85 transition-colors hover:text-primary-foreground"
          >
            {t.nav.contact}
          </Link>
        </div>
        <LanguageSwitch language={language} onChange={onChangeLanguage} />
      </div>
    </nav>
  );
}
