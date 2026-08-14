import demoVideo from "@/assets/fileit-demo.mp4.asset.json";
import dashboardAsset from "@/assets/Screenshot_2026-08-11_014203.png.asset.json";
import backupAsset from "@/assets/Screenshot_2026-08-11_022402.png.asset.json";
import scanningAsset from "@/assets/Screenshot_2026-08-11_031644.png.asset.json";

// Product-first hero: looping 16s screen recording of FileIT, with small floating accents
export function HeroVideo() {
  return (
    <div className="relative w-full">
      <figure className="float-slow relative overflow-hidden rounded-2xl bg-card shadow-[var(--shadow-float)] ring-1 ring-primary-foreground/15">
        <video
          className="block w-full"
          src={demoVideo.url}
          poster={dashboardAsset.url}
          autoPlay
          muted
          loop
          playsInline
          preload="metadata"
          aria-label="Ukázka aplikace FileIT: skenování, klasifikace a uspořádání dokumentů"
        />
      </figure>

      {/* Scanning progress accent */}
      <figure className="float-medium absolute -left-10 -top-8 hidden w-[30%] overflow-hidden rounded-2xl bg-card shadow-[var(--shadow-card)] ring-1 ring-primary-foreground/15 lg:block">
        <img
          src={scanningAsset.url}
          alt="Průběh klasifikace souborů"
          loading="lazy"
          className="w-full"
        />
      </figure>

      {/* Backup confirmation accent */}
      <figure className="float-fast absolute -right-8 top-[78%] hidden w-[38%] overflow-hidden rounded-xl bg-card shadow-[var(--shadow-card)] ring-1 ring-primary-foreground/15 sm:block">
        <img
          src={backupAsset.url}
          alt="Potvrzení uložené zálohy"
          loading="lazy"
          className="w-full"
        />
      </figure>
    </div>
  );
}
