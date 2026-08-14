import { useEffect, useState } from "react";

// Rotating benefit statements, pure CSS fade animation per active item
const statements = [
  "Tisíce dokumentů. Řád za minuty.",
  "Každý klient. Každá smlouva. Na svém místě.",
  "Klasifikace bez kompromisů.",
  "GDPR-ready. Lokálně. Bezpečně.",
];

export function HeroCarousel() {
  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setActiveIndex((current) => (current + 1) % statements.length);
    }, 3800);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <div className="min-h-[7.5rem] sm:min-h-[9rem]">
      <p
        key={activeIndex}
        className="font-display text-3xl leading-tight font-extrabold text-primary-foreground sm:text-5xl lg:text-6xl"
        style={{ animation: "fade-slide 3.8s ease-in-out both" }}
      >
        {statements[activeIndex]}
      </p>
      <div className="mt-6 flex gap-2" aria-hidden="true">
        {statements.map((statement, index) => (
          <span
            key={statement}
            className={`h-1.5 rounded-full transition-all duration-500 ${
              index === activeIndex
                ? "w-10 bg-brand-pink"
                : "w-4 bg-primary-foreground/30"
            }`}
          />
        ))}
      </div>
    </div>
  );
}
