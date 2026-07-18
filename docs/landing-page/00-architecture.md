# 00 — Landing Page Architecture

## 1. Tech Stack

```json
{
  "framework": "Next.js 14+ (App Router)",
  "styling": "Tailwind CSS 4 + shadcn/ui",
  "animations": "Framer Motion",
  "analytics": "PostHog",
  "forms": "React Hook Form + Zod",
  "i18n": "next-intl (Arabic + English)",
  "seo": "Next.js metadata + JSON-LD structured data",
  "icons": "Lucide React",
  "images": "Next.js Image (optimized)"
}
```

### Why This Stack

| Choice | Rationale |
|--------|-----------|
| **Next.js** | SSR for SEO, static generation for performance, image optimization |
| **Tailwind 4** | Rapid prototyping, consistent design system |
| **Framer Motion** | Smooth animations for feature showcases and scroll reveals |
| **next-intl** | Mature Arabic RTL support, lazy loading, SEO-friendly |
| **PostHog** | Product analytics, A/B testing, session recording |

---

## 2. Project Structure

```
landing-page/
├── app/
│   ├── [locale]/                    # i18n routes
│   │   ├── layout.tsx               # Root layout with metadata
│   │   ├── page.tsx                 # Main landing page
│   │   ├── pricing/page.tsx         # Pricing page
│   │   └── features/page.tsx        # Features detail page
│   └── api/
│       └── contact/route.ts         # Contact form API
├── components/
│   ├── landing/
│   │   ├── Hero.tsx
│   │   ├── Features.tsx
│   │   ├── HowItWorks.tsx
│   │   ├── Integrations.tsx
│   │   ├── Pricing.tsx
│   │   ├── Testimonials.tsx
│   │   ├── Security.tsx
│   │   ├── FAQ.tsx
│   │   ├── CTASection.tsx
│   │   └── Footer.tsx
│   ├── ui/                          # shadcn/ui components
│   └── shared/                      # Shared components
├── lib/
│   ├── i18n/
│   └── analytics.ts
├── public/
│   ├── images/
│   └── icons/
└── styles/
    └── globals.css
```

---

## 3. SEO Strategy

```typescript
// app/[locale]/layout.tsx
export const metadata: Metadata = {
  title: 'Payment Orchestration Platform — UAE',
  description: 'AI-native payment orchestration for UAE merchants. Connect multiple acquirers, intelligent failover, automated reconciliation.',
  openGraph: {
    title: 'Payment Orchestration Platform',
    description: 'Connect once, route everywhere. AI-powered payment operations.',
    images: ['/og-image.png'],
    locale: 'en_AE',
    type: 'website',
  },
  alternates: {
    canonical: 'https://platform.ae',
    languages: {
      'en': '/en',
      'ar': '/ar',
    },
  },
};
```

### Structured Data (JSON-LD)

```json
{
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  "name": "Payment Orchestration Platform",
  "applicationCategory": "BusinessApplication",
  "operatingSystem": "Web",
  "offers": {
    "@type": "Offer",
    "priceCurrency": "AED",
    "price": "0",
    "description": "Free tier available"
  },
  "aggregateRating": {
    "@type": "AggregateRating",
    "ratingValue": "4.8",
    "reviewCount": "150"
  }
}
```

---

## 4. i18n Configuration

```typescript
// lib/i18n/index.ts
import { getRequestConfig } from 'next-intl/server';

export default getRequestConfig(async ({ locale }) => ({
  messages: (await import(`../../messages/${locale}.json`)).default,
}));

// RTL support
// app/[locale]/layout.tsx
<html lang={locale} dir={locale === 'ar' ? 'rtl' : 'ltr'}>
```

---

## 5. Performance Targets

| Metric | Target |
|--------|--------|
| Lighthouse Performance | > 95 |
| LCP | < 2.5s |
| FID | < 100ms |
| CLS | < 0.1 |
| Time to Interactive | < 3s |
| Bundle Size | < 200KB gzipped |
