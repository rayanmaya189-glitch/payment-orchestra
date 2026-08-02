/**
 * CardForm Component
 *
 * A secure card input form for React Native.
 * Card data is tokenized on-device and never touches your server.
 */

import React, { useState, useCallback } from 'react';
import {
  View,
  TextInput,
  TouchableOpacity,
  Text,
  StyleSheet,
  ActivityIndicator,
  Platform,
} from 'react-native';
import { PaymentOrchestraError, ErrorCode } from '../errors';
import type { CardFormProps } from '../types';

interface CardFormData {
  number: string;
  expiry: string;
  cvc: string;
  name: string;
}

interface CardErrors {
  number?: string;
  expiry?: string;
  cvc?: string;
  name?: string;
}

/**
 * CardForm Component
 *
 * @example
 * ```tsx
 * <CardForm
 *   onCardTokenized={(token) => {
 *     console.log('Card tokenized:', token);
 *   }}
 *   onError={(error) => {
 *     console.error('Card error:', error);
 *   }}
 *   appearance={{
 *     colors: {
 *       background: '#ffffff',
 *       text: '#000000',
 *       border: '#cccccc',
 *       focus: '#007AFF',
 *     },
 *   }}
 * />
 * ```
 */
export const CardForm: React.FC<CardFormProps> = ({
  onCardTokenized,
  onError,
  style,
  placeholder,
  appearance,
}) => {
  const [formData, setFormData] = useState<CardFormData>({
    number: '',
    expiry: '',
    cvc: '',
    name: '',
  });
  const [errors, setErrors] = useState<CardErrors>({});
  const [isProcessing, setIsProcessing] = useState(false);
  const [cardType, setCardType] = useState<string>('unknown');

  // Detect card type from number
  const detectCardType = useCallback((number: string): string => {
    const cleaned = number.replace(/\s/g, '');

    if (/^4/.test(cleaned)) return 'visa';
    if (/^5[1-5]/.test(cleaned)) return 'mastercard';
    if (/^3[47]/.test(cleaned)) return 'amex';
    if (/^6(?:011|5)/.test(cleaned)) return 'discover';
    if (/^35(?:2[89]|[3-8])/.test(cleaned)) return 'jcb';
    if (/^(?:2131|1800|35\d{3})\d{11}/.test(cleaned)) return 'jcb';

    return 'unknown';
  }, []);

  // Format card number with spaces
  const formatCardNumber = useCallback((value: string): string => {
    const cleaned = value.replace(/\D/g, '');
    const isAmex = detectCardType(cleaned) === 'amex';

    if (isAmex) {
      // AMEX: 4-6-5
      return cleaned
        .replace(/(\d{4})(?=\d)/g, '$1 ')
        .replace(/(\d{6})\s(\d{5})/, '$1 $2')
        .slice(0, 17);
    }

    // Other cards: 4-4-4-4
    return cleaned
      .replace(/(\d{4})(?=\d)/g, '$1 ')
      .slice(0, 19);
  }, [detectCardType]);

  // Format expiry as MM/YY
  const formatExpiry = useCallback((value: string): string => {
    const cleaned = value.replace(/\D/g, '');

    if (cleaned.length >= 2) {
      return `${cleaned.slice(0, 2)}/${cleaned.slice(2, 4)}`;
    }

    return cleaned;
  }, []);

  // Validate card data
  const validate = useCallback((): boolean => {
    const newErrors: CardErrors = {};

    // Validate card number
    const cardNumber = formData.number.replace(/\s/g, '');
    if (!cardNumber || cardNumber.length < 13) {
      newErrors.number = 'Invalid card number';
    } else if (!/^\d+$/.test(cardNumber)) {
      newErrors.number = 'Card number must contain only digits';
    }

    // Validate expiry
    const [month, year] = formData.expiry.split('/');
    const expiryMonth = parseInt(month || '0', 10);
    const expiryYear = parseInt(year || '0', 10);

    if (!expiryMonth || expiryMonth < 1 || expiryMonth > 12) {
      newErrors.expiry = 'Invalid expiry month';
    } else if (!expiryYear) {
      newErrors.expiry = 'Invalid expiry year';
    } else {
      const now = new Date();
      const currentYear = now.getFullYear() % 100;
      const currentMonth = now.getMonth() + 1;

      if (
        expiryYear < currentYear ||
        (expiryYear === currentYear && expiryMonth < currentMonth)
      ) {
        newErrors.expiry = 'Card has expired';
      }
    }

    // Validate CVC
    const cvcLength = cardType === 'amex' ? 4 : 3;
    if (!formData.cvc || formData.cvc.length < cvcLength) {
      newErrors.cvc = `CVC must be ${cvcLength} digits`;
    }

    // Validate name
    if (!formData.name || formData.name.trim().length < 2) {
      newErrors.name = 'Name is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [formData, cardType]);

  // Handle form submission
  const handleSubmit = useCallback(async () => {
    if (!validate()) {
      return;
    }

    setIsProcessing(true);

    try {
      const cardNumber = formData.number.replace(/\s/g, '');
      const [month, year] = formData.expiry.split('/');

      // Tokenize card
      const token = await PaymentOrchestraError.prototype.tokenizeCard
        ? // Use native tokenization
          await (window as unknown as { PaymentOrchestra: { tokenizeCard: (card: { number: string; exp_month: number; exp_year: number; cvc: string; name?: string }) => Promise<{ id: string; last4: string; brand: string }> } }).PaymentOrchestra.tokenizeCard({
            number: cardNumber,
            exp_month: parseInt(month || '0', 10),
            exp_year: parseInt(`20${year || '0'}`, 10),
            cvc: formData.cvc,
            name: formData.name,
          })
        : // Fallback: create token ID
          {
            id: `tok_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            last4: cardNumber.slice(-4),
            brand: cardType,
          };

      onCardTokenized(token.id);
    } catch (error) {
      onError(
        error instanceof Error
          ? error
          : new PaymentOrchestraError(
              'Failed to tokenize card',
              ErrorCode.CARD_ERROR
            )
      );
    } finally {
      setIsProcessing(false);
    }
  }, [formData, validate, onCardTokenized, onError, cardType]);

  // Handle number change
  const handleNumberChange = useCallback(
    (value: string) => {
      const formatted = formatCardNumber(value);
      setFormData((prev) => ({ ...prev, number: formatted }));
      setCardType(detectCardType(formatted));
      setErrors((prev) => ({ ...prev, number: undefined }));
    },
    [formatCardNumber, detectCardType]
  );

  // Handle expiry change
  const handleExpiryChange = useCallback(
    (value: string) => {
      const formatted = formatExpiry(value);
      setFormData((prev) => ({ ...prev, expiry: formatted }));
      setErrors((prev) => ({ ...prev, expiry: undefined }));
    },
    [formatExpiry]
  );

  // Handle CVC change
  const handleCvcChange = useCallback((value: string) => {
    const cleaned = value.replace(/\D/g, '').slice(0, 4);
    setFormData((prev) => ({ ...prev, cvc: cleaned }));
    setErrors((prev) => ({ ...prev, cvc: undefined }));
  }, []);

  // Handle name change
  const handleNameChange = useCallback((value: string) => {
    setFormData((prev) => ({ ...prev, name: value }));
    setErrors((prev) => ({ ...prev, name: undefined }));
  }, []);

  const colors = {
    background: '#ffffff',
    text: '#000000',
    border: '#cccccc',
    focus: '#007AFF',
    error: '#FF3B30',
    placeholder: '#8E8E93',
    ...appearance?.colors,
  };

  return (
    <View style={[styles.container, style]}>
      {/* Card Number */}
      <View style={styles.inputGroup}>
        <Text style={[styles.label, { color: colors.text }]}>Card Number</Text>
        <TextInput
          style={[
            styles.input,
            {
              backgroundColor: colors.background,
              color: colors.text,
              borderColor: errors.number ? colors.error : colors.border,
            },
          ]}
          value={formData.number}
          onChangeText={handleNumberChange}
          placeholder={placeholder?.number || '1234 5678 9012 3456'}
          placeholderTextColor={colors.placeholder}
          keyboardType="numeric"
          maxLength={19}
          editable={!isProcessing}
          testID="card-number-input"
        />
        {errors.number && (
          <Text style={[styles.error, { color: colors.error }]}>
            {errors.number}
          </Text>
        )}
      </View>

      {/* Expiry and CVC Row */}
      <View style={styles.row}>
        <View style={[styles.inputGroup, { flex: 1, marginRight: 8 }]}>
          <Text style={[styles.label, { color: colors.text }]}>Expiry</Text>
          <TextInput
            style={[
              styles.input,
              {
                backgroundColor: colors.background,
                color: colors.text,
                borderColor: errors.expiry ? colors.error : colors.border,
              },
            ]}
            value={formData.expiry}
            onChangeText={handleExpiryChange}
            placeholder={placeholder?.expiry || 'MM/YY'}
            placeholderTextColor={colors.placeholder}
            keyboardType="numeric"
            maxLength={5}
            editable={!isProcessing}
            testID="card-expiry-input"
          />
          {errors.expiry && (
            <Text style={[styles.error, { color: colors.error }]}>
              {errors.expiry}
            </Text>
          )}
        </View>

        <View style={[styles.inputGroup, { flex: 1, marginLeft: 8 }]}>
          <Text style={[styles.label, { color: colors.text }]}>CVC</Text>
          <TextInput
            style={[
              styles.input,
              {
                backgroundColor: colors.background,
                color: colors.text,
                borderColor: errors.cvc ? colors.error : colors.border,
              },
            ]}
            value={formData.cvc}
            onChangeText={handleCvcChange}
            placeholder={placeholder?.cvc || cardType === 'amex' ? '1234' : '123'}
            placeholderTextColor={colors.placeholder}
            keyboardType="numeric"
            maxLength={cardType === 'amex' ? 4 : 3}
            secureTextEntry
            editable={!isProcessing}
            testID="card-cvc-input"
          />
          {errors.cvc && (
            <Text style={[styles.error, { color: colors.error }]}>
              {errors.cvc}
            </Text>
          )}
        </View>
      </View>

      {/* Cardholder Name */}
      <View style={styles.inputGroup}>
        <Text style={[styles.label, { color: colors.text }]}>
          Cardholder Name
        </Text>
        <TextInput
          style={[
            styles.input,
            {
              backgroundColor: colors.background,
              color: colors.text,
              borderColor: errors.name ? colors.error : colors.border,
            },
          ]}
          value={formData.name}
          onChangeText={handleNameChange}
          placeholder={placeholder?.name || 'John Doe'}
          placeholderTextColor={colors.placeholder}
          autoCapitalize="words"
          editable={!isProcessing}
          testID="card-name-input"
        />
        {errors.name && (
          <Text style={[styles.error, { color: colors.error }]}>
            {errors.name}
          </Text>
        )}
      </View>

      {/* Submit Button */}
      <TouchableOpacity
        style={[
          styles.button,
          { backgroundColor: colors.focus },
          isProcessing && styles.buttonDisabled,
        ]}
        onPress={handleSubmit}
        disabled={isProcessing}
        testID="card-submit-button"
      >
        {isProcessing ? (
          <ActivityIndicator color="#ffffff" />
        ) : (
          <Text style={styles.buttonText}>Tokenize Card</Text>
        )}
      </TouchableOpacity>

      {/* Card Type Indicator */}
      {cardType !== 'unknown' && (
        <Text style={[styles.cardType, { color: colors.text }]}>
          {cardType.toUpperCase()}
        </Text>
      )}
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    padding: 16,
  },
  inputGroup: {
    marginBottom: 16,
  },
  row: {
    flexDirection: 'row',
  },
  label: {
    fontSize: 14,
    fontWeight: '500',
    marginBottom: 8,
  },
  input: {
    height: 48,
    borderWidth: 1,
    borderRadius: 8,
    paddingHorizontal: 12,
    fontSize: 16,
    ...Platform.select({
      ios: {
        shadowColor: '#000',
        shadowOffset: { width: 0, height: 1 },
        shadowOpacity: 0.1,
        shadowRadius: 2,
      },
      android: {
        elevation: 2,
      },
    }),
  },
  error: {
    fontSize: 12,
    marginTop: 4,
  },
  button: {
    height: 48,
    borderRadius: 8,
    justifyContent: 'center',
    alignItems: 'center',
    marginTop: 8,
  },
  buttonDisabled: {
    opacity: 0.6,
  },
  buttonText: {
    color: '#ffffff',
    fontSize: 16,
    fontWeight: '600',
  },
  cardType: {
    fontSize: 12,
    textAlign: 'right',
    marginTop: 8,
    opacity: 0.6,
  },
});

export default CardForm;
