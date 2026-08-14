import { Github } from "lucide-react";
import { Link } from "@tanstack/react-router";

import { Logo } from "./logo";
import type { Content } from "@/lib/i18n";

export function SiteFooter({ t }: { t: Content }) {
  return (
    <footer className="bg-ink py-12">
      <div className="mx-auto flex max-w-6xl flex-col gap-6 px-6 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <Logo />
          <p className="mt-3 text-sm text-primary-foreground/70">
            {t.footer.publisher}
          </p>
          <a
            href="https://file-app.uk"
            className="mt-1 block text-xs tracking-widest text-primary-foreground/50 transition-colors hover:text-primary-foreground/80"
          >
            https://file-app.uk
          </a>
        </div>
        <div className="flex flex-col gap-2 text-sm text-primary-foreground/70 sm:items-end">
          <Link
            to="/kontakt"
            className="transition-colors hover:text-primary-foreground"
          >
            {t.nav.contact}
          </Link>
          <a
            href="https://github.com/jardaKarlik/fileIT"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 transition-colors hover:text-primary-foreground"
          >
            <Github className="h-4 w-4" />
            GitHub
          </a>
          <a
            href="mailto:info@file-app.uk"
            className="transition-colors hover:text-primary-foreground"
          >
            info@file-app.uk
          </a>
        </div>
      </div>
    </footer>
  );
}
