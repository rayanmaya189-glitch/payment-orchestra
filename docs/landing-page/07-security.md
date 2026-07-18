# 07 — Security Section

## 1. Security Trust Signals

```tsx
// components/landing/Security.tsx
'use client';

import { motion } from 'framer-motion';

const securityFeatures = [
  {
    icon: 'shield',
    title: 'PCI-DSS Aligned',
    description: 'Architecture designed for SAQ-A scope. No raw PAN handling. Tokenization via acquirer SDKs.',
  },
  {
    icon: 'lock',
    title: 'Bank-Grade Encryption',
    description: 'AES-256 envelope encryption for all sensitive data. HSM-backed key management. TLS 1.3 everywhere.',
  },
  {
    icon: 'check-circle',
    title: 'Immutable Audit Trail',
    description: 'Event-sourced audit log. Every money-movement event recorded with actor, timestamp, and reason. Hash-chained for tamper evidence.',
  },
  {
    icon: 'users',
    title: 'Role-Based Access',
    description: 'ABAC policies with Maker/Checker dual-control. WebAuthn MFA for privileged operations.',
  },
  {
    icon: 'database',
    title: 'UAE Data Residency',
    description: 'Full stack deployed within UAE infrastructure. Data never leaves the country.',
  },
  {
    icon: 'eye',
    title: 'No Custody',
    description: 'Your funds flow directly between acquirers and your bank. We never hold or touch them.',
  },
];

const complianceBadges = [
  { name: 'PCI-DSS', logo: '/logos/pci-dss.svg' },
  { name: 'ISO 27001', logo: '/logos/iso-27001.svg' },
  { name: 'SOC 2', logo: '/logos/soc2.svg' },
  { name: 'UAE Central Bank', logo: '/logos/uae-cb.svg' },
];

export function Security() {
  return (
    <section id="security" className="bg-gray-50 py-24 dark:bg-gray-800">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
            Security you can trust
          </h2>
          <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
            Built for the most demanding financial requirements
          </p>
        </div>

        <div className="mt-16 grid gap-8 sm:grid-cols-2 lg:grid-cols-3">
          {securityFeatures.map((feature, index) => (
            <motion.div
              key={feature.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: index * 0.1 }}
              viewport={{ once: true }}
              className="rounded-xl bg-white p-8 shadow-sm dark:bg-gray-900"
            >
              <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-red-100 dark:bg-red-900">
                <Icon name={feature.icon} className="h-6 w-6 text-red-600 dark:text-red-400" />
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

        {/* Compliance badges */}
        <div className="mt-16 text-center">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
            Compliance & Certifications
          </h3>
          <div className="mt-6 flex flex-wrap items-center justify-center gap-8">
            {complianceBadges.map((badge) => (
              <img
                key={badge.name}
                src={badge.logo}
                alt={badge.name}
                className="h-12 w-auto object-contain opacity-70 hover:opacity-100 transition-opacity"
              />
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
```
