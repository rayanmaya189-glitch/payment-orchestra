# 10 — Footer

## 1. Footer Component

```tsx
// components/landing/Footer.tsx
export function Footer() {
  return (
    <footer className="bg-gray-900 text-gray-300">
      <div className="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
        <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-5">
          {/* Brand */}
          <div className="lg:col-span-2">
            <div className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-lg bg-blue-600" />
              <span className="text-xl font-bold text-white">Payment Orchestra</span>
            </div>
            <p className="mt-4 max-w-md text-sm text-gray-400">
              AI-native payment orchestration for UAE merchants. Connect once, route everywhere.
            </p>
            <div className="mt-6 flex gap-4">
              <a href="#" className="text-gray-400 hover:text-white transition-colors">
                <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24"><path d="M24 4.557c-.883.392-1.832.656-2.828.775 1.017-.609 1.798-1.574 2.165-2.724-.951.564-2.005.974-3.127 1.195-.896-.957-2.173-1.555-3.591-1.555-3.179 0-5.515 2.966-4.797 6.035-4.091-.205-7.719-2.165-10.148-5.144-1.29 2.213-.669 5.108 1.523 6.574-.806-.026-1.566-.247-2.229-.616-.054 2.281 1.581 4.415 3.949 4.89-.693.188-1.452.232-2.224.084.626 1.956 2.444 3.379 4.6 3.419-2.089 1.643-4.736 2.349-7.29 2.04 2.179 1.397 4.768 2.212 7.548 2.212 9.057 0 14.356-7.727 14.041-14.646.962-.695 1.797-1.562 2.457-2.549z"/></svg>
              </a>
              <a href="#" className="text-gray-400 hover:text-white transition-colors">
                <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24"><path d="M19 0h-14c-2.761 0-5 2.239-5 5v14c0 2.761 2.239 5 5 5h14c2.762 0 5-2.239 5-5v-14c0-2.761-2.238-5-5-5zm-11 19h-3v-11h3v11zm-1.5-12.268c-.966 0-1.75-.79-1.75-1.764s.784-1.764 1.75-1.764 1.75.79 1.75 1.764-.783 1.764-1.75 1.764zm13.5 12.268h-3v-5.604c0-3.368-4-3.113-4 0v5.604h-3v-11h3v1.765c1.396-2.586 7-2.777 7 2.476v6.759z"/></svg>
              </a>
            </div>
          </div>

          {/* Product */}
          <div>
            <h3 className="text-sm font-semibold text-white">Product</h3>
            <ul className="mt-4 space-y-3">
              <li><a href="#features" className="text-sm text-gray-400 hover:text-white transition-colors">Features</a></li>
              <li><a href="#pricing" className="text-sm text-gray-400 hover:text-white transition-colors">Pricing</a></li>
              <li><a href="/docs" className="text-sm text-gray-400 hover:text-white transition-colors">Documentation</a></li>
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">API Reference</a></li>
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">Status</a></li>
            </ul>
          </div>

          {/* Company */}
          <div>
            <h3 className="text-sm font-semibold text-white">Company</h3>
            <ul className="mt-4 space-y-3">
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">About</a></li>
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">Blog</a></li>
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">Careers</a></li>
              <li><a href="#contact" className="text-sm text-gray-400 hover:text-white transition-colors">Contact</a></li>
            </ul>
          </div>

          {/* Legal */}
          <div>
            <h3 className="text-sm font-semibold text-white">Legal</h3>
            <ul className="mt-4 space-y-3">
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">Privacy Policy</a></li>
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">Terms of Service</a></li>
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">Cookie Policy</a></li>
              <li><a href="#" className="text-sm text-gray-400 hover:text-white transition-colors">Security</a></li>
            </ul>
          </div>
        </div>

        {/* Bottom bar */}
        <div className="mt-12 border-t border-gray-800 pt-8 flex flex-col md:flex-row justify-between items-center gap-4">
          <p className="text-sm text-gray-400">
            © 2024 Payment Orchestra. All rights reserved.
          </p>
          <div className="flex items-center gap-6">
            <img src="/logos/pci-dss.svg" alt="PCI-DSS" className="h-6 opacity-50" />
            <img src="/logos/iso-27001.svg" alt="ISO 27001" className="h-6 opacity-50" />
            <img src="/logos/uae-cb.svg" alt="UAE Central Bank" className="h-6 opacity-50" />
          </div>
        </div>
      </div>
    </footer>
  );
}
```
