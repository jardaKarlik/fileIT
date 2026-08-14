import { useEffect, useState } from "react";

// Rotating benefit statements, cross-faded via stacked layers (no remount jump)
const defaultStatements = [
  "Tisíce dokumentů. Řád za minuty.",
  "Každý klient. Každá smlouva. Na svém místě.",
  "Klasifikace bez kompromisů.",
  "GDPR-ready. Lokálně. Bezpečně.",
];

export function HeroCarousel({ statements = defaultStatements }: { statements?: string[] }) {
  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    setActiveIndex(0);
  }, [statements]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setActiveIndex((current) => (current + 1) % statements.length);
    }, 6500);
    return () => window.clearInterval(timer);
  }, [statements]);

  return (
    <div>
      {/* Layers share the same grid cell, so the tallest statement defines the height */}
      <div className="grid">
        {statements.map((statement, index) => {
          const isActive = index === activeIndex;
          return (
            <p
              key={statement}
              aria-hidden={!isActive}
              className={`col-start-1 row-start-1 font-display text-3xl leading-tight font-extrabold text-primary-foreground transition-[opacity,transform,filter] duration-[1400ms] ease-[cubic-bezier(0.22,1,0.36,1)] sm:text-5xl lg:text-6xl ${
                isActive
                  ? "opacity-100 blur-0 translate-y-0"
                  : "pointer-events-none opacity-0 blur-[2px] translate-y-2"
              }`}
            >
              {statement}
            </p>
          );
        })}
      </div>
      <div className="mt-6 flex gap-2" aria-hidden="true">
        {statements.map((statement, index) => (
          <span
            key={statement}
            className={`h-1.5 rounded-full transition-all duration-1000 ease-out ${
              index === activeIndex ? "w-10 bg-brand-pink" : "w-4 bg-primary-foreground/30"
            }`}
          />
        ))}
      </div>
    </div>
  );
}
