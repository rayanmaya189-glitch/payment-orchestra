package com.paymentorchestra

import android.app.Activity
import android.content.Intent
import com.facebook.react.bridge.*
import com.facebook.react.modules.core.DeviceEventManagerModule
import com.google.android.gms.wallet.*
import com.google.android.gms.wallet.WalletConstants.ENVIRONMENT_TEST

/**
 * PaymentOrchestraGooglePayModule - Android Native Module
 *
 * Native bridge for Google Pay integration in React Native.
 */
class PaymentOrchestraGooglePayModule(reactContext: ReactApplicationContext) :
    ReactContextBaseJavaModule(reactContext) {

    private var paymentsClient: PaymentsClient? = null
    private var promise: Promise? = null
    private val googlePayRequestCode = 10001

    override fun getName(): String {
        return "PaymentOrchestraGooglePay"
    }

    /**
     * Check if Google Pay is available
     */
    @ReactMethod
    fun canMakePayments(promise: Promise) {
        try {
            val client = getPaymentsClient(ENVIRONMENT_TEST)
            val request = IsReadyToPayRequest.newBuilder()
                .addAllowedPaymentMethod(WalletConstants.PAYMENT_METHOD_CARD)
                .addAllowedPaymentMethod(WalletConstants.PAYMENT_METHOD_TOKENIZED_CARD)
                .build()

            client.isReadyToPay(request)
                .addOnSuccessListener { result ->
                    promise.resolve(result)
                }
                .addOnFailureListener { e ->
                    promise.resolve(false)
                }
        } catch (e: Exception) {
            promise.reject("ERROR", e.message, e)
        }
    }

    /**
     * Check if Google Pay is ready to process payments with specific card networks
     */
    @ReactMethod
    fun isReadyToPay(allowedPaymentMethods: ReadableArray, promise: Promise) {
        try {
            val client = getPaymentsClient(ENVIRONMENT_TEST)
            val request = IsReadyToPayRequest.newBuilder()
                .addAllowedPaymentMethod(WalletConstants.PAYMENT_METHOD_CARD)
                .addAllowedPaymentMethod(WalletConstants.PAYMENT_METHOD_TOKENIZED_CARD)
                .build()

            client.isReadyToPay(request)
                .addOnSuccessListener { result ->
                    promise.resolve(result)
                }
                .addOnFailureListener { e ->
                    promise.resolve(false)
                }
        } catch (e: Exception) {
            promise.reject("ERROR", e.message, e)
        }
    }

    /**
     * Authorize Google Pay payment
     */
    @ReactMethod
    fun authorize(options: ReadableMap, promise: Promise) {
        this.promise = promise

        try {
            val merchantId = options.getString("merchantId") ?: ""
            val merchantName = options.getString("merchantName") ?: "Payment"
            val environment = options.getInt("environment", ENVIRONMENT_TEST)
            val amount = if (options.hasKey("amount")) options.getDouble("amount") else null
            val currency = options.getString("currency") ?: "USD"

            val paymentsClient = getPaymentsClient(environment)
            this.paymentsClient = paymentsClient

            // Create payment data request
            val request = createPaymentDataRequest(merchantName, amount, currency)

            // Auto-resolve to show payment sheet
            val autoResolveClient = getPaymentsClient(environment)

            // For development, simulate successful payment
            if (amount != null) {
                // Create mock payment data for testing
                val paymentData = createMockPaymentData(merchantId, currency, amount)
                handlePaymentSuccess(paymentData)
            } else {
                promise.reject("MISSING_AMOUNT", "Amount is required")
            }
        } catch (e: Exception) {
            promise.reject("ERROR", e.message, e)
        }
    }

    /**
     * Create a payment request for advanced use cases
     */
    @ReactMethod
    fun createPaymentRequest(params: ReadableMap, promise: Promise) {
        try {
            val merchantName = params.getMap("merchantInfo")?.getString("merchantName") ?: ""
            val environment = ENVIRONMENT_TEST

            // Create payment data request
            val request = createPaymentDataRequest(merchantName, null, "USD")

            // For development, return mock data
            val result = Arguments.createMap().apply {
                putMap("paymentMethodData", Arguments.createMap().apply {
                    putMap("tokenizationData", Arguments.createMap().apply {
                        putString("token", "mock_google_pay_token_${System.currentTimeMillis()}")
                        putString("type", "PAYMENT_GATEWAY")
                    })
                    putMap("info", Arguments.createMap().apply {
                        putString("cardNetwork", "VISA")
                        putString("cardDetails", "4242")
                    })
                    putString("type", "CARD")
                })
            }

            promise.resolve(result)
        } catch (e: Exception) {
            promise.reject("ERROR", e.message, e)
        }
    }

    /**
     * Create a payment data request
     */
    private fun createPaymentDataRequest(
        merchantName: String,
        amount: Double?,
        currency: String
    ): PaymentDataRequest {
        val builder = PaymentDataRequest.newBuilder()
            .setTransactionInfo(
                TransactionInfo.newBuilder()
                    .setTotalPriceStatus(
                        if (amount != null) WalletConstants.TOTAL_PRICE_STATUS_FINAL
                        else WalletConstants.TOTAL_PRICE_STATUS_ESTIMATED
                    )
                    .setTotalPrice(amount?.toString() ?: "0.00")
                    .setCurrencyCode(currency)
                    .build()
            )
            .setMerchantInfo(
                MerchantInfo.newBuilder()
                    .setMerchantName(merchantName)
                    .build()
            )
            .addAllowedPaymentMethod(WalletConstants.PAYMENT_METHOD_CARD)
            .addAllowedPaymentMethod(WalletConstants.PAYMENT_METHOD_TOKENIZED_CARD)

        // Card parameters
        val cardParams = PaymentMethodTokenizationParameters.newBuilder()
            .setPaymentMethodTokenizationType(
                WalletConstants.PAYMENT_METHOD_TOKENIZATION_TYPE_PAYMENT_GATEWAY
            )
            .addParameter("gateway", "stripe")
            .addParameter("stripe:publishableKey", "pk_test_...")
            .build()

        builder.setPaymentMethodTokenizationParameters(cardParams)
        builder.setCardRequirements(
            CardRequirements.newBuilder()
                .addAllowedCardNetwork(WalletConstants.CARD_NETWORK_VISA)
                .addAllowedCardNetwork(WalletConstants.CARD_NETWORK_MASTERCARD)
                .addAllowedCardNetwork(WalletConstants.CARD_NETWORK_AMEX)
                .addAllowedCardNetwork(WalletConstants.CARD_NETWORK_DISCOVER)
                .setAllowPrepaidCards(true)
                .setBillingAddressRequired(true)
                .build()
        )

        return builder.build()
    }

    /**
     * Create mock payment data for testing
     */
    private fun createMockPaymentData(
        merchantId: String,
        currency: String,
        amount: Double
    ): PaymentData {
        // This is a simplified mock - in production, use real Google Pay flow
        // For testing purposes, we create a mock PaymentData object
        throw NotImplementedError("Use real Google Pay flow in production")
    }

    /**
     * Handle successful payment
     */
    private fun handlePaymentSuccess(paymentData: PaymentData) {
        val paymentMethodData = Arguments.createMap().apply {
            putMap("tokenizationData", Arguments.createMap().apply {
                putString("token", paymentData.paymentMethodToken?.token ?: "")
                putString("type", "PAYMENT_GATEWAY")
            })
            putMap("info", Arguments.createMap().apply {
                putString("cardNetwork", paymentData.cardInfo?.cardNetwork ?: "")
                putString("cardDetails", paymentData.cardInfo?.cardDetailsNumber ?: "")
            })
            putString("type", paymentData.paymentMethodType ?: "CARD")
        })

        val result = Arguments.createMap().apply {
            putMap("paymentMethodData", paymentMethodData)
        }

        promise?.resolve(result)
        promise = null
    }

    /**
     * Get or create PaymentsClient
     */
    private fun getPaymentsClient(environment: Int): PaymentsClient {
        return paymentsClient ?: run {
            val options = Wallet.WalletOptions.Builder()
                .setEnvironment(environment)
                .build()
            Wallet.getPaymentsClient(reactApplicationContext, options).also {
                paymentsClient = it
            }
        }
    }

    /**
     * Send event to React Native
     */
    private fun sendEvent(eventName: String, params: WritableMap?) {
        reactApplicationContext
            .getJSModule(DeviceEventManagerModule.RCTDeviceEventEmitter::class.java)
            .emit(eventName, params)
    }

    companion object {
        const val MODULE_NAME = "PaymentOrchestraGooglePay"
    }
}
