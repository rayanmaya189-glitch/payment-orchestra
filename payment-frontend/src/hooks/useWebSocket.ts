import { useEffect, useRef, useCallback, useState } from 'react';

interface WebSocketOptions {
  url: string;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  onMessage?: (data: unknown) => void;
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (error: Event) => void;
}

interface WebSocketState {
  isConnected: boolean;
  lastMessage: unknown | null;
  error: Event | null;
}

export function useWebSocket({
  url,
  reconnectInterval = 3000,
  maxReconnectAttempts = 10,
  onMessage,
  onOpen,
  onClose,
  onError,
}: WebSocketOptions) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [state, setState] = useState<WebSocketState>({
    isConnected: false,
    lastMessage: null,
    error: null,
  });

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        setState((prev) => ({ ...prev, isConnected: true, error: null }));
        reconnectAttemptsRef.current = 0;
        onOpen?.();
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          setState((prev) => ({ ...prev, lastMessage: data }));
          onMessage?.(data);
        } catch {
          // Handle non-JSON messages
          setState((prev) => ({ ...prev, lastMessage: event.data }));
          onMessage?.(event.data);
        }
      };

      ws.onclose = () => {
        setState((prev) => ({ ...prev, isConnected: false }));
        onClose?.();

        // Attempt to reconnect
        if (reconnectAttemptsRef.current < maxReconnectAttempts) {
          reconnectTimeoutRef.current = setTimeout(() => {
            reconnectAttemptsRef.current += 1;
            connect();
          }, reconnectInterval * Math.pow(2, reconnectAttemptsRef.current));
        }
      };

      ws.onerror = (error) => {
        setState((prev) => ({ ...prev, error }));
        onError?.(error);
      };
    } catch (error) {
      console.error('WebSocket connection error:', error);
    }
  }, [url, reconnectInterval, maxReconnectAttempts, onMessage, onOpen, onClose, onError]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
    }
    wsRef.current?.close();
    wsRef.current = null;
  }, []);

  const send = useCallback((data: unknown) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(data));
    }
  }, []);

  useEffect(() => {
    connect();
    return () => disconnect();
  }, [connect, disconnect]);

  return {
    ...state,
    send,
    reconnect: connect,
    disconnect,
  };
}

// Example usage for payment updates
export function usePaymentUpdates(operatorId: string | undefined) {
  const [payments, setPayments] = useState<unknown[]>([]);

  const handlePaymentUpdate = useCallback((data: unknown) => {
    setPayments((prev) => {
      const typedData = data as { type: string; payment: unknown };
      if (typedData.type === 'payment_created') {
        return [typedData.payment, ...prev].slice(0, 50); // Keep last 50
      }
      if (typedData.type === 'payment_updated') {
        return prev.map((p) =>
          (p as { id: string }).id === (typedData.payment as { id: string }).id
            ? typedData.payment
            : p
        );
      }
      return prev;
    });
  }, []);

  const ws = useWebSocket({
    url: operatorId
      ? `wss://api.paymentorchestra.com/ws/payments?operator_id=${operatorId}`
      : '',
    onMessage: handlePaymentUpdate,
  });

  return {
    ...ws,
    payments,
  };
}

// Example usage for gateway health updates
export function useGatewayHealthUpdates() {
  const [gateways, setGateways] = useState<Map<string, unknown>>(new Map());

  const handleGatewayUpdate = useCallback((data: unknown) => {
    setGateways((prev) => {
      const typedData = data as { gateway_id: string; status: string };
      const next = new Map(prev);
      next.set(typedData.gateway_id, data);
      return next;
    });
  }, []);

  const ws = useWebSocket({
    url: 'wss://api.paymentorchestra.com/ws/gateways',
    onMessage: handleGatewayUpdate,
  });

  return {
    ...ws,
    gateways: Array.from(gateways.values()),
  };
}
