# 02 — Features Section

## 1. Features Grid

```tsx
// components/landing/Features.tsx
'use client';

import { motion } from 'framer-motion';

const features = [
  {
    icon: 'route',
    title: 'Smart Routing',
    description: 'Define routing rules by card scheme, currency, and amount — no code changes needed.',
    color: 'blue',
  },
  {
    icon: 'shield',
    title: 'Intelligent Failover',
    description: 'Automatically retry on secondary acquirers when primary declines. Recover 15-25% of lost transactions.',
    color: 'green',
  },
  {
    icon: 'brain',
    title: 'AI Assistant',
    description: 'Natural-language assistant grounded in your data. Ask about settlements, anomalies, and reports.',
    color: 'purple',
  },
  {
    icon: 'check-circle',
    title: 'Auto Reconciliation',
    description: 'Automated settlement matching across all acquirers. Cut manual reconciliation by 70%.',
    color: 'emerald',
  },
  {
    icon: 'lock',
    title: 'Compliance-Grade Audit',
    description: 'Immutable audit trail from day one. UAE Central Bank and PCI-DSS aligned.',
    color: 'red',
  },
  {
    icon: 'globe',
    title: 'Pure Router - No Custody',
    description: 'Your customers pay your payment gateways directly; your gateways settle to your bank account. We route the instructions only - we never hold or touch your money.',
    color: 'orange',
  },
];

export function Features() {
  return (
    <section id="features" className="bg-white py-24 dark:bg-gray-900">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
            Everything you need to orchestrate payments
          </h2>
          <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
            Connect once, route everywhere, reconcile automatically.
          </p>
        </div>

        <div className="mt-16 grid gap-8 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((feature, index) => (
            <motion.div
              key={feature.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: index * 0.1 }}
              viewport={{ once: true }}
              className="rounded-xl border border-gray-200 p-8 shadow-sm hover:shadow-md transition-shadow dark:border-gray-700"
            >
              <div className={`mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-${feature.color}-100 dark:bg-${feature.color}-900`}>
                <Icon name={feature.icon} className={`h-6 w-6 text-${feature.color}-600 dark:text-${feature.color}-400`} />
              </div>
              <h3 className="text-xl font-semibold text-gray-900 dark:text-white">
                {feature.title}
              </h3>
              <p className="mt-2 text-gray-600 dark:text-gray-300">
                {feature.description}
              </p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
```

---

## 2. Feature Detail Cards

```tsx
function FeatureDetailCard({
  title,
  description,
  image,
  bullets,
  reversed = false,
}: {
  title: string;
  description: string;
  image: string;
  bullets: string[];
  reversed?: boolean;
}) {
  return (
    <div className={`grid gap-12 lg:grid-cols-2 lg:items-center ${reversed ? 'lg:direction-rtl' : ''}`}>
      <motion.div
        initial={{ opacity: 0, x: reversed ? 20 : -20 }}
        whileInView={{ opacity: 1, x: 0 }}
        viewport={{ once: true }}
      >
        <h3 className="text-2xl font-bold text-gray-900 dark:text-white">{title}</h3>
        <p className="mt-4 text-gray-600 dark:text-gray-300">{description}</p>
        <ul className="mt-6 space-y-3">
          {bullets.map((bullet, i) => (
            <li key={i} className="flex items-start gap-3">
              <svg className="mt-0.5 h-5 w-5 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
              <span className="text-gray-600 dark:text-gray-300">{bullet}</span>
            </li>
          ))}
        </ul>
      </motion.div>
      <motion.div
        initial={{ opacity: 0, x: reversed ? -20 : 20 }}
        whileInView={{ opacity: 1, x: 0 }}
        viewport={{ once: true }}
      >
        <img src={image} alt={title} className="rounded-xl shadow-lg" />
      </motion.div>
    </div>
  );
}

// Usage
<FeatureDetailCard
  title="Smart Routing & Failover"
  description="Define routing rules that match your business needs."
  image="/images/features/routing.png"
  bullets={[
    'Route by card scheme, currency, and amount',
    'Automatic failover on decline or timeout',
    'Gateway profile rotation for cost optimization',
    'Real-time success rate monitoring',
  ]}
/>
```
