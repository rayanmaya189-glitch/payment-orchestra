# 08 — FAQ Section

## 1. FAQ Component

```tsx
// components/landing/FAQ.tsx
'use client';

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

const faqs = [
  {
    question: 'What payment acquirers do you support?',
    answer: 'We support Network International, Checkout.com, Telr, PayTabs, Magnati, and more. We add new acquirers regularly. Contact us if you need a specific provider.',
  },
  {
    question: 'How does intelligent failover work?',
    answer: 'When a payment is declined or times out on your primary acquirer, we automatically retry on your secondary acquirer — without any action required. This typically recovers 15-25% of otherwise lost transactions.',
  },
  {
    question: 'Do you hold my funds?',
    answer: 'No. We never hold merchant or customer funds. All settlement occurs directly between your acquirers/banks and your bank account. We only orchestrate the payment flow.',
  },
  {
    question: 'Is the platform PCI-DSS compliant?',
    answer: 'Our architecture is designed for PCI-DSS SAQ-A scope. We never handle raw card numbers — tokenization happens via the acquirer\'s own SDK. We undergo annual PCI-DSS assessments.',
  },
  {
    question: 'How does the AI Assistant work?',
    answer: 'Our AI Assistant uses retrieval-augmented generation (RAG) over your own transaction and settlement data. It can answer questions about authorization rates, settlement status, decline reasons, and reconciliation — always citing specific sources from your data.',
  },
  {
    question: 'What about data residency?',
    answer: 'The entire platform is deployed within UAE infrastructure. Your data never leaves the country. We comply with UAE data protection requirements.',
  },
  {
    question: 'How long does integration take?',
    answer: 'Most merchants connect their first acquirer in under an hour. Full integration with routing, reconciliation, and webhooks typically takes 1-2 days.',
  },
  {
    question: 'What if I need support?',
    answer: 'Growth plan includes priority email support. Enterprise plan includes a dedicated account manager and SLA guarantee.',
  },
];

export function FAQ() {
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  return (
    <section id="faq" className="bg-white py-24 dark:bg-gray-900">
      <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
            Frequently asked questions
          </h2>
        </div>

        <div className="mt-16 space-y-4">
          {faqs.map((faq, index) => (
            <div
              key={index}
              className="rounded-xl border border-gray-200 dark:border-gray-700"
            >
              <button
                onClick={() => setOpenIndex(openIndex === index ? null : index)}
                className="flex w-full items-center justify-between p-6 text-left"
              >
                <span className="text-lg font-semibold text-gray-900 dark:text-white">
                  {faq.question}
                </span>
                <svg
                  className={`h-5 w-5 text-gray-500 transition-transform ${
                    openIndex === index ? 'rotate-180' : ''
                  }`}
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </button>
              
              <AnimatePresence>
                {openIndex === index && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: 'auto', opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.3 }}
                    className="overflow-hidden"
                  >
                    <div className="px-6 pb-6 text-gray-600 dark:text-gray-300">
                      {faq.answer}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
```
