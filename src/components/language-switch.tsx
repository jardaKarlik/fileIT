import type { Language } from "@/lib/i18n";

// Compact CZ | EN toggle used in the hero nav
export function LanguageSwitch({
  language,
  onChange,
}: {
  language: Language;
  onChange: (next: Language) => void;
}) {
  const options: { value: Language; label: string }[] = [
    { value: "cs", label: "CZ" },
    { value: "en", label: "EN" },
  ];

  return (
    <div
      className="flex items-center gap-1 rounded-full border border-primary-foreground/25 bg-primary-foreground/10 p-1 backdrop-blur-sm"
      role="group"
      aria-label="Language"
    >
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          onClick={() => onChange(option.value)}
          aria-pressed={language === option.value}
          className={`rounded-full px-3 py-1 text-xs font-bold tracking-wide transition-colors ${
            language === option.value
              ? "bg-primary-foreground text-brand-indigo"
              : "text-primary-foreground/75 hover:text-primary-foreground"
          }`}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
