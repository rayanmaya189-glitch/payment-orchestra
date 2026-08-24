# 06 — Testimonials Section

## 1. Testimonials Grid

```tsx
// components/landing/Testimonials.tsx
'use client';

import { motion } from 'framer-motion';

const testimonials = [
  {
    quote: "We reduced our reconciliation time from 3 days to 3 hours. The AI assistant is a game-changer for our finance team.",
    author: "Fatima Al-Rashid",
    role: "Finance Director",
    company: "Emirates E-Commerce Co.",
    avatar: "/images/testimonials/fatima.jpg",
  },
  {
    quote: "Integrating with multiple acquirers used to take months. With Payment Orchestra, we connected 3 providers in a single afternoon.",
    author: "Rashid Mohammed",
    role: "Head of Engineering",
    company: "Gulf Payments Ltd.",
    avatar: "/images/testimonials/rashid.jpg",
  },
  {
    quote: "The automatic failover recovered AED 2.3M in otherwise-lost transactions last quarter. The ROI was immediate.",
    author: "Omar Hassan",
    role: "CFO",
    company: "Dubai Retail Group",
    avatar: "/images/testimonials/omar.jpg",
  },
];

export function Testimonials() {
  return (
    <section id="testimonials" className="bg-white py-24 dark:bg-gray-900">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
            Trusted by UAE merchants
          </h2>
          <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
            See what our customers are saying
          </p>
        </div>

        <div className="mt-16 grid gap-8 md:grid-cols-3">
          {testimonials.map((testimonial, index) => (
            <motion.div
              key={testimonial.author}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: index * 0.1 }}
              viewport={{ once: true }}
              className="rounded-xl border border-gray-200 p-8 shadow-sm dark:border-gray-700"
            >
              {/* Stars */}
              <div className="flex gap-1 text-yellow-400">
                {[1, 2, 3, 4, 5].map((star) => (
                  <svg key={star} className="h-5 w-5 fill-current" viewBox="0 0 20 20">
                    <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
                  </svg>
                ))}
              </div>

              <blockquote className="mt-6 text-gray-600 dark:text-gray-300">
                &ldquo;{testimonial.quote}&rdquo;
              </blockquote>

              <div className="mt-6 flex items-center gap-4">
                <img
                  src={testimonial.avatar}
                  alt={testimonial.author}
                  className="h-12 w-12 rounded-full object-cover"
                />
                <div>
                  <div className="font-semibold text-gray-900 dark:text-white">
                    {testimonial.author}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">
                    {testimonial.role}, {testimonial.company}
                  </div>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
```
