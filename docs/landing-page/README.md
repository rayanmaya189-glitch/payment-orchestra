# Landing Page Documentation

## Overview

Marketing landing page for the AI-Native Payment Orchestration Platform. The landing page is the first touchpoint for merchants in the UAE and GCC region, communicating the platform's value proposition, features, and trust signals.

## Tech Stack

```
Framework:      Next.js 14+ (App Router) — or Astro for static
Styling:        Tailwind CSS 4 + shadcn/ui
Animations:     Framer Motion
Analytics:      PostHog / Mixpanel
A/B Testing:    PostHog feature flags
Forms:          React Hook Form + Zod
i18n:           next-intl (Arabic + English, RTL support)
SEO:            Next.js metadata + structured data (JSON-LD)
```

## File Index

| File | Description |
|------|-------------|
| `README.md` | This file |
| `00-architecture.md` | Tech stack, project structure, SEO, i18n |
| `01-hero.md` | Hero section, CTA, trust badges |
| `02-features.md` | Feature showcase with animations |
| `03-how-it-works.md` | Step-by-step platform walkthrough |
| `04-integrations.md` | Acquirer/PSP partner logos and capabilities |
| `05-pricing.md` | Pricing tiers and comparison |
| `06-testimonials.md` | Customer testimonials and case studies |
| `07-security.md` | Security and compliance trust signals |
| `08-faq.md` | Frequently asked questions |
| `09-cta-sections.md` | Call-to-action sections throughout page |
| `10-footer.md` | Footer with links, legal, contact |
| `11-seo.md` | SEO metadata, structured data, Open Graph |

## Design Principles

1. **Trust-first**: Security and compliance badges prominently displayed
2. **UAE-localized**: Arabic RTL support, AED currency, local imagery
3. **Mobile-responsive**: Optimized for all device sizes
4. **Fast**: Static generation where possible, minimal JS
5. **Accessible**: WCAG 2.1 AA, semantic HTML, screen reader support
6. **Conversion-optimized**: Clear CTAs, social proof, risk reversal
