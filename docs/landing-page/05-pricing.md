# 05 — Pricing Section

## 1. Pricing Tiers

```tsx
// components/landing/Pricing.tsx
'use client';

import { motion } from 'framer-motion';

const tiers = [
  {
    name: 'Starter',
    price: '0',
    period: 'forever',
    description: 'Perfect for small merchants getting started',
    features: [
      '1 acquirer connection',
      'Basic routing (priority)',
      'Transaction dashboard',
      'Email support',
      'Up to 1,000 transactions/month',
    ],
    cta: 'Start Free',
    popular: false,
  },
  {
    name: 'Growth',
    price: '499',
    period: '/month',
    description: 'For growing businesses with multiple acquirers',
    features: [
      'Up to 3 acquirer connections',
      'Smart routing & failover',
      'Auto reconciliation',
      'AI Assistant (basic)',
      'Up to 10,000 transactions/month',
      'Priority email support',
      'Webhook integrations',
    ],
    cta: 'Start Trial',
    popular: true,
  },
  {
    name: 'Enterprise',
    price: 'Custom',
    period: '',
    description: 'For large merchants with complex needs',
    features: [
      'Unlimited acquirer connections',
      'Advanced routing & optimization',
      'Full AI Assistant suite',
      'Dedicated account manager',
      'Custom integrations',
      'SLA guarantee (99.99%)',
      'On-premise deployment option',
    ],
    cta: 'Contact Sales',
    popular: false,
  },
];

export function Pricing() {
  return (
    <section id="pricing" className="bg-gray-50 py-24 dark:bg-gray-800">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
            Simple, transparent pricing
          </h2>
          <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
            Start free, scale as you grow
          </p>
        </div>

        <div className="mt-16 grid gap-8 lg:grid-cols-3">
          {tiers.map((tier, index) => (
            <motion.div
              key={tier.name}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: index * 0.1 }}
              viewport={{ once: true }}
              className={`relative rounded-2xl border-2 p-8 ${
                tier.popular
                  ? 'border-blue-500 shadow-xl'
                  : 'border-gray-200 dark:border-gray-700'
              }`}
            >
              {tier.popular && (
                <div className="absolute -top-4 left-1/2 -translate-x-1/2">
                  <span className="rounded-full bg-blue-600 px-4 py-1 text-sm font-semibold text-white">
                    Most Popular
                  </span>
                </div>
              )}

              <h3 className="text-xl font-bold text-gray-900 dark:text-white">{tier.name}</h3>
              <p className="mt-2 text-sm text-gray-600 dark:text-gray-300">{tier.description}</p>

              <div className="mt-6">
                <span className="text-4xl font-bold text-gray-900 dark:text-white">
                  {tier.price === '0' ? 'Free' : `AED ${tier.price}`}
                </span>
                <span className="text-gray-600 dark:text-gray-400">{tier.period}</span>
              </div>

              <ul className="mt-8 space-y-3">
                {tier.features.map((feature) => (
                  <li key={feature} className="flex items-start gap-3">
                    <svg className="mt-0.5 h-5 w-5 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                    </svg>
                    <span className="text-gray-600 dark:text-gray-300">{feature}</span>
                  </li>
                ))}
              </ul>

              <button
                className={`mt-8 w-full rounded-lg py-3 font-semibold transition-colors ${
                  tier.popular
                    ? 'bg-blue-600 text-white hover:bg-blue-700'
                    : 'bg-gray-100 text-gray-900 hover:bg-gray-200 dark:bg-gray-700 dark:text-white dark:hover:bg-gray-600'
                }`}
              >
                {tier.cta}
              </button>
            </motion.div>
          ))}
        </div>

        {/* Enterprise CTA */}
        <div className="mt-12 text-center">
          <p className="text-gray-600 dark:text-gray-300">
            Need a custom solution? <a href="#contact" className="text-blue-600 underline">Talk to our sales team</a>
          </p>
        </div>
      </div>
    </section>
  );
}
```
