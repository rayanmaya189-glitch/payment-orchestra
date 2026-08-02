/**
 * usePaymentIntent Hook
 *
 * React hook for managing a payment intent lifecycle.
 */

import { useState, useEffect, useCallback } from 'react';
import { PaymentOrchestra } from '../client';
import type {
  PaymentIntent,
  ConfirmPaymentIntentParams,
  UsePaymentIntentResult,
} from '../types';

interface UsePaymentIntentOptions {
  client: PaymentOrchestra | null;
  paymentIntentId?: string;
  autoFetch?: boolean;
}

/**
 * Hook to manage a payment intent
 *
 * @example
 * ```tsx
 * function Checkout({ client }) {
 *   const {
 *     paymentIntent,
 *     isLoading,
 *     error,
 *     confirm,
 *     cancel,
 *   } = usePaymentIntent({
 *     client,
 *     paymentIntentId: 'pi_1234567890',
 *   });
 *
 *   const handlePayment = async () => {
 *     try {
 *       const result = await confirm({
 *         payment_method: {
 *           card: { token: 'tok_1234567890' },
 *         },
 *       });
 *       console.log('Payment succeeded:', result);
 *     } catch (err) {
 *       console.error('Payment failed:', err);
 *     }
 *   };
 *
 *   return (
 *     <View>
 *       {isLoading && <ActivityIndicator />}
 *       {error && <Text>Error: {error.message}</Text>}
 *       {paymentIntent && (
 *         <Button
 *           title={`Pay $${(paymentIntent.amount / 100).toFixed(2)}`}
 *           onPress={handlePayment}
 *         />
 *       )}
 *     </View>
 *   );
 * }
 * ```
 */
export function usePaymentIntent(
  options: UsePaymentIntentOptions
): UsePaymentIntentResult {
  const { client, paymentIntentId, autoFetch = true } = options;

  const [paymentIntent, setPaymentIntent] = useState<PaymentIntent | null>(
    null
  );
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Fetch payment intent
  const fetchPaymentIntent = useCallback(async () => {
    if (!client || !paymentIntentId) {
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const intent = await client.paymentIntents.retrieve(paymentIntentId);
      setPaymentIntent(intent);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setIsLoading(false);
    }
  }, [client, paymentIntentId]);

  // Auto-fetch on mount
  useEffect(() => {
    if (autoFetch && paymentIntentId) {
      fetchPaymentIntent();
    }
  }, [autoFetch, fetchPaymentIntent, paymentIntentId]);

  // Confirm payment intent
  const confirm = useCallback(
    async (params: ConfirmPaymentIntentParams): Promise<PaymentIntent> => {
      if (!client || !paymentIntentId) {
        throw new Error('Client and paymentIntentId are required');
      }

      setIsLoading(true);
      setError(null);

      try {
        const result = await client.paymentIntents.confirm(
          paymentIntentId,
          params
        );
        setPaymentIntent(result);
        return result;
      } catch (err) {
        const error = err instanceof Error ? err : new Error(String(err));
        setError(error);
        throw error;
      } finally {
        setIsLoading(false);
      }
    },
    [client, paymentIntentId]
  );

  // Cancel payment intent
  const cancel = useCallback(async (): Promise<void> => {
    if (!client || !paymentIntentId) {
      throw new Error('Client and paymentIntentId are required');
    }

    setIsLoading(true);
    setError(null);

    try {
      const result = await client.paymentIntents.cancel(paymentIntentId);
      setPaymentIntent(result);
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      setError(error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  }, [client, paymentIntentId]);

  return {
    paymentIntent,
    isLoading,
    error,
    confirm,
    cancel,
  };
}

export default usePaymentIntent;
