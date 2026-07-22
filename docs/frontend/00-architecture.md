# 00 — Frontend Architecture

## 1. Tech Stack

```json
{
  "framework": "React 19 + React Router DOM 7",
  "build": "Vite 8",
  "language": "TypeScript 5.x (strict mode)",
  "styling": "Tailwind CSS 4",
  "state": {
    "server": "TanStack React Query v5",
    "client": "Zustand v5"
  },
  "http": "Axios v1.x",
  "storage": "Session Storage (encrypted)",
  "forms": "Zod validation",
  "charts": "Recharts",
  "tables": "TanStack Table v9",
  "i18n": "react-i18next (Arabic + English)",
  "icons": "Lucide React"
}
```

### Why This Stack

| Choice | Rationale |
|--------|-----------|
| **React 19** | Server components not needed (SPA dashboard); React 19 perf improvements, use() hook, actions |
| **Vite 8** | Fast HMR, native ESM, optimized builds; no SSR needed for admin dashboard |
| **React Router 7** | File-based routing, loaders/actions, nested layouts |
| **Tailwind 4** | CSS-first config, no build step for config, native v4 features |
| **Axios** | Request/response interceptors for auth, retry, error handling; better than fetch for complex scenarios |
| **Session Storage** | Encrypted session tokens (not localStorage/cookies); survives page refresh, cleared on tab close |
| **Zustand** | Lightweight client state (UI state, sidebar, modals); no boilerplate |
| **Zod** | Runtime type validation for API responses and form inputs |

---

## 2. Project Structure

```
src/
├── main.tsx                          # Vite entry point
├── App.tsx                           # Root component with providers
├── routes/                           # React Router routes
│   ├── __root.tsx                    # Root layout (auth check, theme)
│   ├── login.tsx                     # Login page
│   ├── register.tsx                  # Registration page
│   ├── _dashboard.tsx                # Authenticated layout (sidebar + topbar)
│   ├── _dashboard.index.tsx          # Main dashboard
│   ├── _dashboard.payments.tsx       # Transaction list
│   ├── _dashboard.payments.$id.tsx   # Transaction detail
│   ├── _dashboard.reconciliation.tsx
│   ├── _dashboard.reconciliation.exceptions.tsx
│   ├── _dashboard.invoices.tsx
│   ├── _dashboard.invoices.$id.tsx
│   ├── _dashboard.subscriptions.tsx
│   ├── _dashboard.connectors.tsx
│   ├── _dashboard.connectors.$id.tsx
│   ├── _dashboard.settings.tsx       # Settings layout
│   ├── _dashboard.settings.api-keys.tsx
│   ├── _dashboard.settings.users.tsx
│   ├── _dashboard.settings.routing.tsx
│   ├── _dashboard.settings.compliance.tsx
│   ├── _dashboard.assistant.tsx      # AI Assistant
│   └── pay.$token.tsx                # Hosted checkout (PCI-DSS isolated)
├── components/
│   ├── ui/                           # Base UI components (shadcn/ui for Vite)
│   ├── dashboard/                    # Dashboard-specific
│   ├── payments/                     # Payment components
│   ├── reconciliation/               # Reconciliation components
│   ├── invoices/                     # Invoice components
│   ├── settings/                     # Settings components
│   ├── assistant/                    # AI Assistant components
│   └── shared/                       # Shared (tables, forms, modals)
├── hooks/                            # Custom React hooks
├── lib/
│   ├── axios.ts                      # Axios instance with interceptors
│   ├── api/                          # Generated API types + client
│   ├── types/                        # TypeScript type definitions
│   ├── utils/                        # Utility functions
│   └── validators/                   # Zod schemas
├── stores/                           # Zustand stores
│   ├── auth.store.ts                 # Auth state (token, user)
│   ├── ui.store.ts                   # UI state (sidebar, modals)
│   └── dashboard.store.ts            # Dashboard filters, date range
├── i18n/                             # i18n configuration
│   ├── index.ts
│   ├── en.json
│   └── ar.json
├── locales/                          # Translation files
│   ├── en/
│   └── ar/
└── styles/
    ├── index.css                     # Tailwind v4 imports
    └── globals.css                   # Custom CSS variables
```

---

## 3. Routing (React Router 7)

```tsx
// src/routes/__root.tsx
import { createRootRoute, Outlet } from '@tanstack/react-router';

export const Route = createRootRoute({
  component: () => (
    <AuthProvider>
      <QueryClientProvider client={queryClient}>
        <I18nextProvider i18n={i18n}>
          <Outlet />
        </I18nextProvider>
      </QueryClientProvider>
    </AuthProvider>
  ),
});
```

### Route Table

| Route | File | Auth | Description |
|-------|------|------|-------------|
| `/` | Redirect | — | → `/dashboard` |
| `/login` | `login.tsx` | No | Login page |
| `/register` | `register.tsx` | No | Registration page |
| `/dashboard` | `_dashboard.index.tsx` | Required | Main dashboard |
| `/payments` | `_dashboard.payments.tsx` | Required | Transaction list |
| `/payments/:id` | `_dashboard.payments.$id.tsx` | Required | Transaction detail |
| `/reconciliation` | `_dashboard.reconciliation.tsx` | Required | Reconciliation dashboard |
| `/reconciliation/exceptions` | `_dashboard.reconciliation.exceptions.tsx` | Required | Exception queue |
| `/invoices` | `_dashboard.invoices.tsx` | Required | Invoice list |
| `/invoices/:id` | `_dashboard.invoices.$id.tsx` | Required | Invoice detail |
| `/subscriptions` | `_dashboard.subscriptions.tsx` | Required | Subscription list |
| `/connectors` | `_dashboard.connectors.tsx` | Required | Acquirer connections |
| `/connectors/:id` | `_dashboard.connectors.$id.tsx` | Required | Connector detail |
| `/settings/api-keys` | `_dashboard.settings.api-keys.tsx` | Required | API key management |
| `/settings/users` | `_dashboard.settings.users.tsx` | Required | User management |
| `/settings/routing` | `_dashboard.settings.routing.tsx` | Required | Routing policy |
| `/settings/compliance` | `_dashboard.settings.compliance.tsx` | Required | KYB, AML, audit |
| `/assistant` | `_dashboard.assistant.tsx` | Required | AI Assistant |
| `/pay/:token` | `pay.$token.tsx` | No | Hosted checkout |

---

## 4. Session Storage (Encrypted)

```typescript
// src/lib/session-storage.ts
import CryptoJS from 'crypto-js';

const ENCRYPTION_KEY = import.meta.env.VITE_SESSION_KEY;

export const sessionStore = {
  set(key: string, value: unknown) {
    const encrypted = CryptoJS.AES.encrypt(JSON.stringify(value), ENCRYPTION_KEY).toString();
    sessionStorage.setItem(key, encrypted);
  },

  get<T>(key: string): T | null {
    const encrypted = sessionStorage.getItem(key);
    if (!encrypted) return null;
    const decrypted = CryptoJS.AES.decrypt(encrypted, ENCRYPTION_KEY).toString();
    return JSON.parse(decrypted) as T;
  },

  remove(key: string) {
    sessionStorage.removeItem(key);
  },

  clear() {
    sessionStorage.clear();
  },
};

// Usage
sessionStore.set('auth_token', { access_token: '...', expires_at: '...' });
const auth = sessionStore.get<{ access_token: string; expires_at: string }>('auth_token');
```

---

## 5. Axios Configuration

```typescript
// src/lib/axios.ts
import axios from 'axios';
import { sessionStore } from './session-storage';

const api = axios.create({
  baseURL: import.meta.env.VITE_API_URL,
  timeout: 30000,
  headers: {
    'Content-Type': 'application/protobuf',
  },
});

// Request interceptor — attach auth token
api.interceptors.request.use((config) => {
  const auth = sessionStore.get<{ access_token: string }>('auth_token');
  if (auth?.access_token) {
    config.headers.Authorization = `Bearer ${auth.access_token}`;
  }

  // Attach CSRF token for mutating requests
  if (['post', 'put', 'patch', 'delete'].includes(config.method || '')) {
    const csrf = sessionStore.get<string>('csrf_token');
    if (csrf) {
      config.headers['X-CSRF-Token'] = csrf;
    }
  }

  // Attach idempotency key for mutating requests
  if (['post', 'put', 'patch'].includes(config.method || '')) {
    config.headers['Idempotency-Key'] = crypto.randomUUID();
  }

  return config;
});

// Response interceptor — handle auth errors, rate limits
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config;

    if (error.response?.status === 401 && !originalRequest._retry) {
      originalRequest._retry = true;
      try {
        const refreshResponse = await axios.post(
          `${import.meta.env.VITE_API_URL}/auth/refresh`,
          {},
          { withCredentials: true }
        );
        const { access_token } = refreshResponse.data;
        sessionStore.set('auth_token', { access_token, expires_at: refreshResponse.data.expires_at });
        originalRequest.headers.Authorization = `Bearer ${access_token}`;
        return api(originalRequest);
      } catch {
        sessionStore.clear();
        window.location.href = '/login';
      }
    }

    if (error.response?.status === 429) {
      const retryAfter = error.response.headers['retry-after'] || 30;
      // Show rate limit toast
    }

    return Promise.reject(error);
  }
);

export default api;
```

---

## 6. Zustand Stores

```typescript
// src/stores/auth.store.ts
import { create } from 'zustand';
import { sessionStore } from '../lib/session-storage';

interface AuthState {
  isAuthenticated: boolean;
  user: { id: string; email: string; role: string } | null;
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
  refreshUser: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
  isAuthenticated: !!sessionStore.get('auth_token'),
  user: sessionStore.get('user'),

  login: async (email, password) => {
    const response = await api.post('/auth/login', { email, password });
    sessionStore.set('auth_token', {
      access_token: response.data.access_token,
      expires_at: response.data.expires_at,
    });
    sessionStore.set('user', response.data.user);
    set({ isAuthenticated: true, user: response.data.user });
  },

  logout: () => {
    sessionStore.clear();
    set({ isAuthenticated: false, user: null });
  },

  refreshUser: async () => {
    const response = await api.get('/auth/me');
    sessionStore.set('user', response.data);
    set({ user: response.data });
  },
}));

// src/stores/ui.store.ts
interface UiState {
  sidebarOpen: boolean;
  toggleSidebar: () => void;
  activeModal: string | null;
  openModal: (id: string) => void;
  closeModal: () => void;
}

export const useUiStore = create<UiState>((set) => ({
  sidebarOpen: true,
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  activeModal: null,
  openModal: (id) => set({ activeModal: id }),
  closeModal: () => set({ activeModal: null }),
}));
```

---

## 7. i18n Configuration

```typescript
// src/i18n/index.ts
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from '../locales/en/common.json';
import ar from '../locales/ar/common.json';

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    ar: { translation: ar },
  },
  lng: localStorage.getItem('language') || 'en',
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
});

export default i18n;
```

### Usage in Components

```tsx
import { useTranslation } from 'react-i18next';

function Dashboard() {
  const { t } = useTranslation();
  return <h1>{t('dashboard.title')}</h1>;
}
```

---

## 8. Environment Variables

```env
VITE_API_URL=https://api.platform.ae/v1
VITE_AI_URL=https://ai.platform.ae/v1
VITE_WS_URL=wss://api.platform.ae/ws
VITE_CHECKOUT_URL=https://pay.platform.ae
VITE_SESSION_KEY=<encryption-key-for-session-storage>
```
