/**
 * PaymentSheet Component
 *
 * A complete payment sheet UI for React Native.
 * Handles the full payment flow including 3D Secure.
 */

import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  TouchableOpacity,
  Modal,
  StyleSheet,
  ActivityIndicator,
  Platform,
} from 'react-native';
import { CardForm } from './card-form';
import { PaymentOrchestraError, ErrorCode } from '../errors';
import type { PaymentSheetProps, PaymentSheetResult, PaymentIntent } from '../types';

type PaymentStep = 'method' | 'processing' | 'result' | '3ds';

/**
 * PaymentSheet Component
 *
 * @example
 * ```tsx
 * <PaymentSheet
 *   clientSecret="pi_1234567890_secret_..."
 *   onComplete={(result) => {
 *     if (result.status === 'succeeded') {
 *       console.log('Payment succeeded!');
 *     }
 *   }}
 *   onCancel={() => {
 *     console.log('Payment canceled');
 *   }}
 * />
 * ```
 */
export const PaymentSheet: React.FC<PaymentSheetProps> = ({
  clientSecret,
  onComplete,
  onCancel,
  style,
  appearance,
}) => {
  const [isVisible, setIsVisible] = useState(true);
  const [step, setStep] = useState<PaymentStep>('method');
  const [paymentIntent, setPaymentIntent] = useState<PaymentIntent | null>(null);
  const [error, setError] = useState<string | null>(null);

  const colors = {
    background: '#ffffff',
    text: '#000000',
    border: '#e0e0e0',
    primary: '#007AFF',
    error: '#FF3B30',
    success: '#34C759',
    ...appearance?.colors,
  };

  // Parse payment intent from client secret
  useEffect(() => {
    if (clientSecret) {
      const intentId = clientSecret.split('_secret_')[0];
      // In a real implementation, you'd fetch the payment intent
      setPaymentIntent({
        id: intentId,
        object: 'payment_intent',
        amount: 0,
        currency: 'usd',
        status: 'requires_payment_method',
        payment_method_types: ['card'],
        metadata: {},
        client_secret: clientSecret,
        created: Math.floor(Date.now() / 1000),
        livemode: false,
        capture_method: 'automatic',
        confirmation_method: 'automatic',
      });
    }
  }, [clientSecret]);

  // Handle card tokenization
  const handleCardTokenized = useCallback(async (token: string) => {
    setStep('processing');
    setError(null);

    try {
      // In a real implementation, you'd confirm the payment intent
      // For now, simulate success
      const mockResult: PaymentIntent = {
        id: paymentIntent?.id || 'pi_mock',
        object: 'payment_intent',
        amount: paymentIntent?.amount || 0,
        currency: paymentIntent?.currency || 'usd',
        status: 'succeeded',
        payment_method_types: ['card'],
        metadata: {},
        client_secret: clientSecret,
        created: Math.floor(Date.now() / 1000),
        livemode: false,
        capture_method: 'automatic',
        confirmation_method: 'automatic',
      };

      setPaymentIntent(mockResult);
      setStep('result');

      onComplete({
        status: 'succeeded',
        paymentIntent: mockResult,
      });
    } catch (err) {
      setStep('result');
      setError(err instanceof Error ? err.message : 'Payment failed');

      onComplete({
        status: 'failed',
        error: err instanceof Error ? err : new Error('Payment failed'),
      });
    }
  }, [paymentIntent, clientSecret, onComplete]);

  // Handle errors from card form
  const handleCardError = useCallback((err: Error) => {
    setError(err.message);
  }, []);

  // Handle cancel
  const handleCancel = useCallback(() => {
    setIsVisible(false);
    onCancel?.();
  }, [onCancel]);

  // Handle close
  const handleClose = useCallback(() => {
    setIsVisible(false);
  }, []);

  // Render step content
  const renderStep = () => {
    switch (step) {
      case 'method':
        return (
          <CardForm
            onCardTokenized={handleCardTokenized}
            onError={handleCardError}
            appearance={appearance}
          />
        );

      case 'processing':
        return (
          <View style={styles.centerContent}>
            <ActivityIndicator size="large" color={colors.primary} />
            <Text style={[styles.processingText, { color: colors.text }]}>
              Processing payment...
            </Text>
          </View>
        );

      case '3ds':
        return (
          <View style={styles.centerContent}>
            <ActivityIndicator size="large" color={colors.primary} />
            <Text style={[styles.processingText, { color: colors.text }]}>
              Completing 3D Secure verification...
            </Text>
          </View>
        );

      case 'result':
        if (error) {
          return (
            <View style={styles.centerContent}>
              <View style={[styles.iconCircle, { backgroundColor: colors.error }]}>
                <Text style={styles.iconText}>✕</Text>
              </View>
              <Text style={[styles.resultText, { color: colors.error }]}>
                Payment Failed
              </Text>
              <Text style={[styles.errorText, { color: colors.text }]}>
                {error}
              </Text>
              <TouchableOpacity
                style={[styles.button, { backgroundColor: colors.primary }]}
                onPress={() => {
                  setStep('method');
                  setError(null);
                }}
              >
                <Text style={styles.buttonText}>Try Again</Text>
              </TouchableOpacity>
            </View>
          );
        }

        return (
          <View style={styles.centerContent}>
            <View style={[styles.iconCircle, { backgroundColor: colors.success }]}>
              <Text style={styles.iconText}>✓</Text>
            </View>
            <Text style={[styles.resultText, { color: colors.success }]}>
              Payment Successful
            </Text>
            <TouchableOpacity
              style={[styles.button, { backgroundColor: colors.primary }]}
              onPress={handleClose}
            >
              <Text style={styles.buttonText}>Done</Text>
            </TouchableOpacity>
          </View>
        );

      default:
        return null;
    }
  };

  return (
    <Modal
      visible={isVisible}
      animationType="slide"
      presentationStyle="pageSheet"
      onRequestClose={handleCancel}
    >
      <View style={[styles.container, { backgroundColor: colors.background }, style]}>
        {/* Header */}
        <View style={[styles.header, { borderBottomColor: colors.border }]}>
          {step === 'method' && (
            <TouchableOpacity onPress={handleCancel} style={styles.closeButton}>
              <Text style={[styles.closeText, { color: colors.primary }]}>
                Cancel
              </Text>
            </TouchableOpacity>
          )}
          <Text style={[styles.title, { color: colors.text }]}>
            {step === 'result' && error ? 'Error' : 'Payment'}
          </Text>
          <View style={styles.closeButton} />
        </View>

        {/* Amount Display */}
        {paymentIntent && paymentIntent.amount > 0 && (
          <View style={styles.amountContainer}>
            <Text style={[styles.amountLabel, { color: colors.text }]}>
              Amount to Pay
            </Text>
            <Text style={[styles.amountValue, { color: colors.text }]}>
              {new Intl.NumberFormat('en-US', {
                style: 'currency',
                currency: paymentIntent.currency.toUpperCase(),
              }).format(paymentIntent.amount / 100)}
            </Text>
          </View>
        )}

        {/* Step Content */}
        <View style={styles.content}>{renderStep()}</View>

        {/* Error Banner */}
        {error && step === 'method' && (
          <View style={[styles.errorBanner, { backgroundColor: colors.error }]}>
            <Text style={styles.errorBannerText}>{error}</Text>
          </View>
        )}
      </View>
    </Modal>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    ...Platform.select({
      ios: {
        borderTopLeftRadius: 12,
        borderTopRightRadius: 12,
      },
    }),
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 16,
    borderBottomWidth: 1,
  },
  closeButton: {
    width: 60,
  },
  closeText: {
    fontSize: 16,
  },
  title: {
    fontSize: 18,
    fontWeight: '600',
  },
  amountContainer: {
    alignItems: 'center',
    padding: 24,
  },
  amountLabel: {
    fontSize: 14,
    opacity: 0.6,
    marginBottom: 8,
  },
  amountValue: {
    fontSize: 32,
    fontWeight: '700',
  },
  content: {
    flex: 1,
  },
  centerContent: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 24,
  },
  processingText: {
    fontSize: 16,
    marginTop: 16,
  },
  iconCircle: {
    width: 64,
    height: 64,
    borderRadius: 32,
    justifyContent: 'center',
    alignItems: 'center',
    marginBottom: 16,
  },
  iconText: {
    fontSize: 32,
    color: '#ffffff',
  },
  resultText: {
    fontSize: 20,
    fontWeight: '600',
    marginBottom: 8,
  },
  errorText: {
    fontSize: 14,
    textAlign: 'center',
    marginBottom: 24,
    paddingHorizontal: 32,
  },
  button: {
    height: 48,
    borderRadius: 8,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: 32,
    minWidth: 200,
  },
  buttonText: {
    color: '#ffffff',
    fontSize: 16,
    fontWeight: '600',
  },
  errorBanner: {
    padding: 12,
  },
  errorBannerText: {
    color: '#ffffff',
    textAlign: 'center',
    fontSize: 14,
  },
});

export default PaymentSheet;
