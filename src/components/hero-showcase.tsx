import dashboardAsset from "@/assets/Screenshot_2026-08-11_014203.png.asset.json";
import organizeAsset from "@/assets/Screenshot_2026-08-11_012747.png.asset.json";
import classroomAsset from "@/assets/Screenshot_2026-08-11_060852.png.asset.json";
import scanningAsset from "@/assets/Screenshot_2026-08-11_031644.png.asset.json";
import backupAsset from "@/assets/Screenshot_2026-08-11_022402.png.asset.json";

// Product-first hero collage: real app screenshots floating on the mesh gradient
export function HeroShowcase() {
  return (
    <div className="relative w-full">
      {/* Main app window */}
      <figure className="float-slow relative overflow-hidden rounded-2xl bg-card shadow-[var(--shadow-float)] ring-1 ring-primary-foreground/15">
        <img
          src={dashboardAsset.url}
          alt="Přehled naskenovaných souborů v aplikaci FileIT"
          loading="eager"
          className="w-full"
        />
      </figure>

      {/* Organize screen, offset behind-right */}
      <figure className="float-medium absolute -right-4 top-[58%] hidden w-[52%] overflow-hidden rounded-2xl bg-card shadow-[var(--shadow-float)] ring-1 ring-primary-foreground/15 sm:block">
        <img
          src={organizeAsset.url}
          alt="Návrh složkové struktury v FileIT"
          loading="lazy"
          className="w-full"
        />
      </figure>

      {/* Classroom cards, offset lower-left */}
      <figure className="float-fast absolute -left-6 top-[74%] hidden w-[44%] overflow-hidden rounded-2xl bg-card shadow-[var(--shadow-float)] ring-1 ring-primary-foreground/15 md:block">
        <img
          src={classroomAsset.url}
          alt="Učebna s nerozpoznanými soubory"
          loading="lazy"
          className="w-full"
        />
      </figure>

      {/* Scanning progress chip */}
      <figure className="float-medium absolute -left-10 -top-8 hidden w-[30%] overflow-hidden rounded-2xl bg-card shadow-[var(--shadow-card)] ring-1 ring-primary-foreground/15 lg:block">
        <img
          src={scanningAsset.url}
          alt="Průběh klasifikace souborů"
          loading="lazy"
          className="w-full"
        />
      </figure>

      {/* Backup confirmation strip */}
      <figure className="float-slow absolute -right-8 -top-10 hidden w-[38%] overflow-hidden rounded-xl bg-card shadow-[var(--shadow-card)] ring-1 ring-primary-foreground/15 lg:block">
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
