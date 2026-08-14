// Site content dictionary for the CZ | EN language switch
export type Language = "cs" | "en";

export type Content = {
  nav: { download: string };
  hero: {
    badge: string;
    statements: string[];
    lead: string;
    download: string;
    secondary: string;
  };
  about: {
    pill: string;
    heading: string;
    cards: { key: string; title: string; text: string }[];
  };
  features: {
    heading: string;
    lead: string;
    cards: { key: string; title: string; text: string }[];
  };
  how: {
    pill: string;
    heading: string;
    steps: { key: string; label: string; text: string }[];
  };
  audience: { heading: string; text: string; pills: string[] };
  download: { heading: string; text: string; button: string; note: string };
  footer: { publisher: string };
};

const cs: Content = {
  nav: { download: "Stáhnout FileIT" },
  hero: {
    badge: "Windows desktop · verze 1.0",
    statements: [
      "Tisíce dokumentů. Řád za minuty.",
      "Každý klient. Každá smlouva. Na svém místě.",
      "Klasifikace bez kompromisů.",
      "Záloha do cloudu. Lokálně i bezpečně.",
    ],
    lead: "FileIT je desktopová aplikace pro Windows, která pomůže všem, kdo chtějí dostat pořádek do složek plných dokumentů — finančním poradcům, podnikatelům i lidem, kteří raději nebudou trávit hodiny tříděním souborů. Projde archiv, rozpozná obsah a uspořádá ho do struktury, které rozumíte, a vše zazálohuje.",
    download: "Stáhnout FileIT",
    secondary: "Jak to funguje",
  },
  about: {
    pill: "O aplikaci",
    heading: "Archiv, který se srovná sám",
    cards: [
      {
        key: "scan",
        title: "Naskenuje archiv",
        text: "Projde tisíce souborů ve vašich složkách a přečte jejich obsah přímo na vašem počítači.",
      },
      {
        key: "recognize",
        title: "Rozpozná dokumenty",
        text: "Určí typ dokumentu, instituci i klienta — bez ručního přepisování a bez tabulek.",
      },
      {
        key: "organize",
        title: "Uspořádá strukturu",
        text: "Navrhne a vytvoří přehlednou složkovou strukturu podle vašich vlastních pravidel.",
      },
      {
        key: "cloud",
        title: "Zazálohuje do cloudu",
        text: "Srovnaný archiv rovnou zálohuje do vašeho cloudu — šifrovaně a s kontrolou verzí. Nic se neztratí.",
      },
    ],
  },
  features: {
    heading: "Klíčové funkce",
    lead: "Vše, co potřebujete pro udržitelný pořádek v klientské dokumentaci.",
    cards: [
      {
        key: "classification",
        title: "Automatická klasifikace dokumentů",
        text: "Smlouvy, výpisy i dodatky se roztřídí samy podle naučených vzorů.",
      },
      {
        key: "institution",
        title: "Rozpoznání instituce",
        text: "Logo, firemní barvy a klíčová slova určí, od koho dokument přišel.",
      },
      {
        key: "classroom",
        title: "Učebna",
        text: "Systém doučíte pár kliknutími — vaše korekce se hned promítnou do výsledků.",
      },
      {
        key: "brain",
        title: "Centrální Mozek",
        text: "Sdílené vzory mezi uživateli zvyšují úspěšnost rozpoznání pro všechny.",
      },
      {
        key: "gdpr",
        title: "GDPR a compliance",
        text: "Klasifikace probíhá lokálně na vašem zařízení. Do cloudu jde jen to, co sami zvolíte.",
      },
      {
        key: "cloudBackup",
        title: "Cloudová záloha a auditní log",
        text: "Šifrovaná záloha do cloudu, každá změna zaznamenaná a kdykoli vratná.",
      },
    ],
  },
  how: {
    pill: "Jak to funguje",
    heading: "Tři kroky k pořádku",
    steps: [
      {
        key: "scan",
        label: "Naskenovat",
        text: "Vyberete složku s archivem. FileIT přečte obsah a připraví přehled nálezů.",
      },
      {
        key: "classify",
        label: "Klasifikovat",
        text: "Aplikace přiřadí typ, instituci a klienta. Nejasné případy vám nabídne ke schválení.",
      },
      {
        key: "organize",
        label: "Uspořádat",
        text: "Potvrdíte návrh a FileIT přesune dokumenty do finální struktury včetně cloudové zálohy.",
      },
    ],
  },
  audience: {
    heading: "Pro české finanční poradce",
    text: "FileIT je navržený pro samostatné poradce i poradenské kanceláře, které spravují stovky klientů a roky nahromaděné dokumentace. Znalost českých institucí a typů smluv je součástí aplikace.",
    pills: ["Samostatní poradci", "Poradenské kanceláře", "Back office týmy"],
  },
  download: {
    heading: "Ke stažení",
    text: "Instalační balíček pro Windows 10 a 11 připravujeme. Podepsaná verze bude k dispozici zde.",
    button: "Stáhnout FileIT pro Windows",
    note: "Připravujeme · verze 1.0",
  },
  footer: { publisher: "Jaroslav Karlík | FileIT | Czech Republic" },
};

const en: Content = {
  nav: { download: "Download FileIT" },
  hero: {
    badge: "Windows desktop · version 1.0",
    statements: [
      "Thousands of documents. Order in minutes.",
      "Every client. Every contract. In its place.",
      "Classification without compromise.",
      "Cloud backup. Local and secure.",
    ],
    lead: "FileIT is a Windows desktop app for Czech financial advisors and their younger colleagues. It goes through your client document archive, recognises what each file is, organises everything into a structure you understand — and backs it all up.",
    download: "Download FileIT",
    secondary: "How it works",
  },
  about: {
    pill: "About the app",
    heading: "An archive that sorts itself",
    cards: [
      {
        key: "scan",
        title: "Scans your archive",
        text: "It reads thousands of files across your folders, right on your own computer.",
      },
      {
        key: "recognize",
        title: "Recognises documents",
        text: "It identifies the document type, the institution and the client — no retyping, no spreadsheets.",
      },
      {
        key: "organize",
        title: "Organises the structure",
        text: "It proposes and builds a clear folder structure based on your own rules.",
      },
      {
        key: "cloud",
        title: "Backs up to the cloud",
        text: "The sorted archive is backed up to your cloud — encrypted and versioned. Nothing gets lost.",
      },
    ],
  },
  features: {
    heading: "Key features",
    lead: "Everything you need to keep client documentation in lasting order.",
    cards: [
      {
        key: "classification",
        title: "Automatic document classification",
        text: "Contracts, statements and amendments sort themselves using learned patterns.",
      },
      {
        key: "institution",
        title: "Institution recognition",
        text: "Logos, brand colours and keywords reveal who the document came from.",
      },
      {
        key: "classroom",
        title: "Classroom",
        text: "Teach the system in a few clicks — your corrections show up in results immediately.",
      },
      {
        key: "brain",
        title: "Central Brain",
        text: "Patterns shared between users raise recognition accuracy for everyone.",
      },
      {
        key: "gdpr",
        title: "GDPR and compliance",
        text: "Classification runs locally on your device. Only what you choose goes to the cloud.",
      },
      {
        key: "cloudBackup",
        title: "Cloud backup and audit log",
        text: "Encrypted cloud backup, every change logged and reversible at any time.",
      },
    ],
  },
  how: {
    pill: "How it works",
    heading: "Three steps to order",
    steps: [
      {
        key: "scan",
        label: "Scan",
        text: "Pick the folder with your archive. FileIT reads the content and prepares an overview.",
      },
      {
        key: "classify",
        label: "Classify",
        text: "The app assigns type, institution and client. Unclear cases come to you for approval.",
      },
      {
        key: "organize",
        label: "Organise",
        text: "Confirm the proposal and FileIT moves the documents into the final structure, cloud backup included.",
      },
    ],
  },
  audience: {
    heading: "For Czech financial advisors",
    text: "FileIT is built for solo advisors and advisory firms managing hundreds of clients and years of accumulated paperwork. Knowledge of Czech institutions and contract types is built in.",
    pills: ["Solo advisors", "Advisory firms", "Back office teams"],
  },
  download: {
    heading: "Download",
    text: "The installer for Windows 10 and 11 is on the way. The signed release will be available here.",
    button: "Download FileIT for Windows",
    note: "Coming soon · version 1.0",
  },
  footer: { publisher: "Jaroslav Karlík | FileIT | Czech Republic" },
};

export const content: Record<Language, Content> = { cs, en };
