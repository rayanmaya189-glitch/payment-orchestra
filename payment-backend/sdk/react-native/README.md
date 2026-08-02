# Payment Orchestra React Native SDK

A complete mobile SDK for the Payment Orchestra payment orchestration platform. Supports iOS (Apple Pay) and Android (Google Pay) with native performance.

## Features

- 🍎 **Apple Pay** - Native Apple Pay integration for iOS
- 🤖 **Google Pay** - Native Google Pay integration for Android
- 💳 **Card Tokenization** - Secure on-device tokenization
- 🔒 **3D Secure** - Built-in 3DS support
- 📱 **React Hooks** - Easy-to-use hooks for React components
- 🎨 **UI Components** - Pre-built CardForm and PaymentSheet components
- 🚀 **TypeScript** - Full TypeScript support

## Installation

```bash
# Install the package
npm install @payment-orchestra/react-native-sdk

# or
yarn add @payment-orchestra/react-native-sdk

# iOS dependencies
cd ios && pod install && cd ..
```

## Quick Start

### 1. Initialize the Client

```typescript
import { PaymentOrchestra } from '@payment-orchestra/react-native-sdk';

const client = new PaymentOrchestra({
  apiKey: 'pk_test_...',
  environment: 'sandbox', // or 'production'
  debug: true, // Enable debug logging
});
```

### 2. Create a Payment Intent

```typescript
const intent = await client.paymentIntents.create({
  amount: 5000, // $50.00
  currency: 'usd',
  payment_method_types: ['card'],
});
```

### 3. Process Payment

```typescript
// Using Card Form
const result = await client.paymentIntents.confirm(intent.id, {
  payment_method: {
    card: {
      token: 'tok_...',
    },
  },
});

// Using Apple Pay (iOS)
const result = await client.paymentIntents.payWithApplePay(intent.id, {
  merchantIdentifier: 'merchant.com.yourcompany',
  label: 'Your Store',
});

// Using Google Pay (Android)
const result = await client.paymentIntents.payWithGooglePay(intent.id, {
  merchantId: 'your-merchant-id',
  merchantName: 'Your Store',
});
```

## React Hooks

### usePaymentOrchestra

Initialize and access the Payment Orchestra client:

```tsx
import { usePaymentOrchestra } from '@payment-orchestra/react-native-sdk';

function App() {
  const { client, isLoading, error } = usePaymentOrchestra({
    apiKey: 'pk_test_...',
    environment: 'sandbox',
  });

  if (isLoading) return <Loading />;
  if (error) return <Error message={error.message} />;

  return <PaymentForm client={client} />;
}
```

### usePaymentIntent

Manage a payment intent lifecycle:

```tsx
import { usePaymentIntent } from '@payment-orchestra/react-native-sdk';

function Checkout({ client, paymentIntentId }) {
  const {
    paymentIntent,
    isLoading,
    error,
    confirm,
    cancel,
  } = usePaymentIntent({
    client,
    paymentIntentId,
  });

  const handlePayment = async () => {
    try {
      await confirm({
        payment_method: {
          card: { token: 'tok_...' },
        },
      });
    } catch (err) {
      console.error('Payment failed:', err);
    }
  };

  return (
    <View>
      {isLoading && <ActivityIndicator />}
      {error && <Text>Error: {error.message}</Text>}
      {paymentIntent && (
        <Button
          title={`Pay $${(paymentIntent.amount / 100).toFixed(2)}`}
          onPress={handlePayment}
        />
      )}
    </View>
  );
}
```

## UI Components

### CardForm

A secure card input form:

```tsx
import { CardForm } from '@payment-orchestra/react-native-sdk';

function PaymentForm() {
  return (
    <CardForm
      onCardTokenized={(token) => {
        console.log('Card tokenized:', token);
      }}
      onError={(error) => {
        console.error('Card error:', error);
      }}
      appearance={{
        colors: {
          background: '#ffffff',
          text: '#000000',
          border: '#cccccc',
          focus: '#007AFF',
        },
      }}
    />
  );
}
```

### PaymentSheet

A complete payment sheet UI:

```tsx
import { PaymentSheet } from '@payment-orchestra/react-native-sdk';

function Checkout({ clientSecret }) {
  return (
    <PaymentSheet
      clientSecret={clientSecret}
      onComplete={(result) => {
        if (result.status === 'succeeded') {
          console.log('Payment succeeded!');
        }
      }}
      onCancel={() => {
        console.log('Payment canceled');
      }}
    />
  );
}
```

## Native Payment Methods

### Apple Pay (iOS)

```typescript
// Check availability
const isAvailable = await client.applePay.isAvailable();

// Authorize payment
const token = await client.applePay.authorize({
  merchantIdentifier: 'merchant.com.yourcompany',
  countryCode: 'US',
  currencyCode: 'USD',
  supportedNetworks: ['visa', 'mastercard', 'amex'],
  merchantCapabilities: ['3ds'],
  amount: 50.00,
  label: 'Your Store',
});
```

### Google Pay (Android)

```typescript
// Check availability
const isAvailable = await client.googlePay.isAvailable();

// Check if ready to pay
const isReady = await client.googlePay.isReadyToPay(['VISA', 'MASTERCARD', 'AMEX']);

// Authorize payment
const token = await client.googlePay.authorize({
  merchantId: 'your-merchant-id',
  merchantName: 'Your Store',
  countryCode: 'US',
  environment: 'TEST', // or 'PRODUCTION'
  allowedPaymentMethods: ['CARD'],
  amount: 50.00,
  currency: 'USD',
});
```

## Resources

### PaymentIntents

```typescript
// Create
const intent = await client.paymentIntents.create({
  amount: 5000,
  currency: 'usd',
});

// Retrieve
const intent = await client.paymentIntents.retrieve('pi_...');

// Confirm
const intent = await client.paymentIntents.confirm('pi_...', {
  payment_method: { card: { token: 'tok_...' } },
});

// Cancel
await client.paymentIntents.cancel('pi_...');

// Capture (for manual capture)
await client.paymentIntents.capture('pi_...', {
  amount_to_capture: 5000,
});

// List
const intents = await client.paymentIntents.list({
  customer: 'cus_...',
  limit: 10,
});
```

### PaymentMethods

```typescript
// Create
const method = await client.paymentMethods.create({
  type: 'card',
  card: {
    number: '4242424242424242',
    exp_month: 12,
    exp_year: 2025,
    cvc: '123',
  },
});

// Tokenize card
const token = await client.paymentMethods.tokenizeCard({
  number: '4242424242424242',
  exp_month: 12,
  exp_year: 2025,
  cvc: '123',
});

// List
const methods = await client.paymentMethods.list({
  customer: 'cus_...',
  type: 'card',
});
```

### Customers

```typescript
// Create
const customer = await client.customers.create({
  email: 'john@example.com',
  name: 'John Doe',
});

// Retrieve
const customer = await client.customers.retrieve('cus_...');

// Update
const customer = await client.customers.update('cus_...', {
  metadata: { user_id: '12345' },
});

// Delete
await client.customers.delete('cus_...');
```

### Refunds

```typescript
// Create
const refund = await client.refunds.create({
  payment_intent: 'pi_...',
  amount: 2500, // Partial refund
  reason: 'requested_by_customer',
});

// Retrieve
const refund = await client.refunds.retrieve('re_...');
```

## Error Handling

```typescript
import { PaymentOrchestraError, ErrorCode } from '@payment-orchestra/react-native-sdk';

try {
  await client.paymentIntents.confirm('pi_...', {
    payment_method: { card: { token: 'tok_...' } },
  });
} catch (error) {
  if (error instanceof PaymentOrchestraError) {
    switch (error.code) {
      case ErrorCode.CARD_ERROR:
        // Card was declined
        console.error('Card error:', error.message);
        console.error('Decline code:', error.declineCode);
        break;
      case ErrorCode.THREE_D_SECURE_REQUIRED:
        // 3DS verification required
        console.log('3DS required');
        break;
      case ErrorCode.NETWORK_ERROR:
        // Network error
        console.error('Network error:', error.message);
        break;
      default:
        console.error('Error:', error.message);
    }

    // Check if error is retryable
    if (error.isRetryable()) {
      // Retry the request
    }

    // Get user-friendly message
    console.log(error.getUserMessage());
  }
}
```

## Platform-Specific Setup

### iOS Setup

1. Enable Apple Pay in your Apple Developer account
2. Add your Merchant ID to `ios/PaymentOrchestraApplePay.swift`
3. Configure your entitlements for Apple Pay
4. Run `pod install` in the ios directory

### Android Setup

1. Set up Google Pay in the Google Pay Console
2. Add your Merchant ID to `android/src/main/java/com/paymentorchestra/PaymentOrchestraGooglePayModule.kt`
3. Configure your `build.gradle` with the correct Google Play Services version
4. Add internet permission to `AndroidManifest.xml`

## Security

- Card data is tokenized on-device and never touches your server
- All communication is encrypted with TLS 1.3
- PCI DSS compliance is maintained by the SDK
- Sensitive operations require user authentication

## Testing

```bash
# Run tests
npm test

# Run iOS tests
cd ios && xcodebuild test -scheme PaymentOrchestraTests

# Run Android tests
cd android && ./gradlew test
```

## TypeScript

The SDK is written in TypeScript and includes full type definitions. Import types from the package:

```typescript
import type {
  PaymentIntent,
  PaymentMethod,
  Customer,
  Refund,
  CardFormProps,
  PaymentSheetProps,
  ApplePayConfig,
  GooglePayConfig,
} from '@payment-orchestra/react-native-sdk';
```

## License

MIT License - see LICENSE file for details.
