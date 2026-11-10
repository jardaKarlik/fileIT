// src/components/HowItWorks/Banners.tsx
// Section banners for the "Jak to funguje" screen, each with an inline SVG
// illustration (no external assets — the app works fully offline).

import React from 'react';

type BannerProps = {
  id: string;
  eyebrow: string;
  title: string;
  lead: React.ReactNode;
  tone: 'privacy' | 'modes' | 'restructure';
  art: React.ReactNode;
  children?: React.ReactNode;
};

export function SectionBanner({ id, eyebrow, title, lead, tone, art, children }: BannerProps) {
  return (
    <section id={id} className={`how-banner ${tone}`} aria-labelledby={`${id}-title`}>
      <div className="how-banner-text">
        <div className="how-banner-eyebrow">{eyebrow}</div>
        <h3 className="how-banner-title" id={`${id}-title`}>{title}</h3>
        <div className="how-banner-lead">{lead}</div>
        {children}
      </div>
      <div className="how-banner-art" aria-hidden="true">{art}</div>
    </section>
  );
}

/** Laptop holding the documents, a shield with a lock, and a crossed-out cloud. */
export function PrivacyArt() {
  return (
    <svg viewBox="0 0 240 170" width="240" height="170">
      <defs>
        <linearGradient id="pv-shield" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#6B2AA2" />
          <stop offset="1" stopColor="#FF1493" />
        </linearGradient>
      </defs>
      {/* crossed-out cloud */}
      <g opacity="0.55">
        <path d="M168 34c0-10 8-18 18-18 8 0 14 5 17 12 9 0 16 7 16 16s-7 16-16 16h-33c-8 0-14-6-14-14 0-7 5-12 12-12z"
          fill="none" stroke="#8A7B9E" strokeWidth="3" />
        <line x1="166" y1="66" x2="222" y2="12" stroke="#FE865B" strokeWidth="4" strokeLinecap="round" />
      </g>
      {/* laptop */}
      <rect x="22" y="70" width="128" height="78" rx="8" fill="#FFFFFF" stroke="#D4CEE0" strokeWidth="3" />
      <rect x="32" y="80" width="108" height="58" rx="4" fill="#F5EBFF" />
      <path d="M10 150h152l-10 10H20z" fill="#D4CEE0" />
      {/* documents on screen */}
      <rect x="42" y="88" width="30" height="38" rx="3" fill="#FFFFFF" stroke="#D4CEE0" strokeWidth="2" />
      <line x1="48" y1="98" x2="66" y2="98" stroke="#C9C1D6" strokeWidth="2" />
      <line x1="48" y1="105" x2="66" y2="105" stroke="#C9C1D6" strokeWidth="2" />
      <line x1="48" y1="112" x2="60" y2="112" stroke="#C9C1D6" strokeWidth="2" />
      <rect x="80" y="92" width="30" height="38" rx="3" fill="#FFFFFF" stroke="#D4CEE0" strokeWidth="2" />
      <line x1="86" y1="102" x2="104" y2="102" stroke="#C9C1D6" strokeWidth="2" />
      <line x1="86" y1="109" x2="104" y2="109" stroke="#C9C1D6" strokeWidth="2" />
      {/* shield + lock */}
      <path d="M150 64l34 12v24c0 24-15 40-34 48-19-8-34-24-34-48V76z" fill="url(#pv-shield)" />
      <rect x="138" y="98" width="24" height="20" rx="4" fill="#FFFFFF" />
      <path d="M143 98v-6a7 7 0 0 1 14 0v6" fill="none" stroke="#FFFFFF" strokeWidth="4" />
      <circle cx="150" cy="107" r="3" fill="#4B0082" />
    </svg>
  );
}

/** The three modes: local disk, local + cloud, cloud to cloud. */
export function ModesArt() {
  return (
    <svg viewBox="0 0 240 170" width="240" height="170">
      <defs>
        <linearGradient id="md-a" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stopColor="#6B2AA2" /><stop offset="1" stopColor="#4B0082" /></linearGradient>
        <linearGradient id="md-b" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stopColor="#FF1493" /><stop offset="1" stopColor="#FF4FB0" /></linearGradient>
        <linearGradient id="md-c" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stopColor="#FE865B" /><stop offset="1" stopColor="#FF9E78" /></linearGradient>
      </defs>
      <path d="M58 98 C 90 40, 150 40, 182 98" fill="none" stroke="#E8E4F0" strokeWidth="4" strokeDasharray="6 7" />
      <path d="M58 112 C 100 150, 140 150, 182 112" fill="none" stroke="#E8E4F0" strokeWidth="4" strokeDasharray="6 7" />
      <circle cx="46" cy="104" r="34" fill="url(#md-a)" />
      <rect x="30" y="92" width="32" height="20" rx="3" fill="none" stroke="#FFFFFF" strokeWidth="3" />
      <line x1="26" y1="117" x2="66" y2="117" stroke="#FFFFFF" strokeWidth="3" strokeLinecap="round" />
      <circle cx="120" cy="52" r="30" fill="url(#md-b)" />
      <path d="M106 58c0-6 5-11 11-11 4 0 8 3 10 7 5 0 9 4 9 9s-4 9-9 9h-19c-5 0-8-4-8-8 0-3 3-6 6-6z" fill="none" stroke="#FFFFFF" strokeWidth="3" />
      <circle cx="194" cy="104" r="34" fill="url(#md-c)" />
      <path d="M180 98h28l-7-7M208 110h-28l7 7" fill="none" stroke="#FFFFFF" strokeWidth="3.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

/** Scattered documents flowing into a tidy folder tree. */
export function RestructureArt() {
  return (
    <svg viewBox="0 0 240 170" width="240" height="170">
      {/* scattered files */}
      <g transform="rotate(-14 40 60)"><rect x="22" y="36" width="34" height="44" rx="4" fill="#FFFFFF" stroke="#D4CEE0" strokeWidth="2.5" /><line x1="29" y1="48" x2="49" y2="48" stroke="#C9C1D6" strokeWidth="2" /><line x1="29" y1="56" x2="49" y2="56" stroke="#C9C1D6" strokeWidth="2" /></g>
      <g transform="rotate(10 60 110)"><rect x="44" y="88" width="34" height="44" rx="4" fill="#FFFFFF" stroke="#D4CEE0" strokeWidth="2.5" /><line x1="51" y1="100" x2="71" y2="100" stroke="#C9C1D6" strokeWidth="2" /><line x1="51" y1="108" x2="71" y2="108" stroke="#C9C1D6" strokeWidth="2" /></g>
      <g transform="rotate(-4 20 120)"><rect x="8" y="104" width="30" height="38" rx="4" fill="#FFE4F3" stroke="#FFB3DA" strokeWidth="2.5" /></g>
      {/* arrow */}
      <path d="M92 86h34" stroke="#FF1493" strokeWidth="4" strokeLinecap="round" />
      <path d="M120 76l12 10-12 10" fill="none" stroke="#FF1493" strokeWidth="4" strokeLinecap="round" strokeLinejoin="round" />
      {/* tree */}
      <path d="M150 34h22l6 6h34v18h-62z" fill="#4B0082" />
      <line x1="160" y1="58" x2="160" y2="140" stroke="#D4CEE0" strokeWidth="3" />
      {[74, 104, 134].map((y, i) => (
        <g key={y}>
          <line x1="160" y1={y} x2="176" y2={y} stroke="#D4CEE0" strokeWidth="3" />
          <path d={`M176 ${y - 9}h14l4 4h22v14h-40z`} fill={['#FF1493', '#6B2AA2', '#FE865B'][i]} />
        </g>
      ))}
    </svg>
  );
}
