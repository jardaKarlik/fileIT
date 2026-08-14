type Tone = "green" | "orange" | "pink";

export function StatusPill({ tone, children }: { tone: Tone; children: string }) {
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
