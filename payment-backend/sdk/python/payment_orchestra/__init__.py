"""
Payment Orchestra Python SDK

Usage:
    from payment_orchestra import PaymentOrchestra

    client = PaymentOrchestra(api_key="pk_live_...")

    # Create a payment intent
    intent = client.payment_intents.create(
        amount=5000,
        currency="USD",
        purpose="payment"
    )

    # Authorize
    client.payment_intents.authorize(
        intent["id"],
        payment_method_token="tok_visa_4242",
        card_scheme="visa"
    )

    # Capture
    client.payment_intents.capture(intent["id"], amount=5000)
"""

from .client import PaymentOrchestra
from .exceptions import PaymentOrchestraError

__version__ = "0.1.0"
__all__ = ["PaymentOrchestra", "PaymentOrchestraError"]
