import { createFileRoute } from "@tanstack/react-router";
import {
  ArrowRight,
  Brain,
  Building2,
  Download,
  FileStack,
  FolderTree,
  GraduationCap,
  ScanLine,
  ShieldCheck,
  Sparkles,
  Users,
} from "lucide-react";

import heroPortrait from "@/assets/hero-portrait.jpg";
import { HeroCarousel } from "@/components/hero-carousel";
import { HeroVideo } from "@/components/hero-video";
import { Button } from "@/components/ui/button";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "FileIT — Inteligentní správa dokumentů | file-app.uk" },
      {
        name: "description",
        content:
          "FileIT (file-app.uk) je desktopová aplikace pro Windows, která automaticky roztřídí a uspořádá archiv klientských dokumentů finančních poradců. Lokálně, bezpečně, GDPR-ready.",
      },
      { property: "og:title", content: "FileIT — Inteligentní správa dokumentů" },
      {
        property: "og:description",
        content:
          "Automatická klasifikace klientských dokumentů pro české finanční poradce. Tisíce dokumentů, řád za minuty.",
      },
      { property: "og:type", content: "website" },
      { property: "og:url", content: "https://file-app.uk" },
      { name: "twitter:card", content: "summary_large_image" },
    ],
    links: [{ rel: "canonical", href: "https://file-app.uk" }],
  }),
  component: LandingPage,
});

const aboutCards = [
  {
    icon: ScanLine,
    title: "Naskenuje archiv",
    text: "Projde tisíce souborů ve vašich složkách a přečte jejich obsah přímo na vašem počítači.",
  },
  {
    icon: Sparkles,
    title: "Rozpozná dokumenty",
    text: "Určí typ dokumentu, instituci i klienta — bez ručního přepisování a bez tabulek.",
  },
  {
    icon: FolderTree,
    title: "Uspořádá strukturu",
    text: "Navrhne a vytvoří přehlednou složkovou strukturu podle vašich vlastních pravidel.",
  },
];

const features = [
  {
    icon: FileStack,
    title: "Automatická klasifikace dokumentů",
    text: "Smlouvy, výpisy i dodatky se roztřídí samy podle naučených vzorů.",
  },
  {
    icon: Building2,
    title: "Rozpoznání instituce",
    text: "Logo, firemní barvy a klíčová slova určí, od koho dokument přišel.",
  },
  {
    icon: GraduationCap,
    title: "Učebna",
    text: "Systém doučíte pár kliknutími — vaše korekce se hned promítnou do výsledků.",
  },
  {
    icon: Brain,
    title: "Centrální Mozek",
    text: "Sdílené vzory mezi uživateli zvyšují úspěšnost rozpoznání pro všechny.",
  },
  {
    icon: ShieldCheck,
    title: "GDPR a compliance",
    text: "Vše probíhá lokálně na vašem zařízení. Žádná data neodcházejí do cloudu.",
  },
  {
    icon: Download,
    title: "Záloha a auditní log",
    text: "Každá změna je zaznamenaná a kdykoli vratná do původního stavu.",
  },
];

const steps = [
  {
    label: "Naskenovat",
    text: "Vyberete složku s archivem. FileIT přečte obsah a připraví přehled nálezů.",
  },
  {
    label: "Klasifikovat",
    text: "Aplikace přiřadí typ, instituci a klienta. Nejasné případy vám nabídne ke schválení.",
  },
  {
    label: "Uspořádat",
    text: "Potvrdíte návrh a FileIT přesune dokumenty do finální struktury včetně zálohy.",
  },
];

// Wordmark: "File" adapts to the background, "IT" always in the coral accent
function Logo({ tone = "light" }: { tone?: "light" | "dark" }) {
  return (
    <div className="flex items-center gap-3">
      <span className="grid h-10 w-10 place-items-center rounded-xl bg-brand-pink font-display text-lg font-extrabold text-primary-foreground shadow-[var(--shadow-card)]">
        F
      </span>
      <span
        className={`font-display text-xl font-extrabold tracking-tight ${
          tone === "dark" ? "text-brand-indigo" : "text-primary-foreground"
        }`}
      >
        File<span className="text-brand-coral">IT</span>
      </span>
    </div>
  );
}

function StatusPill({ tone, children }: { tone: "green" | "orange" | "pink"; children: string }) {
  const toneClass =
    tone === "green"
      ? "bg-status-green/15 text-status-green"
      : tone === "orange"
        ? "bg-brand-coral/15 text-brand-coral"
        : "bg-brand-pink/15 text-brand-pink-deep";

  return (
    <span
      className={`inline-flex items-center gap-2 rounded-full px-3 py-1 text-xs font-semibold ${toneClass}`}
    >
      <span className="h-1.5 w-1.5 rounded-full bg-current" />
      {children}
    </span>
  );
}

function LandingPage() {
  return (
    <div className="font-sans">
      {/* Hero with animated mesh gradient */}
      <header className="mesh-bg relative overflow-hidden">
        <nav className="relative z-20 mx-auto flex max-w-6xl items-center justify-between px-6 py-6">
          <div>
            <Logo />
            <span className="mt-1 block pl-13 text-xs font-medium tracking-widest text-primary-foreground/55">
              file-app.uk
            </span>
          </div>
          <Button
            asChild
            className="rounded-full bg-brand-pink px-5 font-semibold text-primary-foreground hover:bg-brand-pink-deep"
          >
            <a href="#ke-stazeni">Stáhnout FileIT</a>
          </Button>
        </nav>

        {/* Portrait as an atmospheric theme layer on the left edge */}
        <img
          src={heroPortrait}
          alt=""
          aria-hidden="true"
          width={1024}
          height={1280}
          className="portrait-edge pointer-events-none absolute -left-24 top-10 z-0 hidden h-[85%] w-[38%] object-cover object-[30%_25%] lg:block"
        />

        <div className="relative z-10 mx-auto grid max-w-6xl items-center gap-16 px-6 pt-6 pb-28 lg:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]">
          <div>
            <StatusPill tone="green">Windows desktop · verze 1.0</StatusPill>
            <div className="mt-6">
              <HeroCarousel />
            </div>
            <p className="mt-6 max-w-md text-base text-primary-foreground/80">
              FileIT je desktopová aplikace pro české finanční poradce a jejich mladší kolegy. Projde
              archiv klientských dokumentů, rozpozná jejich obsah a uspořádá je do struktury, které
              rozumíte a vše zazálohuje.
            </p>
            <div className="mt-8 flex flex-wrap items-center gap-4">
              <Button
                asChild
                size="lg"
                className="rounded-full bg-brand-pink px-7 text-base font-bold text-primary-foreground shadow-[var(--shadow-float)] hover:bg-brand-pink-deep"
              >
                <a href="#ke-stazeni">
                  Stáhnout FileIT <ArrowRight className="ml-2 h-4 w-4" />
                </a>
              </Button>
              <a
                href="#o-aplikaci"
                className="text-sm font-semibold text-primary-foreground/85 underline-offset-4 hover:underline"
              >
                Jak to funguje
              </a>
            </div>
          </div>

          {/* Product screenshots are the focal point */}
          <div className="relative pb-28 sm:pb-36 md:pb-44 lg:pb-48">
            <HeroVideo />
          </div>
        </div>
      </header>

      <main>
        {/* About */}
        <section id="o-aplikaci" className="bg-background py-20">
          <div className="mx-auto max-w-6xl px-6">
            <StatusPill tone="pink">O aplikaci</StatusPill>
            <h2 className="mt-4 max-w-2xl font-display text-3xl font-extrabold tracking-tight text-foreground sm:text-4xl">
              Archiv, který se srovná sám
            </h2>
            <div className="mt-10 grid gap-6 md:grid-cols-3">
              {aboutCards.map((card) => (
                <article
                  key={card.title}
                  className="rounded-2xl border border-border bg-card p-7 shadow-[var(--shadow-card)]"
                >
                  <span className="grid h-11 w-11 place-items-center rounded-xl bg-brand-violet/10 text-brand-violet">
                    <card.icon className="h-5 w-5" />
                  </span>
                  <h3 className="mt-5 font-display text-lg font-bold text-card-foreground">
                    {card.title}
                  </h3>
                  <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{card.text}</p>
                </article>
              ))}
            </div>
          </div>
        </section>

        {/* Features */}
        <section id="funkce" className="mesh-bg py-20">
          <div className="mx-auto max-w-6xl px-6">
            <h2 className="max-w-2xl font-display text-3xl font-extrabold tracking-tight text-primary-foreground sm:text-4xl">
              Klíčové funkce
            </h2>
            <p className="mt-3 max-w-xl text-primary-foreground/80">
              Vše, co potřebujete pro udržitelný pořádek v klientské dokumentaci.
            </p>
            <div className="mt-10 grid gap-6 md:grid-cols-2 lg:grid-cols-3">
              {features.map((feature) => (
                <article
                  key={feature.title}
                  className="rounded-2xl bg-card p-7 shadow-[var(--shadow-float)]"
                >
                  <span className="grid h-11 w-11 place-items-center rounded-xl bg-brand-pink/12 text-brand-pink-deep">
                    <feature.icon className="h-5 w-5" />
                  </span>
                  <h3 className="mt-5 font-display text-lg font-bold text-card-foreground">
                    {feature.title}
                  </h3>
                  <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                    {feature.text}
                  </p>
                </article>
              ))}
            </div>
          </div>
        </section>

        {/* How it works */}
        <section id="jak-to-funguje" className="bg-background py-20">
          <div className="mx-auto max-w-6xl px-6">
            <StatusPill tone="orange">Jak to funguje</StatusPill>
            <h2 className="mt-4 font-display text-3xl font-extrabold tracking-tight text-foreground sm:text-4xl">
              Tři kroky k pořádku
            </h2>
            <ol className="mt-12 grid gap-8 md:grid-cols-3">
              {steps.map((step, index) => (
                <li key={step.label} className="relative">
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
              Pro české finanční poradce
            </h2>
            <p className="mx-auto mt-4 max-w-2xl text-base leading-relaxed text-muted-foreground">
              FileIT je navržený pro samostatné poradce i poradenské kanceláře, které spravují
              stovky klientů a roky nahromaděné dokumentace. Znalost českých institucí a typů
              smluv je součástí aplikace.
            </p>
            <div className="mt-8 flex flex-wrap justify-center gap-3">
              <StatusPill tone="green">Samostatní poradci</StatusPill>
              <StatusPill tone="pink">Poradenské kanceláře</StatusPill>
              <StatusPill tone="orange">Back office týmy</StatusPill>
            </div>
          </div>
        </section>

        {/* Download */}
        <section id="ke-stazeni" className="mesh-bg py-24">
          <div className="mx-auto max-w-3xl px-6">
            <div className="rounded-3xl bg-card p-10 text-center shadow-[var(--shadow-float)]">
              <h2 className="font-display text-3xl font-extrabold tracking-tight text-card-foreground">
                Ke stažení
              </h2>
              <p className="mx-auto mt-3 max-w-lg text-sm leading-relaxed text-muted-foreground">
                Instalační balíček pro Windows 10 a 11 připravujeme. Podepsaná verze bude
                k dispozici zde.
              </p>
              <Button
                size="lg"
                disabled
                className="mt-8 rounded-full bg-brand-pink px-8 text-base font-bold text-primary-foreground hover:bg-brand-pink-deep"
              >
                <Download className="mr-2 h-4 w-4" /> Stáhnout FileIT pro Windows
              </Button>
              <p className="mt-4 text-xs text-muted-foreground">Připravujeme · verze 1.0</p>
            </div>
          </div>
        </section>
      </main>

      <footer className="bg-ink py-12">
        <div className="mx-auto flex max-w-6xl flex-col gap-6 px-6 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <Logo />
            <p className="mt-3 text-sm text-primary-foreground/70">
              Jaroslav Karlík | FileIT | Czech Republic
            </p>
            <a
              href="https://file-app.uk"
              className="mt-1 block text-xs tracking-widest text-primary-foreground/50 hover:text-primary-foreground/80"
            >
              https://file-app.uk
            </a>
          </div>
          <div className="flex flex-col gap-2 text-sm text-primary-foreground/70 sm:items-end">
            <a href="https://github.com" className="hover:text-primary-foreground">
              GitHub
            </a>
            <a href="mailto:info@file-app.uk" className="hover:text-primary-foreground">
              info@file-app.uk
            </a>
          </div>
        </div>
      </footer>
    </div>
  );
}
