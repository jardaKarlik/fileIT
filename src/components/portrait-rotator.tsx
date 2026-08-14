import { useEffect, useState } from "react";

import portraitA from "@/assets/hero-portrait.jpg";
import portraitB from "@/assets/portrait-b.jpg";
import portraitC from "@/assets/portrait-c.jpg";
import portraitD from "@/assets/portrait-d.jpg";
import portraitE from "@/assets/portrait-e.jpg";
import portraitF from "@/assets/portrait-f.jpg";
import portraitG from "@/assets/portrait-g.jpg";
import portraitH from "@/assets/portrait-h.jpg";

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
];

export function PortraitRotator() {
  const [activeIndex, setActiveIndex] = useState(0);

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
      {portraits.map((portrait, index) => (
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

