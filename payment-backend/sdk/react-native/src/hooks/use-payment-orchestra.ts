/**
 * usePaymentOrchestra Hook
 *
 * React hook for initializing and using the Payment Orchestra client.
 */

import { useState, useEffect, useRef } from 'react';
import { PaymentOrchestra } from '../client';
import type { UsePaymentOrchestraResult } from '../types';

interface UsePaymentOrchestraOptions {
  apiKey: string;
  environment?: 'sandbox' | 'production';
  baseUrl?: string;
  debug?: boolean;
  autoInitialize?: boolean;
}

/**
 * Hook to initialize and access the Payment Orchestra client
 *
 * @example
 * ```tsx
 * function App() {
 *   const { client, isLoading, error } = usePaymentOrchestra({
 *     apiKey: 'pk_test_...',
 *     environment: 'sandbox',
 *   });
 *
 *   if (isLoading) return <Loading />;
 *   if (error) return <Error message={error.message} />;
 *
 *   return <PaymentForm client={client} />;
 * }
 * ```
 */
export function usePaymentOrchestra(
  options: UsePaymentOrchestraOptions
): UsePaymentOrchestraResult {
  const [client, setClient] = useState<PaymentOrchestra | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const clientRef = useRef<PaymentOrchestra | null>(null);

  useEffect(() => {
    let mounted = true;

    const initialize = async () => {
      try {
        setIsLoading(true);
        setError(null);

        // Create client instance
        const newClient = new PaymentOrchestra({
          apiKey: options.apiKey,
          environment: options.environment,
          baseUrl: options.baseUrl,
          debug: options.debug,
        });

        // Validate setup
        const validation = await newClient.validateSetup();
        if (!validation.valid) {
          throw new Error(
            `Setup validation failed: ${validation.issues.join(', ')}`
          );
        }

        if (mounted) {
          clientRef.current = newClient;
          setClient(newClient);
          setIsLoading(false);
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err : new Error(String(err)));
          setIsLoading(false);
        }
      }
    };

    if (options.autoInitialize !== false && options.apiKey) {
      initialize();
    } else {
      setIsLoading(false);
    }

    return () => {
      mounted = false;
    };
  }, [options.apiKey, options.environment, options.baseUrl]);

  return {
    client,
    isLoading,
    error,
  };
}

export default usePaymentOrchestra;
