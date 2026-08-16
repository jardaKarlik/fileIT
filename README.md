# FileIT: Smart Document Manager

Create a single-page marketing/product website for **FileIT** — a Windows desktop application for Czech financial advisors that intelligently scans, classifies, and reorganizes client document archives. The website exists primarily to satisfy a code signing certificate requirement (SignPath), so it must be clean, professional, and clearly identify the product and publisher.

## DESIGN SYSTEM — extract from the app's own UI:
- **Color palette**: Deep indigo/purple (#2D1B69, #4C1D95), hot pink/deep pink (#EC4899, #DB2777), coral/orange (#F97316), white text on dark sections
- **Gradients**: Animated mesh gradient background — purple → deep pink → coral/orange — similar to the Stripe gradient animation style. Use CSS animated gradient with keyframes (no external dependencies, pure CSS so it's cheap/static to host)
- **Typography**: Bold heavy headlines (800 weight), clean sans-serif body
- **Cards**: White/light cards with subtle shadows floating on the gradient, rounded corners (16px+)
- **Accent dots/pills**: Small colored indicator pills (green, orange, pink) used as status badges
- **Overall feel**: Modern SaaS, premium, feminine-leaning business aesthetic — think a businesswoman who means business. Confident, precise, elegant.

## HERO SECTION:
- Full-viewport hero with the animated mesh gradient background (purple → pink → coral)
- A **close-up face image placeholder** (professional businesswoman, confident look) that bleeds into the gradient — use a CSS mask/blend so the photo edge dissolves into the background. For stage 1, use a high-quality placeholder from Unsplash (search: businesswoman portrait professional)
- Over the image or beside it: a **rotating/fading text carousel** cycling through 3–4 short benefit statements:
  - "Tisíce dokumentů. Řád za minuty." 
  - "Každý klient. Každá smlouva. Na svém místě."
  - "Klasifikace bez kompromisů."
  - "GDPR-ready. Lokálně. Bezpečně."
- FileIT logo text top-left (use the pink/indigo color scheme, same style as app — square icon + wordmark)
- CTA button: "Stáhnout FileIT" (pink/coral, bold)

## SECTIONS (skeleton with placeholder text):
1. **Hero** — as described above
2. **O aplikaci** ("About") — 3-column card grid with icons: what FileIT does in 3 short bullets
3. **Klíčové funkce** ("Key Features") — 4–6 feature cards with icon + title + 1-line placeholder description:
   - Automatická klasifikace dokumentů
   - Rozpoznání instituce (logo, barvy, klíčová slova)
   - Učebna — trénink systému uživatelem
   - Centrální Mozek — sdílené vzory mezi uživateli
   - GDPR & compliance — vše lokálně, žádná data do cloudu
   - Záloha & auditní log
4. **Jak to funguje** ("How it works") — 3-step visual timeline: Naskenovat → Klasifikovat → Uspořádat
5. **Pro koho** ("For whom") — single focus: Czech financial advisors / IFA firms
6. **Ke stažení** ("Download") — simple download CTA block (placeholder button, no actual file yet)
7. **Footer** — publisher info: "Jaroslav Karlík | fileIT | Czech Republic" + GitHub placeholder link + contact email placeholder

## TECHNICAL REQUIREMENTS:
- Pure React + Tailwind (Lovable default stack)
- Animated CSS gradient — NO heavy JS animation libs. Use `@keyframes` in CSS for the mesh gradient background
- Responsive (mobile-friendly)
- All text in Czech (with English comments in code)
- No backend, no forms, no auth in stage 1
- Keep it static and deployable as a simple Lovable preview URL
- Add a `<meta>` description and proper `<title>FileIT — Inteligentní správa dokumentů</title>`

## STAGE 1 GOAL:
Deliver a skeleton where all sections exist with placeholder content, the gradient animation runs, the hero photo dissolves into background, and the text carousel rotates. Content quality doesn't matter — structure and visual language does. I'll review and iterate in stage 2.

This project was built with [Lovable](https://lovable.dev).

**Live app**: https://fileit-doc-organizer.lovable.app

## Build with Lovable

Continue developing this project in the [Lovable editor](https://lovable.dev/projects/e68d744f-33d8-4924-80d3-77d09538dc75).

- **Ship faster**: describe what you want to build and Lovable handles the code.
- **Stay in sync**: every change made in Lovable is committed straight to this repository.
- **Full ownership**: this code is yours. Push to `main` on GitHub and your changes sync back into Lovable, ready for your next prompt.

## Development

Prefer working locally? You need Node.js and npm — [install with nvm](https://github.com/nvm-sh/nvm#installing-and-updating).

```sh
git clone <this-repository-url>
cd <repository-name>
npm i
npm run dev
```
