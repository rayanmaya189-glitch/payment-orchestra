# 11 — SEO & Structured Data

## 1. Meta Tags (react-helmet-async)

```tsx
// src/components/seo/SEOHead.tsx
import { Helmet } from 'react-helmet-async';

interface SEOProps {
  title?: string;
  description?: string;
  image?: string;
  url?: string;
}

export function SEOHead({
  title = 'Payment Orchestration Platform — UAE',
  description = 'AI-native payment orchestration for UAE merchants. Connect multiple acquirers, intelligent failover, automated reconciliation. PCI-DSS compliant. No custody.',
  image = '/og-image.png',
  url = 'https://platform.ae',
}: SEOProps) {
  return (
    <Helmet>
      {/* Basic */}
      <title>{title} | Payment Orchestra</title>
      <meta name="description" content={description} />
      <meta name="keywords" content={[
        'payment orchestration UAE',
        'payment gateway UAE',
        'acquirer routing',
        'payment reconciliation',
        'AI payment assistant',
        'UAE payment platform',
        'merchant payment solutions',
        'payment failover',
        'PCI-DSS compliant payment',
        'UAE Central Bank regulated',
      ].join(', ')} />
      <link rel="canonical" href={url} />

      {/* Open Graph */}
      <meta property="og:type" content="website" />
      <meta property="og:title" content={title} />
      <meta property="og:description" content={description} />
      <meta property="og:url" content={url} />
      <meta property="og:site_name" content="Payment Orchestra" />
      <meta property="og:image" content={image} />
      <meta property="og:locale" content="en_AE" />

      {/* Twitter */}
      <meta name="twitter:card" content="summary_large_image" />
      <meta name="twitter:title" content={title} />
      <meta name="twitter:description" content={description} />
      <meta name="twitter:image" content={image} />

      {/* i18n */}
      <link rel="alternate" href="https://platform.ae/en" hrefLang="en" />
      <link rel="alternate" href="https://platform.ae/ar" hrefLang="ar" />
      <link rel="alternate" href="https://platform.ae" hrefLang="x-default" />
    </Helmet>
  );
}
```

---

## 2. Structured Data (JSON-LD)

```tsx
// src/components/seo/StructuredData.tsx
export function StructuredData() {
  const organization = {
    '@context': 'https://schema.org',
    '@type': 'Organization',
    name: 'Payment Orchestra',
    url: 'https://platform.ae',
    logo: 'https://platform.ae/logo.png',
    description: 'AI-native payment orchestration platform for UAE merchants.',
    address: {
      '@type': 'PostalAddress',
      addressCountry: 'AE',
      addressRegion: 'Dubai',
    },
    sameAs: [
      'https://twitter.com/paymentorchestra',
      'https://linkedin.com/company/paymentorchestra',
    ],
  };

  const software = {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: 'Payment Orchestra',
    applicationCategory: 'BusinessApplication',
    operatingSystem: 'Web',
    offers: {
      '@type': 'AggregateOffer',
      priceCurrency: 'AED',
      lowPrice: '0',
      highPrice: '499',
      offerCount: '3',
    },
    aggregateRating: {
      '@type': 'AggregateRating',
      ratingValue: '4.8',
      reviewCount: '150',
      bestRating: '5',
    },
  };

  const faq = {
    '@context': 'https://schema.org',
    '@type': 'FAQPage',
    mainEntity: [
      {
        '@type': 'Question',
        name: 'What payment acquirers do you support?',
        acceptedAnswer: {
          '@type': 'Answer',
          text: 'We support Network International, Checkout.com, Telr, PayTabs, Magnati, and more.',
        },
      },
      // ... other FAQs
    ],
  };

  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(organization) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(software) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(faq) }}
      />
    </>
  );
}
```

---

## 3. Sitemap (Static)

```typescript
// public/sitemap.xml (static file for Vite SPA)
// or generated via build script
```

### Build Script for Sitemap

```typescript
// scripts/generate-sitemap.ts
import fs from 'fs';

const baseUrl = 'https://platform.ae';
const pages = [
  { url: '/', priority: 1.0, changefreq: 'weekly' },
  { url: '/pricing', priority: 0.8, changefreq: 'monthly' },
  { url: '/features', priority: 0.8, changefreq: 'monthly' },
];

const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${pages.map(p => `  <url>
    <loc>${baseUrl}${p.url}</loc>
    <lastmod>${new Date().toISOString().split('T')[0]}</lastmod>
    <changefreq>${p.changefreq}</changefreq>
    <priority>${p.priority}</priority>
  </url>`).join('\n')}
</urlset>`;

fs.writeFileSync('public/sitemap.xml', sitemap);
```

---

## 4. robots.txt

```txt
# public/robots.txt
User-agent: *
Allow: /
Disallow: /api/
Disallow: /admin/

Sitemap: https://platform.ae/sitemap.xml
```

---

## 5. Page Speed Optimization Checklist

| Optimization | Implementation |
|--------------|---------------|
| Image optimization | Framer Motion lazy loading, WebP format, responsive srcset |
| Font optimization | Variable fonts, font-display: swap |
| Script optimization | Vite code splitting, lazy loading non-critical chunks |
| CSS optimization | Tailwind CSS purging unused styles via Vite |
| Static generation | Pre-render critical pages at build time |
| Compression | Brotli/Gzip via Vercel or nginx |
| Prefetching | React Router prefetch for critical navigation |
| Bundle analysis | Vite bundle analyzer for size optimization |
