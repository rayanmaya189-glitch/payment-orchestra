# 00 — Landing Page Architecture

## 1. Tech Stack

```json
{
  "framework": "React 19 + React Router DOM 7",
  "build": "Vite 8",
  "styling": "Tailwind CSS 4",
  "animations": "Framer Motion",
  "state": {
    "server": "TanStack React Query v5",
    "client": "Zustand v5"
  },
  "http": "Axios v1.x",
  "storage": "Session Storage (encrypted)",
  "forms": "Zod validation",
  "i18n": "react-i18next (Arabic + English)",
  "icons": "Lucide React"
}
```

### Why This Stack

| Choice | Rationale |
|--------|-----------|
| **React 19** | Server components not needed for landing page; React 19 perf improvements |
| **Vite 8** | Fast HMR, native ESM, optimized builds |
| **Tailwind 4** | CSS-first config, rapid prototyping, consistent design system |
| **Framer Motion** | Smooth animations for feature showcases and scroll reveals |
| **Axios** | Request/response interceptors for analytics, error handling |
| **Session Storage** | Encrypted tokens for analytics tracking |
| **Zod** | Runtime type validation for contact form inputs |
| **react-i18next** | Mature Arabic RTL support, lazy loading of translation bundles |

---

## 2. Project Structure

```
landing-page/
├── src/
│   ├── main.tsx                        # Vite entry point
│   ├── App.tsx                         # Root component with providers
│   ├── routes/                         # React Router routes
│   │   ├── __root.tsx                  # Root layout (theme, i18n)
│   │   ├── index.tsx                   # Main landing page
│   │   ├── pricing.tsx                 # Pricing page
│   │   ├── features.tsx                # Features detail page
│   │   └── contact.tsx                 # Contact page
│   ├── components/
│   │   ├── landing/
│   │   │   ├── Hero.tsx
│   │   │   ├── Features.tsx
│   │   │   ├── HowItWorks.tsx
│   │   │   ├── Integrations.tsx
│   │   │   ├── Pricing.tsx
│   │   │   ├── Testimonials.tsx
│   │   │   ├── Security.tsx
│   │   │   ├── FAQ.tsx
│   │   │   ├── CTASection.tsx
│   │   │   └── Footer.tsx
│   │   └── shared/                      # Shared components
│   ├── hooks/                           # Custom React hooks
│   ├── lib/
│   │   ├── axios.ts                     # Axios instance
│   │   ├── validators/                  # Zod schemas
│   │   └── i18n/                        # i18n config
│   ├── stores/                          # Zustand stores
│   └── locales/                         # Translation files
│       ├── en/
│       └── ar/
└── styles/
    └── index.css                        # Tailwind v4 imports
```

---

## 3. Routing (React Router 7)

```tsx
// src/routes/__root.tsx
import { createRootRoute, Outlet } from '@tanstack/react-router';

export const Route = createRootRoute({
  component: () => (
    <QueryClientProvider client={queryClient}>
      <I18nextProvider i18n={i18n}>
        <Outlet />
      </I18nextProvider>
    </QueryClientProvider>
  ),
});
```

---

## 4. SEO (Static Meta Tags)

```tsx
// src/routes/index.tsx
import { Helmet } from 'react-helmet-async';

export function LandingPage() {
  return (
    <>
      <Helmet>
        <title>Payment Orchestration Platform — UAE</title>
        <meta name="description" content="AI-native payment orchestration for UAE merchants. Connect multiple acquirers, intelligent failover, automated reconciliation." />
        <meta property="og:title" content="Payment Orchestration Platform — UAE" />
        <meta property="og:description" content="Connect once, route everywhere." />
        <meta property="og:image" content="/og-image.png" />
        <link rel="canonical" href="https://platform.ae" />
      </Helmet>
      {/* Page content */}
    </>
  );
}
```

---

## 5. i18n Configuration

```tsx
// src/lib/i18n/index.ts
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from '../../locales/en/common.json';
import ar from '../../locales/ar/common.json';

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    ar: { translation: ar },
  },
  lng: localStorage.getItem('language') || 'en',
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
});
```

### RTL Support

```tsx
// src/App.tsx
function App() {
  const { i18n } = useTranslation();
  const isRtl = i18n.language === 'ar';

  return (
    <div dir={isRtl ? 'rtl' : 'ltr'} className={isRtl ? 'font-arabic' : ''}>
      <RouterProvider router={router} />
    </div>
  );
}
```

---

## 6. Performance Targets

| Metric | Target |
|--------|--------|
| Lighthouse Performance | > 95 |
| LCP | < 2.5s |
| FID | < 100ms |
| CLS | < 0.1 |
| Time to Interactive | < 3s |
| Bundle Size | < 200KB gzipped |
