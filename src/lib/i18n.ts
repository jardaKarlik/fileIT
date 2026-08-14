// Site content dictionary for the CZ | EN language switch
export type Language = "cs" | "en";

export type Content = {
  nav: { home: string; contact: string; download: string };
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
  contact: {
    title: string;
    lead: string;
    form: {
      name: string;
      namePlaceholder: string;
      email: string;
      emailPlaceholder: string;
      subject: string;
      subjectPlaceholder: string;
      message: string;
      messagePlaceholder: string;
      submit: string;
      sending: string;
      success: string;
      successDetail: string;
      error: string;
      errors: {
        name: string;
        email: string;
        message: string;
      };
    };
    info: {
      heading: string;
      email: string;
      github: string;
      featurebase: string;
      response: string;
    };
  };
  footer: { publisher: string };
  faq: { heading: string; items: { q: string; a: string }[] };
};

const cs: Content = {
  nav: { home: "Domů", contact: "Kontakt", download: "Stáhnout FileIT" },
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
    heading: "Pro každého, kdo chce mít pořádek",
    text: "FileIT je navržený pro všechny, kdo spravují dokumenty — finanční poradce, malé firmy, ale i méně technicky zdatné uživatele, kteří hledají nástroj, který udělá většinu práce za ně. Znalost českých institucí a typů smluv je součástí aplikace.",
    pills: ["Finanční poradci", "Malé firmy", "Méně zdatní uživatelé"],
  },
  download: {
    heading: "Ke stažení",
    text: "Instalační balíček pro Windows 10 a 11 připravujeme. Podepsaná verze bude k dispozici zde.",
    button: "Stáhnout FileIT pro Windows",
    note: "Připravujeme · verze 1.0",
  },
  contact: {
    title: "Kontaktujte nás",
    lead: "Máte dotaz, nápad nebo potřebujete pomoc s FileIT? Napište nám — rádi se ozveme.",
    form: {
      name: "Jméno",
      namePlaceholder: "Jak vám můžeme říkat?",
      email: "E-mail",
      emailPlaceholder: "vas@email.cz",
      subject: "Předmět",
      subjectPlaceholder: "O čem je vaše zpráva?",
      message: "Zpráva",
      messagePlaceholder: "Napište nám, s čím potřebujete pomoci...",
      submit: "Odeslat zprávu",
      sending: "Odesílání...",
      success: "Zpráva odeslána",
      successDetail: "Děkujeme. Ozveme se vám co nejdříve.",
      error: "Odeslání se nezdařilo. Zkuste to prosím znovu.",
      errors: {
        name: "Jméno musí mít alespoň 2 znaky.",
        email: "Zadejte platný e-mail.",
        message: "Zpráva musí mít alespoň 10 znaků.",
      },
    },
    info: {
      heading: "Další kontakty",
      email: "E-mail",
      github: "GitHub",
      featurebase: "Hlasování o funkcích",
      response: "Odpovídáme obvykle do 1–2 pracovních dnů.",
    },
  },
  footer: { publisher: "Jaroslav Karlík | FileIT | Czech Republic" },
  faq: {
    heading: "Časté dotazy",
    items: [
      {
        q: "Co FileIT vlastně dělá?",
        a: "Projde vaše složky, přečte dokumenty a sám je roztřídí do přehledné struktury. Smlouvy, výpisy, pojistky a další důležité papíry najde bez toho, abyste museli cokoli ručně přepisovat.",
      },
      {
        q: "Je to bezpečné? Kam se dostanou moje dokumenty?",
        a: "Všechno probíhá přímo na vašem počítači. Dokumenty z něj neodcházejí, pokud je sami nepošlete do cloudu. Zálohu do cloudu provádíte vy, a je šifrovaná, takže ji přečtete jen vy.",
      },
      {
        q: "Musím být zkušený v počítačích?",
        a: "Vůbec ne. FileIT se ovládá klikáním a každý krok vysvětluje jednoduše. Vyberete složku, aplikace vám ukáže, co našla, a vy jen potvrdíte návrh.",
      },
      {
        q: "Co když FileIT něco roztřídí špatně?",
        a: "Jednoduše to opravíte. Aplikace se z vaší korekce naučí a příště bude chytřejší. Čím déle ji používáte, tím přesnější je.",
      },
      {
        q: "Jak funguje cloudová záloha?",
        a: "Roztříděné dokumenty můžete jedním kliknutím zálohovat do svého cloudu. Záloha je šifrovaná a uchovává historii změn, takže nic neztratíte a můžete se kdykoli vrátit zpět.",
      },
      {
        q: "Pro koho je FileIT určený?",
        a: "Pro každého, kdo chce mít pořádek v dokumentech — finanční poradce, živnostníky, seniory i běžné uživatele, kteří nechtějí trávit hodiny tříděním souborů.",
      },
    ],
  },
};

const en: Content = {
  nav: { home: "Home", contact: "Contact", download: "Download FileIT" },
  hero: {
    badge: "Windows desktop · version 1.0",
    statements: [
      "Thousands of documents. Order in minutes.",
      "Every client. Every contract. In its place.",
      "Classification without compromise.",
      "Cloud backup. Local and secure.",
    ],
    lead: "FileIT is a Windows desktop app for anyone who wants to bring order to a folder full of documents — financial advisors, small-business owners, and people who would rather not spend hours wrestling with files. It scans your archive, recognises what each file is, organises everything into a structure you understand, and backs it all up.",
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
    heading: "For anyone who wants their files sorted",
    text: "FileIT is built for anyone who manages documents — financial advisors, small businesses, and less tech-savvy users looking for a tool that does most of the work for them. Knowledge of Czech institutions and contract types is built in.",
    pills: ["Financial advisors", "Small businesses", "Less tech-savvy users"],
  },
  download: {
    heading: "Download",
    text: "The installer for Windows 10 and 11 is on the way. The signed release will be available here.",
    button: "Download FileIT for Windows",
    note: "Coming soon · version 1.0",
  },
  contact: {
    title: "Contact us",
    lead: "Have a question, idea, or need help with FileIT? Drop us a line — we'll get back to you.",
    form: {
      name: "Name",
      namePlaceholder: "What should we call you?",
      email: "Email",
      emailPlaceholder: "you@example.com",
      subject: "Subject",
      subjectPlaceholder: "What is your message about?",
      message: "Message",
      messagePlaceholder: "Tell us how we can help...",
      submit: "Send message",
      sending: "Sending...",
      success: "Message sent",
      successDetail: "Thank you. We'll be in touch as soon as possible.",
      error: "Submission failed. Please try again.",
      errors: {
        name: "Name must be at least 2 characters.",
        email: "Please enter a valid email.",
        message: "Message must be at least 10 characters.",
      },
    },
    info: {
      heading: "Other contacts",
      email: "Email",
      github: "GitHub",
      featurebase: "Feature voting",
      response: "We usually respond within 1–2 business days.",
    },
  },
  footer: { publisher: "Jaroslav Karlík | FileIT | Czech Republic" },
  faq: {
    heading: "Frequently asked questions",
    items: [
      {
        q: "What does FileIT actually do?",
        a: "It scans your folders, reads your documents and sorts them into a clear structure on its own. Contracts, statements, insurance policies and other important papers are found without any manual typing.",
      },
      {
        q: "Is it safe? Where do my documents go?",
        a: "Everything happens right on your computer. Your documents never leave it unless you choose to back them up to the cloud. Any cloud backup is done by you, encrypted, so only you can read it.",
      },
      {
        q: "Do I need to be good with computers?",
        a: "Not at all. FileIT is point-and-click and explains every step in plain language. You pick a folder, the app shows you what it found, and you simply confirm the proposal.",
      },
      {
        q: "What if FileIT sorts something wrong?",
        a: "You can correct it easily. The app learns from your feedback and gets smarter next time. The more you use it, the more accurate it becomes.",
      },
      {
        q: "How does cloud backup work?",
        a: "With one click, you can back up the sorted documents to your own cloud. The backup is encrypted and keeps a history of changes, so nothing is lost and you can revert at any time.",
      },
      {
        q: "Who is FileIT for?",
        a: "Anyone who wants their documents in order — financial advisors, self-employed people, seniors and everyday users who don't want to spend hours sorting files.",
      },
    ],
  },
};

export const content: Record<Language, Content> = { cs, en };
