# 11 — API Client & Data Fetching

## 1. Axios Instance

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

// Request interceptor
api.interceptors.request.use((config) => {
  // Auth token
  const auth = sessionStore.get<{ access_token: string }>('auth_token');
  if (auth?.access_token) {
    config.headers.Authorization = `Bearer ${auth.access_token}`;
  }

  // CSRF token for mutating requests
  if (['post', 'put', 'patch', 'delete'].includes(config.method || '')) {
    const csrf = sessionStore.get<string>('csrf_token');
    if (csrf) config.headers['X-CSRF-Token'] = csrf;
  }

  // Idempotency key for mutating requests
  if (['post', 'put', 'patch'].includes(config.method || '')) {
    config.headers['Idempotency-Key'] = crypto.randomUUID();
  }

  return config;
});

// Response interceptor
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    // Handle 401 (token refresh)
    // Handle 429 (rate limit)
    // Handle 5xx (server error)
    return Promise.reject(error);
  }
);

export default api;
```

---

## 2. React Query Hooks

### Payment Hooks

```typescript
// src/hooks/use-payments.ts
import { useInfiniteQuery, useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import api from '../lib/axios';

export function usePayments(filters: PaymentFilters) {
  return useInfiniteQuery({
    queryKey: ['payments', filters],
    queryFn: ({ pageParam }) =>
      api.get('/v1/payment-intents', {
        params: { ...filters, cursor: pageParam, limit: 20 },
      }).then(r => r.data),
    getNextPageParam: (lastPage) => lastPage.pagination?.next_cursor,
    initialPageParam: undefined,
  });
}

export function usePayment(id: string) {
  return useQuery({
    queryKey: ['payment', id],
    queryFn: () => api.get(`/v1/payment-intents/${id}`).then(r => r.data),
    staleTime: 30_000,
  });
}

export function useCreatePaymentIntent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreatePaymentIntentData) =>
      api.post('/v1/payment-intents', data).then(r => r.data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['payments'] });
      queryClient.invalidateQueries({ queryKey: ['dashboard-stats'] });
    },
  });
}

export function useCapturePayment() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ paymentIntentId, amount }) =>
      api.post(`/v1/payment-intents/${paymentIntentId}/capture`, { amount }).then(r => r.data),

    onMutate: async ({ paymentIntentId }) => {
      await queryClient.cancelQueries({ queryKey: ['payment', paymentIntentId] });
      const previous = queryClient.getQueryData(['payment', paymentIntentId]);
      queryClient.setQueryData(['payment', paymentIntentId], (old: any) => ({
        ...old, status: 'Capturing',
      }));
      return { previous };
    },

    onError: (err, variables, context) => {
      queryClient.setQueryData(['payment', variables.paymentIntentId], context?.previous);
    },

    onSettled: (data, error, variables) => {
      queryClient.invalidateQueries({ queryKey: ['payment', variables.paymentIntentId] });
      queryClient.invalidateQueries({ queryKey: ['payments'] });
    },
  });
}
```

### Reconciliation Hooks

```typescript
// src/hooks/use-reconciliation.ts
export function useReconciliationExceptions(filters: ExceptionFilters) {
  return useInfiniteQuery({
    queryKey: ['reconciliation-exceptions', filters],
    queryFn: ({ pageParam }) =>
      api.get('/v1/reconciliation/exceptions', {
        params: { ...filters, cursor: pageParam, limit: 50 },
      }).then(r => r.data),
    getNextPageParam: (lastPage) => lastPage.pagination?.next_cursor,
    initialPageParam: undefined,
  });
}

export function useResolveException() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ exceptionId, resolution }) =>
      api.post(`/v1/reconciliation/exceptions/${exceptionId}/resolve`, resolution).then(r => r.data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['reconciliation-exceptions'] });
      queryClient.invalidateQueries({ queryKey: ['dashboard-stats'] });
    },
  });
}
```

### AI Assistant Hooks

```typescript
// src/hooks/use-assistant.ts
import { useState, useCallback } from 'react';
import api from '../lib/axios';

export function useAssistantQuery() {
  return useMutation({
    mutationFn: (data: { question: string; session_id?: string }) =>
      api.post('/v1/assistant/query', data).then(r => r.data),
  });
}

export function useAssistantStream() {
  const [response, setResponse] = useState('');
  const [citations, setCitations] = useState<Citation[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);

  const stream = useCallback((question: string, sessionId?: string) => {
    setResponse('');
    setCitations([]);
    setIsStreaming(true);

    const eventSource = new EventSource(
      `${import.meta.env.VITE_AI_URL}/v1/assistant/stream?question=${encodeURIComponent(question)}&session_id=${sessionId}`
    );

    eventSource.onmessage = (event) => {
      const chunk = JSON.parse(event.data);
      setResponse(prev => prev + chunk.text);
      if (chunk.citations) setCitations(chunk.citations);
    };

    eventSource.onerror = () => {
      eventSource.close();
      setIsStreaming(false);
    };

    return () => {
      eventSource.close();
      setIsStreaming(false);
    };
  }, []);

  return { response, citations, isStreaming, stream };
}
```

---

## 3. Optimistic Updates

```typescript
// Optimistic payment status update
export function useOptimisticCapture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ paymentIntentId, amount }) =>
      api.post(`/v1/payment-intents/${paymentIntentId}/capture`, { amount }).then(r => r.data),

    onMutate: async ({ paymentIntentId }) => {
      await queryClient.cancelQueries({ queryKey: ['payment', paymentIntentId] });
      const previous = queryClient.getQueryData(['payment', paymentIntentId]);
      queryClient.setQueryData(['payment', paymentIntentId], (old: any) => ({
        ...old, status: 'Capturing',
      }));
      return { previous };
    },

    onError: (err, variables, context) => {
      queryClient.setQueryData(['payment', variables.paymentIntentId], context?.previous);
    },

    onSettled: (data, error, variables) => {
      queryClient.invalidateQueries({ queryKey: ['payment', variables.paymentIntentId] });
    },
  });
}
```

---

## 4. Error Handling

```typescript
// src/lib/api-errors.ts
import { toast } from 'sonner'; // or your preferred toast library

export function handleApiError(error: any) {
  const apiError = error.response?.data?.error;

  if (!apiError) {
    toast.error('An unexpected error occurred');
    return;
  }

  switch (apiError.code) {
    case 'INSUFFICIENT_REFUNDABLE_BALANCE':
      toast.error(apiError.message);
      break;
    case 'DUPLICATE_IDEMPOTENCY_KEY':
      // Idempotent replay — show existing result
      toast.info('Payment already processed');
      break;
    case 'RATE_LIMITED':
      toast.warning(`Rate limited. Retry after ${apiError.details.retry_after_seconds}s`);
      break;
    case 'PAYMENT_INTENT_NOT_FOUND':
      toast.error('Payment not found');
      break;
    default:
      toast.error(apiError.message || 'An error occurred');
  }
}
```

---

## 5. Real-time Updates (WebSocket)

```typescript
// src/hooks/use-realtime.ts
import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';

export function useRealtimeUpdates() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const ws = new WebSocket(import.meta.env.VITE_WS_URL);

    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);

      switch (message.type) {
        case 'payment_status_changed':
          queryClient.invalidateQueries({ queryKey: ['payment', message.payment_intent_id] });
          queryClient.invalidateQueries({ queryKey: ['payments'] });
          queryClient.invalidateQueries({ queryKey: ['dashboard-stats'] });
          break;

        case 'settlement_matched':
          queryClient.invalidateQueries({ queryKey: ['reconciliation-exceptions'] });
          queryClient.invalidateQueries({ queryKey: ['dashboard-stats'] });
          break;

        case 'aml_alert':
          toast.warning(`AML Alert: ${message.description}`);
          break;
      }
    };

    ws.onerror = () => {
      setTimeout(() => ws.reconnect(), 5000);
    };

    return () => ws.close();
  }, [queryClient]);
}
```

---

## 6. Idempotency

```typescript
// Axios interceptor auto-generates idempotency keys
api.interceptors.request.use((config) => {
  if (['post', 'put', 'patch'].includes(config.method || '')) {
    config.headers['Idempotency-Key'] = crypto.randomUUID();
  }
  return config;
});

// Client can also supply their own for retry scenarios
const retryPayment = async (idempotencyKey: string, data: CreatePaymentIntentData) => {
  return api.post('/v1/payment-intents', data, {
    headers: { 'Idempotency-Key': idempotencyKey }, // Same key = idempotent retry
  }).then(r => r.data);
};
```
