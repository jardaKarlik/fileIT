import { createFileRoute } from "@tanstack/react-router";
import {
  ArrowRight,
  Brain,
  Building2,
  CloudUpload,
  Download,
  FileStack,
  FolderTree,
  Github,
  GraduationCap,
  Heart,
  HelpCircle,
  ScanLine,
  ShieldCheck,
  Sparkles,
  Users,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useState } from "react";

import { HeroCarousel } from "@/components/hero-carousel";
import { HeroShowcase } from "@/components/hero-showcase";
import { PortraitRotator } from "@/components/portrait-rotator";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { StatusPill } from "@/components/status-pill";
import { Button } from "@/components/ui/button";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { content, type Language } from "@/lib/i18n";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "FileIT — Inteligentní správa dokumentů | file-app.uk" },
      {
        name: "description",
        content:
          "FileIT (file-app.uk) je desktopová aplikace pro Windows, která automaticky roztřídí a uspořádá archiv dokumentů. Lokálně, bezpečně, GDPR-ready.",
      },
      { property: "og:title", content: "FileIT — Inteligentní správa dokumentů" },
      {
        property: "og:description",
        content:
          "Automatická klasifikace dokumentů pro každého, kdo chce mít pořádek. Tisíce dokumentů, řád za minuty.",
      },
      { property: "og:type", content: "website" },
      { property: "og:url", content: "https://file-app.uk" },
      { name: "twitter:card", content: "summary_large_image" },
    ],
    links: [{ rel: "canonical", href: "https://file-app.uk" }],
  }),
  component: LandingPage,
});

// Icons are language-independent, matched to dictionary keys
const aboutIcons: Record<string, LucideIcon> = {
  scan: ScanLine,
  recognize: Sparkles,
  organize: FolderTree,
  cloud: CloudUpload,
};

const pillTones: ("green" | "pink" | "orange")[] = ["green", "pink", "orange"];

const featureIcons: Record<string, LucideIcon> = {
  classification: FileStack,
  institution: Building2,
  classroom: GraduationCap,
  brain: Brain,
  gdpr: ShieldCheck,
  cloudBackup: CloudUpload,
};

function LandingPage() {
  const [language, setLanguage] = useState<Language>("cs");
  const t = content[language];

  return (
    <div className="font-sans" lang={language === "cs" ? "cs" : "en"}>
      {/* Hero with animated mesh gradient */}
      <header className="mesh-bg relative overflow-hidden">
        <SiteHeader language={language} onChangeLanguage={setLanguage} t={t} />

        {/* Rotating atmospheric character layer on the left edge */}
        <PortraitRotator />

        <div className="relative z-10 mx-auto grid max-w-6xl items-center gap-16 px-6 pt-6 pb-28 lg:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]">
          <div>
            <StatusPill tone="green">{t.hero.badge}</StatusPill>
            <div className="mt-6">
              <HeroCarousel statements={t.hero.statements} />
            </div>
            <p className="mt-6 max-w-md text-base text-primary-foreground/80">{t.hero.lead}</p>
            <div className="mt-8 flex flex-wrap items-center gap-4">
              <Button
                asChild
                size="lg"
                className="rounded-full bg-brand-pink px-7 text-base font-bold text-primary-foreground shadow-[var(--shadow-float)] hover:bg-brand-pink-deep"
              >
                <a href="#ke-stazeni">
                  {t.hero.download} <ArrowRight className="ml-2 h-4 w-4" />
                </a>
              </Button>
              <a
                href="#o-aplikaci"
                className="text-sm font-semibold text-primary-foreground/85 underline-offset-4 hover:underline"
              >
                {t.hero.secondary}
              </a>
            </div>

            {/* GitHub + feature voting links below the main CTA */}
            <div className="mt-5 flex flex-wrap items-center gap-4">
              <a
                href="https://github.com/jardaKarlik/fileIT"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-2 rounded-full border border-primary-foreground/20 bg-primary-foreground/10 px-4 py-2 text-sm font-semibold text-primary-foreground backdrop-blur-sm transition-colors hover:bg-primary-foreground/20"
              >
                <Github className="h-4 w-4" />
                GitHub
              </a>
              <a
                href="https://fileit.featurebase.app/"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-2 rounded-full border border-primary-foreground/20 bg-primary-foreground/10 px-4 py-2 text-sm font-semibold text-primary-foreground backdrop-blur-sm transition-colors hover:bg-primary-foreground/20"
              >
                <Heart className="h-4 w-4" />
                {language === "cs" ? "Hlasovat o nových funkcích" : "Vote for new features"}
              </a>
            </div>
          </div>

          {/* Product screenshots are the focal point */}
          <div className="relative pb-28 sm:pb-36 md:pb-44 lg:pb-48">
            <HeroShowcase />
          </div>
        </div>
      </header>

      <main>
        {/* About */}
        <section id="o-aplikaci" className="bg-background py-20">
          <div className="mx-auto max-w-6xl px-6">
            <StatusPill tone="pink">{t.about.pill}</StatusPill>
            <h2 className="mt-4 max-w-2xl font-display text-3xl font-extrabold tracking-tight text-foreground sm:text-4xl">
              {t.about.heading}
            </h2>
            <div className="mt-10 grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
              {t.about.cards.map((card) => {
                const Icon = aboutIcons[card.key] ?? ScanLine;
                return (
                  <article
                    key={card.key}
                    className="rounded-2xl border border-border bg-card p-7 shadow-[var(--shadow-card)]"
                  >
                    <span className="grid h-11 w-11 place-items-center rounded-xl bg-brand-violet/10 text-brand-violet">
                      <Icon className="h-5 w-5" />
                    </span>
                    <h3 className="mt-5 font-display text-lg font-bold text-card-foreground">
                      {card.title}
                    </h3>
                    <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                      {card.text}
                    </p>
                  </article>
                );
              })}
            </div>
          </div>
        </section>

        {/* Features */}
        <section id="funkce" className="mesh-bg py-20">
          <div className="mx-auto max-w-6xl px-6">
            <h2 className="max-w-2xl font-display text-3xl font-extrabold tracking-tight text-primary-foreground sm:text-4xl">
              {t.features.heading}
            </h2>
            <p className="mt-3 max-w-xl text-primary-foreground/80">{t.features.lead}</p>
            <div className="mt-10 grid gap-6 md:grid-cols-2 lg:grid-cols-3">
              {t.features.cards.map((feature) => {
                const Icon = featureIcons[feature.key] ?? FileStack;
                return (
                  <article
                    key={feature.key}
                    className="rounded-2xl bg-card p-7 shadow-[var(--shadow-float)]"
                  >
                    <span className="grid h-11 w-11 place-items-center rounded-xl bg-brand-pink/12 text-brand-pink-deep">
                      <Icon className="h-5 w-5" />
                    </span>
                    <h3 className="mt-5 font-display text-lg font-bold text-card-foreground">
                      {feature.title}
                    </h3>
                    <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                      {feature.text}
                    </p>
                  </article>
                );
              })}
            </div>
          </div>
        </section>

        {/* How it works */}
        <section id="jak-to-funguje" className="bg-background py-20">
          <div className="mx-auto max-w-6xl px-6">
            <StatusPill tone="orange">{t.how.pill}</StatusPill>
            <h2 className="mt-4 font-display text-3xl font-extrabold tracking-tight text-foreground sm:text-4xl">
              {t.how.heading}
            </h2>
            <ol className="mt-12 grid gap-8 md:grid-cols-3">
              {t.how.steps.map((step, index) => (
                <li key={step.key} className="relative">
                  <div className="flex items-center gap-4">
                    <span className="grid h-12 w-12 shrink-0 place-items-center rounded-full bg-brand-violet font-display text-lg font-extrabold text-primary-foreground">
                      {index + 1}
                    </span>
                    <span className="h-px flex-1 bg-gradient-to-r from-brand-pink to-brand-coral" />
                  </div>
                  <h3 className="mt-5 font-display text-xl font-bold text-foreground">
                    {step.label}
                  </h3>
                  <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{step.text}</p>
                </li>
              ))}
            </ol>
          </div>
        </section>

        {/* For whom */}
        <section id="pro-koho" className="bg-secondary py-20">
          <div className="mx-auto max-w-4xl px-6 text-center">
            <span className="mx-auto grid h-12 w-12 place-items-center rounded-2xl bg-brand-pink/12 text-brand-pink-deep">
              <Users className="h-5 w-5" />
            </span>
            <h2 className="mt-6 font-display text-3xl font-extrabold tracking-tight text-foreground sm:text-4xl">
              {t.audience.heading}
            </h2>
            <p className="mx-auto mt-4 max-w-2xl text-base leading-relaxed text-muted-foreground">
              {t.audience.text}
            </p>
            <div className="mt-8 flex flex-wrap justify-center gap-3">
              {t.audience.pills.map((pill, index) => (
                <StatusPill key={pill} tone={pillTones[index] ?? "pink"}>
                  {pill}
                </StatusPill>
              ))}
            </div>
          </div>
        </section>

        {/* FAQ */}
        <section id="faq" className="bg-background py-20">
          <div className="mx-auto max-w-4xl px-6">
            <div className="text-center">
              <span className="mx-auto grid h-12 w-12 place-items-center rounded-2xl bg-brand-violet/10 text-brand-violet">
                <HelpCircle className="h-5 w-5" />
              </span>
              <h2 className="mt-6 font-display text-3xl font-extrabold tracking-tight text-foreground sm:text-4xl">
                {t.faq.heading}
              </h2>
            </div>
            <div className="mt-10 rounded-2xl border border-border bg-card p-6 shadow-[var(--shadow-card)] sm:p-8">
              <Accordion type="single" collapsible className="w-full">
                {t.faq.items.map((item, index) => (
                  <AccordionItem key={index} value={`item-${index}`}>
                    <AccordionTrigger className="text-base font-semibold text-card-foreground">
                      {item.q}
                    </AccordionTrigger>
                    <AccordionContent className="text-sm leading-relaxed text-muted-foreground">
                      {item.a}
                    </AccordionContent>
                  </AccordionItem>
                ))}
              </Accordion>
            </div>
          </div>
        </section>

        {/* Download */}
        <section id="ke-stazeni" className="mesh-bg py-24">
          <div className="mx-auto max-w-3xl px-6">
            <div className="rounded-3xl bg-card p-10 text-center shadow-[var(--shadow-float)]">
              <h2 className="font-display text-3xl font-extrabold tracking-tight text-card-foreground">
                {t.download.heading}
              </h2>
              <p className="mx-auto mt-3 max-w-lg text-sm leading-relaxed text-muted-foreground">
                {t.download.text}
              </p>
              <Button
                size="lg"
                disabled
                className="mt-8 rounded-full bg-brand-pink px-8 text-base font-bold text-primary-foreground hover:bg-brand-pink-deep"
              >
                <Download className="mr-2 h-4 w-4" /> {t.download.button}
              </Button>
              <p className="mt-4 text-xs text-muted-foreground">{t.download.note}</p>
            </div>
          </div>
        </section>
      </main>

      <SiteFooter t={t} />
    </div>
  );
}
