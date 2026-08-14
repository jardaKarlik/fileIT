import { useEffect, useMemo, useState } from "react";

import portraitA from "@/assets/hero-portrait.jpg";
import portraitB from "@/assets/portrait-b.jpg";
import portraitC from "@/assets/portrait-c.jpg";
import portraitD from "@/assets/portrait-d.jpg";
import portraitE from "@/assets/portrait-e.jpg";
import portraitF from "@/assets/portrait-f.jpg";
import portraitG from "@/assets/portrait-g.jpg";
import portraitH from "@/assets/portrait-h.jpg";
import portraitI from "@/assets/portrait-i.jpg.asset.json";
import portraitJ from "@/assets/portrait-j.png.asset.json";
import portraitK from "@/assets/portrait-k.jpg.asset.json";

// Atmospheric character layer: diverse ages and roles, some drift, some stay still
const portraits = [
  { src: portraitA, motion: true, position: "object-[30%_25%]" },
  { src: portraitB, motion: false, position: "object-[45%_20%]" },
  { src: portraitC, motion: true, position: "object-[40%_30%]" },
  { src: portraitD, motion: false, position: "object-[50%_22%]" },
  { src: portraitE, motion: true, position: "object-[35%_25%]" },
  { src: portraitF, motion: false, position: "object-[55%_25%]" },
  { src: portraitG, motion: true, position: "object-[45%_25%]" },
  { src: portraitH, motion: false, position: "object-[40%_22%]" },
  { src: portraitI.url, motion: true, position: "object-[50%_20%]" },
  { src: portraitJ.url, motion: false, position: "object-[50%_25%]" },
  { src: portraitK.url, motion: true, position: "object-[45%_20%]" },
];

// Fisher-Yates shuffle so the character order differs on every visit
function shuffle<T>(items: readonly T[]): T[] {
  const copy = [...items];
  for (let i = copy.length - 1; i > 0; i -= 1) {
    const j = Math.floor(Math.random() * (i + 1));
    [copy[i], copy[j]] = [copy[j], copy[i]];
  }
  return copy;
}

export function PortraitRotator() {
  const [activeIndex, setActiveIndex] = useState(0);
  const [randomized, setRandomized] = useState(false);
  // Keep the SSR/first paint order stable, then randomize after hydration
  const order = useMemo(() => (randomized ? shuffle(portraits) : portraits), [randomized]);

  useEffect(() => {
    setRandomized(true);
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setActiveIndex((current) => (current + 1) % portraits.length);
    }, 5200);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <div
      aria-hidden="true"
      className="portrait-edge pointer-events-none absolute -left-24 top-10 z-0 hidden h-[85%] w-[38%] lg:block"
    >
      {order.map((portrait, index) => (
        <img
          key={portrait.src}
          src={portrait.src}
          alt=""
          width={1024}
          height={1280}
          loading={index === 0 ? "eager" : "lazy"}
          className={`absolute inset-0 h-full w-full object-cover transition-opacity duration-[1600ms] ease-in-out ${
            portrait.position
          } ${index === activeIndex ? "opacity-100" : "opacity-0"} ${
            portrait.motion ? "portrait-drift" : ""
          }`}
        />
      ))}
    </div>
  );
}
