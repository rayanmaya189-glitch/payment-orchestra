# 01 — Hero Section

## 1. Hero Component

```tsx
// components/landing/Hero.tsx
'use client';

import { motion } from 'framer-motion';

export function Hero() {
  return (
    <section className="relative overflow-hidden bg-gradient-to-br from-blue-50 to-indigo-100 dark:from-gray-900 dark:to-gray-800">
      {/* Background decoration */}
      <div className="absolute inset-0 overflow-hidden">
        <div className="absolute -top-40 -right-40 h-80 w-80 rounded-full bg-blue-200/30 blur-3xl" />
        <div className="absolute -bottom-40 -left-40 h-80 w-80 rounded-full bg-indigo-200/30 blur-3xl" />
      </div>

      <div className="relative mx-auto max-w-7xl px-4 py-24 sm:px-6 lg:px-8">
        <div className="grid gap-12 lg:grid-cols-2 lg:items-center">
          {/* Left: Content */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
          >
            <div className="mb-6 inline-flex items-center rounded-full bg-blue-100 px-4 py-2 text-sm font-medium text-blue-700 dark:bg-blue-900 dark:text-blue-300">
              <span className="mr-2">🇦🇪</span>
              Built for UAE Merchants
            </div>

            <h1 className="text-4xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-5xl lg:text-6xl">
              Connect Once,{' '}
              <span className="bg-gradient-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent">
                Route Everywhere
              </span>
            </h1>

            <p className="mt-6 text-lg text-gray-600 dark:text-gray-300">
              The AI-native payment orchestration platform that lets you connect multiple acquirers,
              define smart routing rules, and reconcile settlements automatically — without ever
              holding your funds.
            </p>

            <div className="mt-8 flex flex-wrap gap-4">
              <a
                href="#pricing"
                className="inline-flex items-center rounded-lg bg-blue-600 px-6 py-3 text-base font-semibold text-white shadow-lg hover:bg-blue-700 transition-colors"
              >
                Start Free Trial
                <svg className="ml-2 h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7l5 5m0 0l-5 5m5-5H6" />
                </svg>
              </a>
              <a
                href="#how-it-works"
                className="inline-flex items-center rounded-lg border border-gray-300 bg-white px-6 py-3 text-base font-semibold text-gray-700 shadow-sm hover:bg-gray-50 transition-colors dark:border-gray-600 dark:bg-gray-800 dark:text-gray-300"
              >
                See How It Works
              </a>
            </div>

            {/* Trust badges */}
            <div className="mt-10 flex flex-wrap items-center gap-6">
              <TrustBadge icon="shield" text="PCI-DSS Compliant" />
              <TrustBadge icon="lock" text="Bank-Grade Security" />
              <TrustBadge icon="check" text="UAE Central Bank Aligned" />
            </div>
          </motion.div>

          {/* Right: Dashboard preview */}
          <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.6, delay: 0.2 }}
            className="relative"
          >
            <div className="rounded-2xl bg-white p-4 shadow-2xl dark:bg-gray-800">
              <DashboardPreview />
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  );
}
```

---

## 2. Trust Badges

```tsx
function TrustBadge({ icon, text }: { icon: string; text: string }) {
  return (
    <div className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
      <div className="flex h-8 w-8 items-center justify-center rounded-full bg-green-100 dark:bg-green-900">
        <Icon name={icon} className="h-4 w-4 text-green-600 dark:text-green-400" />
      </div>
      <span>{text}</span>
    </div>
  );
}
```

---

## 3. Dashboard Preview

```tsx
function DashboardPreview() {
  return (
    <div className="space-y-4">
      {/* Stats row */}
      <div className="grid grid-cols-3 gap-3">
        <PreviewCard title="Today" value="1,234" trend="+12%" />
        <PreviewCard title="Auth Rate" value="98.5%" trend="+2.1%" />
        <PreviewCard title="Revenue" value="AED 45K" trend="+8%" />
      </div>
      
      {/* Chart placeholder */}
      <div className="h-32 rounded-lg bg-gradient-to-r from-blue-100 to-indigo-100 dark:from-blue-900 dark:to-indigo-900" />
      
      {/* Recent transactions */}
      <div className="space-y-2">
        {[1, 2, 3].map((i) => (
          <div key={i} className="flex items-center justify-between rounded-lg bg-gray-50 p-3 dark:bg-gray-700">
            <div className="flex items-center gap-3">
              <div className="h-8 w-8 rounded-full bg-blue-100 dark:bg-blue-900" />
              <div>
                <div className="h-3 w-24 rounded bg-gray-200 dark:bg-gray-600" />
                <div className="mt-1 h-2 w-16 rounded bg-gray-200 dark:bg-gray-600" />
              </div>
            </div>
            <div className="h-3 w-16 rounded bg-gray-200 dark:bg-gray-600" />
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## 4. Social Proof Bar

```tsx
function SocialProofBar() {
  return (
    <section className="border-y border-gray-200 bg-white py-12 dark:border-gray-800 dark:bg-gray-900">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="grid grid-cols-2 gap-8 md:grid-cols-4">
          <StatItem value="500+" label="Merchants" />
          <StatItem value="AED 2B+" label="Processed Monthly" />
          <StatItem value="99.99%" label="Uptime" />
          <StatItem value="3" label="UAE Acquirers" />
        </div>
      </div>
    </section>
  );
}

function StatItem({ value, label }: { value: string; label: string }) {
  return (
    <div className="text-center">
      <div className="text-3xl font-bold text-blue-600">{value}</div>
      <div className="mt-1 text-sm text-gray-600 dark:text-gray-400">{label}</div>
    </div>
  );
}
```
