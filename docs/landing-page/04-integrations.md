# 04 — Integrations Section

## 1. Acquirer Logos Grid

```tsx
// components/landing/Integrations.tsx
'use client';

import { motion } from 'framer-motion';

const integrations = [
  { name: 'Network International', logo: '/logos/network-international.svg', region: 'UAE' },
  { name: 'Checkout.com', logo: '/logos/checkout-com.svg', region: 'Global' },
  { name: 'Telr', logo: '/logos/telr.svg', region: 'MENA' },
  { name: 'PayTabs', logo: '/logos/paytabs.svg', region: 'MENA' },
  { name: 'Magnati', logo: '/logos/magnati.svg', region: 'UAE' },
  { name: 'Stripe', logo: '/logos/stripe.svg', region: 'Global' },
];

const cardSchemes = [
  { name: 'Visa', logo: '/logos/visa.svg' },
  { name: 'Mastercard', logo: '/logos/mastercard.svg' },
  { name: 'Amex', logo: '/logos/amex.svg' },
  { name: 'Mada', logo: '/logos/mada.svg' },
  { name: 'UnionPay', logo: '/logos/unionpay.svg' },
];

export function Integrations() {
  return (
    <section id="integrations" className="bg-white py-24 dark:bg-gray-900">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
            Connect with leading acquirers
          </h2>
          <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
            One integration, multiple payment providers
          </p>
        </div>

        {/* Acquirer logos */}
        <div className="mt-16 grid grid-cols-2 gap-8 md:grid-cols-3 lg:grid-cols-6">
          {integrations.map((integration, index) => (
            <motion.div
              key={integration.name}
              initial={{ opacity: 0, y: 10 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.3, delay: index * 0.1 }}
              viewport={{ once: true }}
              className="flex flex-col items-center justify-center rounded-xl border border-gray-200 p-6 hover:border-blue-300 hover:shadow-md transition-all dark:border-gray-700"
            >
              <img
                src={integration.logo}
                alt={integration.name}
                className="h-12 w-auto object-contain"
              />
              <span className="mt-3 text-sm text-gray-600 dark:text-gray-400">
                {integration.region}
              </span>
            </motion.div>
          ))}
        </div>

        {/* Card schemes */}
        <div className="mt-16 text-center">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
            Supported Card Schemes
          </h3>
          <div className="mt-6 flex flex-wrap items-center justify-center gap-8">
            {cardSchemes.map((scheme) => (
              <img
                key={scheme.name}
                src={scheme.logo}
                alt={scheme.name}
                className="h-10 w-auto object-contain opacity-70 hover:opacity-100 transition-opacity"
              />
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
```
