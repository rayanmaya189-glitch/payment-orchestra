# 11 — SEO & Structured Data

## 1. Meta Tags

```typescript
// app/[locale]/page.tsx
import { Metadata } from 'next';

export const metadata: Metadata = {
  title: {
    default: 'Payment Orchestration Platform — UAE | Connect Once, Route Everywhere',
    template: '%s | Payment Orchestra',
  },
  description: 'AI-native payment orchestration for UAE merchants. Connect multiple acquirers, intelligent failover, automated reconciliation. PCI-DSS compliant. No custody.',
  keywords: [
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
  ],
  openGraph: {
    title: 'Payment Orchestration Platform — UAE',
    description: 'Connect once, route everywhere. AI-powered payment operations for UAE merchants.',
    url: 'https://platform.ae',
    siteName: 'Payment Orchestra',
    images: [
      {
        url: '/og-image.png',
        width: 1200,
        height: 630,
        alt: 'Payment Orchestra Platform',
      },
    ],
    locale: 'en_AE',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Payment Orchestration Platform — UAE',
    description: 'Connect once, route everywhere. AI-powered payment operations.',
    images: ['/og-image.png'],
  },
  alternates: {
    canonical: 'https://platform.ae',
    languages: {
      'en': '/en',
      'ar': '/ar',
    },
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      'max-video-preview': -1,
      'max-image-preview': 'large',
      'max-snippet': -1,
    },
  },
};
```

---

## 2. Structured Data (JSON-LD)

```typescript
// components/landing/StructuredData.tsx
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
    mainEntity: faqs.map(faq => ({
      '@type': 'Question',
      name: faq.question,
      acceptedAnswer: {
        '@type': 'Answer',
        text: faq.answer,
      },
    })),
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

## 3. Sitemap

```typescript
// app/sitemap.ts
import { MetadataRoute } from 'next';

export default function sitemap(): MetadataRoute.Sitemap {
  const baseUrl = 'https://platform.ae';
  
  return [
    {
      url: baseUrl,
      lastModified: new Date(),
      changeFrequency: 'weekly',
      priority: 1,
    },
    {
      url: `${baseUrl}/pricing`,
      lastModified: new Date(),
      changeFrequency: 'monthly',
      priority: 0.8,
    },
    {
      url: `${baseUrl}/features`,
      lastModified: new Date(),
      changeFrequency: 'monthly',
      priority: 0.8,
    },
    {
      url: `${baseUrl}/docs`,
      lastModified: new Date(),
      changeFrequency: 'weekly',
      priority: 0.6,
    },
  ];
}
```

---

## 4. Robots.txt

```typescript
// app/robots.ts
import { MetadataRoute } from 'next';

export default function robots(): MetadataRoute.Robots {
  return {
    rules: {
      userAgent: '*',
      allow: '/',
      disallow: ['/api/', '/admin/'],
    },
    sitemap: 'https://platform.ae/sitemap.xml',
  };
}
```

---

## 5. Page Speed Optimization Checklist

| Optimization | Implementation |
|--------------|---------------|
| Image optimization | Next.js `<Image>` with `priority` for above-fold |
| Font optimization | `next/font` for Inter + Noto Sans Arabic |
| Script optimization | `next/script strategy="lazy"` for analytics |
| CSS optimization | Tailwind CSS purging unused styles |
| Static generation | `generateStaticParams()` for locale pages |
| ISR | `revalidate: 3600` for content pages |
| Prefetching | `<Link prefetch>` for critical navigation |
| Compression | Brotli/Gzip via Vercel or nginx |
