# 03 — How It Works

## 1. Steps Section

```tsx
// components/landing/HowItWorks.tsx
'use client';

import { motion } from 'framer-motion';

const steps = [
  {
    step: '01',
    title: 'Connect Your Acquirers',
    description: 'Add your existing acquirer/PSP credentials in minutes. We support Network International, Checkout.com, Telr, and more.',
    icon: 'plug',
  },
  {
    step: '02',
    title: 'Configure Routing Rules',
    description: 'Define how transactions route across your acquirers — by card scheme, currency, amount, or cost. No code changes needed.',
    icon: 'settings',
  },
  {
    step: '03',
    title: 'Process Payments',
    description: 'Your customers checkout normally. We handle routing, failover, and 3D Secure automatically.',
    icon: 'credit-card',
  },
  {
    step: '04',
    title: 'Reconcile Automatically',
    description: 'Settlements from all acquirers are matched against your transactions automatically. Review exceptions in one place.',
    icon: 'check-circle',
  },
];

export function HowItWorks() {
  return (
    <section id="how-it-works" className="bg-gray-50 py-24 dark:bg-gray-800">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
            How it works
          </h2>
          <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
            Get started in four simple steps
          </p>
        </div>

        <div className="mt-16 grid gap-8 md:grid-cols-2 lg:grid-cols-4">
          {steps.map((step, index) => (
            <motion.div
              key={step.step}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: index * 0.15 }}
              viewport={{ once: true }}
              className="relative"
            >
              {/* Connector line */}
              {index < steps.length - 1 && (
                <div className="absolute left-1/2 top-12 hidden h-0.5 w-full bg-blue-200 lg:block dark:bg-blue-800" />
              )}
              
              <div className="relative text-center">
                <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-blue-600 text-2xl font-bold text-white">
                  {step.step}
                </div>
                <h3 className="mt-6 text-xl font-semibold text-gray-900 dark:text-white">
                  {step.title}
                </h3>
                <p className="mt-2 text-gray-600 dark:text-gray-300">
                  {step.description}
                </p>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
```

---

## 2. Interactive Demo Preview

```tsx
function InteractiveDemo() {
  const [activeStep, setActiveStep] = useState(0);

  return (
    <section className="bg-white py-24 dark:bg-gray-900">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="grid gap-12 lg:grid-cols-2">
          {/* Step selector */}
          <div className="space-y-4">
            {steps.map((step, index) => (
              <button
                key={index}
                onClick={() => setActiveStep(index)}
                className={`w-full rounded-xl p-6 text-left transition-all ${
                  activeStep === index
                    ? 'bg-blue-50 border-2 border-blue-500 dark:bg-blue-900/30'
                    : 'bg-gray-50 hover:bg-gray-100 dark:bg-gray-800 dark:hover:bg-gray-700'
                }`}
              >
                <div className="flex items-center gap-4">
                  <span className="flex h-10 w-10 items-center justify-center rounded-full bg-blue-600 text-sm font-bold text-white">
                    {step.step}
                  </span>
                  <div>
                    <h3 className="font-semibold text-gray-900 dark:text-white">{step.title}</h3>
                    <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">{step.description}</p>
                  </div>
                </div>
              </button>
            ))}
          </div>

          {/* Demo preview */}
          <motion.div
            key={activeStep}
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="rounded-2xl bg-gray-100 p-8 dark:bg-gray-800"
          >
            <img
              src={`/images/demo/step-${activeStep + 1}.png`}
              alt={steps[activeStep].title}
              className="rounded-xl shadow-lg"
            />
          </motion.div>
        </div>
      </div>
    </section>
  );
}
```
