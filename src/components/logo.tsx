type LogoTone = "light" | "dark";

export function Logo({ tone = "light" }: { tone?: LogoTone }) {
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
