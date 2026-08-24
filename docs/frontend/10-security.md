# 10 — Frontend Security

## 1. Authentication Flow

```
1. User enters email + password
2. POST /v1/auth/login → { access_token, refresh_token, expires_at }
3. access_token stored in encrypted Session Storage
4. refresh_token stored in HttpOnly cookie (SameSite=Strict)
5. On every Axios request: Authorization: Bearer <access_token>
6. When access_token expires: POST /v1/auth/refresh → new tokens
7. When refresh_token expires: redirect to /login
```

### Token Management

```typescript
// src/stores/auth.store.ts
import { create } from 'zustand';
import { sessionStore } from '../lib/session-storage';
import api from '../lib/axios';

interface AuthState {
  isAuthenticated: boolean;
  user: { id: string; email: string; role: string } | null;
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  isAuthenticated: !!sessionStore.get('auth_token'),
  user: sessionStore.get('user'),

  login: async (email, password) => {
    const { data } = await api.post('/auth/login', { email, password });

    // Store access token in encrypted Session Storage
    sessionStore.set('auth_token', {
      access_token: data.access_token,
      expires_at: data.expires_at,
    });

    // Store user info
    sessionStore.set('user', data.user);

    // Fetch CSRF token
    const csrfRes = await api.get('/auth/csrf-token');
    sessionStore.set('csrf_token', csrfRes.data.token);

    set({ isAuthenticated: true, user: data.user });
  },

  logout: () => {
    sessionStore.clear();
    set({ isAuthenticated: false, user: null });
    window.location.href = '/login';
  },
}));
```

### Axios Interceptor (Auto-Refresh)

```typescript
// src/lib/axios.ts
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config;

    if (error.response?.status === 401 && !originalRequest._retry) {
      originalRequest._retry = true;

      try {
        const { data } = await axios.post(
          `${import.meta.env.VITE_API_URL}/auth/refresh`,
          {},
          { withCredentials: true } // Send refresh token cookie
        );

        // Update stored token
        sessionStore.set('auth_token', {
          access_token: data.access_token,
          expires_at: data.expires_at,
        });

        // Retry original request
        originalRequest.headers.Authorization = `Bearer ${data.access_token}`;
        return api(originalRequest);
      } catch {
        // Refresh failed — force logout
        sessionStore.clear();
        window.location.href = '/login';
      }
    }

    return Promise.reject(error);
  }
);
```

---

## 2. CSRF Protection

```typescript
// All mutating Axios requests include CSRF token via interceptor
api.interceptors.request.use((config) => {
  if (['post', 'put', 'patch', 'delete'].includes(config.method || '')) {
    const csrf = sessionStore.get<string>('csrf_token');
    if (csrf) {
      config.headers['X-CSRF-Token'] = csrf;
    }
  }
  return config;
});

// CSRF token refresh on page load
useEffect(() => {
  api.get('/auth/csrf-token').then(({ data }) => {
    sessionStore.set('csrf_token', data.token);
  });
}, []);
```

---

## 3. XSS Prevention

### Content Security Policy (via Vite config or nginx)

```
Content-Security-Policy:
  default-src 'self';
  script-src 'self';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data:;
  connect-src 'self' https://api.platform.ae https://ai.platform.ae;
  frame-ancestors 'none';
  base-uri 'self';
  form-action 'self';
```

### React Auto-Escaping

React 19 automatically escapes JSX content. Avoid `dangerouslySetInnerHTML`.

```tsx
// SAFE — React auto-escapes
<div>{userInput}</div>

// DANGEROUS — only with sanitized HTML
<div dangerouslySetInnerHTML={{ __html: sanitize(userInput) }} />
```

---

## 4. Sensitive Data Handling

```typescript
// NEVER store in localStorage
localStorage.setItem('token', token); // ❌ WRONG

// NEVER store in sessionStorage without encryption
sessionStorage.setItem('card_data', cardData); // ❌ WRONG

// ✅ CORRECT — encrypted Session Storage
sessionStore.set('auth_token', { access_token: token }); // ✅ Encrypted

// NEVER put in URL params
router.push(`/payment?token=${sensitiveToken}`); // ❌ WRONG

// NEVER log to console
console.log('API Key:', apiKey); // ❌ WRONG
```

---

## 5. Input Validation (Zod)

```typescript
// src/lib/validators/payment.ts
import { z } from 'zod';

export const createPaymentSchema = z.object({
  amount: z.number().positive().max(1_000_000_00), // max 1M AED in minor units
  currency: z.enum(['AED', 'USD', 'EUR', 'SAR']),
  description: z.string().max(255).optional(),
  purpose: z.enum(['payment', 'card_verification']),
});

export const refundSchema = z.object({
  amount: z.number().positive(),
  currency: z.enum(['AED', 'USD', 'EUR', 'SAR']),
});

// Validate before API call
const validated = createPaymentSchema.parse(formData);
await api.post('/v1/payment-intents', validated);
```

---

## 6. Rate Limit Handling

```typescript
// Axios interceptor handles 429
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 429) {
      const retryAfter = parseInt(error.response.headers['retry-after'] || '30');

      // Show rate limit notification
      toast({
        title: t('errors.rate_limited'),
        description: t('errors.retry_after', { seconds: retryAfter }),
        variant: 'warning',
      });

      // Disable form submission
      setIsRateLimited(true);
      setTimeout(() => setIsRateLimited(false), retryAfter * 1000);
    }
    return Promise.reject(error);
  }
);
```

---

## 7. Error Boundary

```tsx
// src/components/shared/ErrorBoundary.tsx
import React from 'react';
import { Button } from '../ui/button';

interface Props {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('Frontend error:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback || (
        <div className="flex h-96 items-center justify-center">
          <div className="text-center">
            <h2 className="text-lg font-semibold">Something went wrong</h2>
            <p className="text-muted-foreground">
              An unexpected error occurred. Please try again.
            </p>
            <Button onClick={() => window.location.reload()} className="mt-4">
              Reload Page
            </Button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
```

---

## 8. Security Headers

| Header | Value | Purpose |
|--------|-------|---------|
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains; preload` | Force HTTPS |
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-Frame-Options` | `DENY` | Prevent clickjacking |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Control referrer leakage |
| `Permissions-Policy` | `camera=(), microphone=(), geolocation=(), payment=()` | Disable unnecessary APIs |
| `Content-Security-Policy` | (see §3) | Prevent XSS |

---

## 9. Session Storage vs localStorage

| Property | Session Storage | localStorage |
|----------|----------------|--------------|
| Scope | Tab only | All tabs |
| Lifetime | Until tab closes | Until explicitly cleared |
| Encrypted | ✅ (our implementation) | ❌ (plain text) |
| XSS risk | Lower (tab-scoped) | Higher (shared across tabs) |
| Multi-tab | ❌ (isolated per tab) | ✅ (shared) |

**Decision**: Session Storage chosen because:
1. Financial data should not persist across tabs
2. Tab-scoped reduces XSS blast radius
3. Encrypted implementation adds defense-in-depth
4. Cleared automatically when user closes browser
