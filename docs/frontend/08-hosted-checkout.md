# 08 — Hosted Checkout Page

## 1. PCI-DSS Isolation

The hosted checkout page (`/pay/[token]`) is **completely isolated** from the admin dashboard:

| Property | Checkout Page | Admin Dashboard |
|----------|--------------|-----------------|
| Domain | `pay.platform.ae` | `admin.platform.ae` |
| Cookies | None (or payment-specific) | Session cookies |
| localStorage | None | Application state |
| CSP | Strict payment-page CSP | Dashboard CSP |
| CORS | No cross-origin requests | Whitelisted origins |
| Auth | None (public) | JWT/Session required |

---

## 2. Checkout Flow

```
1. Customer opens payment link URL
2. Page validates token → loads amount, description
3. Customer enters card details (via acquirer's tokenization SDK)
   - Card data NEVER touches our servers
   - Tokenization happens client-side via acquirer's SDK
4. Submit → POST /v1/payment-intents + AuthorizePaymentIntent
5. If 3DS required → redirect to acquirer's 3DS page
6. On completion → redirect to success/cancel URL
```

---

## 3. Checkout Page Component

```tsx
// app/pay/[token]/page.tsx
export default async function CheckoutPage({ params }) {
  const link = await getPaymentLink(params.token);

  if (!link) return <NotFound />;
  if (link.status === 'expired') return <ExpiredPage />;

  return (
    <CheckoutLayout>
      <CheckoutHeader
        merchantName={link.merchantName}
        logo={link.merchantLogo}
      />

      <CheckoutForm
        amount={link.amount}
        currency={link.currency}
        description={link.description}
        paymentMethods={link.supportedPaymentMethods}
        onSubmit={handlePayment}
      />

      <CheckoutFooter
        merchantName={link.merchantName}
        poweredBy="Payment Orchestra"
      />
    </CheckoutLayout>
  );
}
```

---

## 4. Payment Form

```tsx
<CheckoutForm
  amount={amount}
  currency={currency}
  description={description}
  paymentMethods={['card']} // MVP: card only
  onSubmit={async (paymentData) => {
    // 1. Create payment intent
    const intent = await publicApi.post('/v1/payment-intents', {
      amount: amount,
      idempotency_key: crypto.randomUUID(),
      purpose: 'payment',
    });

    // 2. Authorize with tokenized card
    const auth = await publicApi.post(`/v1/payment-intents/${intent.id}/authorize`, {
      payment_method_token: paymentData.token,
    });

    // 3. Handle result
    if (auth.status === 'Authorized') {
      // Redirect to success page
      window.location.href = `${successUrl}?payment_intent=${intent.id}`;
    } else if (auth.status === 'requires_3ds') {
      // Redirect to 3DS
      window.location.href = auth.three_ds_data.three_ds_url;
    } else {
      // Show error
      setError(auth.decline_reason || 'Payment failed');
    }
  }}
/>
```

---

## 5. Acquirer Tokenization Integration

```tsx
// Client-side tokenization via acquirer's SDK
// Card data NEVER reaches our servers

<PaymentElement
  clientSecret={acquirerClientSecret}
  options={{
    layout: 'tabs',
    fields: {
      billingDetails: {
        name: 'required',
        email: 'required',
      },
    },
  }}
  onLoadError={(error) => {
    console.error('Payment element load error:', error);
    setError('Payment form failed to load. Please try again.');
  }}
/>
```

---

## 6. 3DS Handling

```tsx
// When authorization returns Requires3DS
const handleThreeDS = async (threeDsData) => {
  // 1. Create 3DS challenge frame
  const challenge = await createThreeDSChallenge(threeDsData);

  // 2. Wait for customer completion
  const result = await challenge.authenticate();

  // 3. Submit result back to platform
  if (result.success) {
    const auth = await publicApi.post(`/v1/payment-intents/${intentId}/authorize`, {
      payment_method_token: token,
      three_ds_result: result,
    });
    // Handle final result
  }
};
```

---

## 7. Security Headers

```
Content-Security-Policy: default-src 'self'; script-src 'self' https://js.stripe.com; style-src 'self' 'unsafe-inline'; frame-src https://hooks.stripe.com;
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: no-referrer
Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()
```

---

## 8. Error States

| State | UI |
|-------|-----|
| Invalid Token | "Payment link not found or expired" |
| Expired Token | "This payment link has expired" |
| Payment Failed | Error message with decline reason |
| 3DS Required | Redirect to acquirer's 3DS page |
| Network Error | "Connection lost. Please try again." |
| Processing | Spinner with "Processing payment..." |
