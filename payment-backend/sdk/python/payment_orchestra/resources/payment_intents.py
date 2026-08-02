"""
Payment Intents resource for managing payment transactions.
"""

from typing import Optional, Dict, Any, List


class PaymentIntents:
    """
    Manage payment intents.

    Example:
        >>> # Create a payment intent
        >>> payment = client.payment_intents.create(
        ...     amount=10000,
        ...     currency="USD",
        ...     metadata={"order_id": "ORD-001"}
        ... )
        >>>
        >>> # Get payment intent
        >>> payment = client.payment_intents.get("pi_abc123")
        >>>
        >>> # List payment intents
        >>> payments = client.payment_intents.list(limit=10)
    """

    def __init__(self, client):
        self._client = client

    def create(
        self,
        amount: int,
        currency: str,
        order_id: Optional[str] = None,
        customer_id: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
        **kwargs,
    ) -> Dict[str, Any]:
        """
        Create a new payment intent.

        Args:
            amount: Amount in minor units (e.g., 10000 for $100.00)
            currency: Three-letter currency code (e.g., 'USD', 'EUR')
            order_id: Optional order reference
            customer_id: Optional customer identifier
            metadata: Optional metadata dictionary
            **kwargs: Additional parameters

        Returns:
            Payment intent object
        """
        payload = {
            "amount": amount,
            "currency": currency,
        }
        if order_id:
            payload["order_id"] = order_id
        if customer_id:
            payload["customer_id"] = customer_id
        if metadata:
            payload["metadata"] = metadata
        payload.update(kwargs)

        return self._client._request("POST", "/v1/payment-intents", json=payload)

    def get(self, payment_intent_id: str) -> Dict[str, Any]:
        """
        Retrieve a payment intent by ID.

        Args:
            payment_intent_id: The payment intent ID

        Returns:
            Payment intent object
        """
        return self._client._request("GET", f"/v1/payment-intents/{payment_intent_id}")

    def list(
        self,
        limit: int = 10,
        offset: int = 0,
        status: Optional[str] = None,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
    ) -> Dict[str, Any]:
        """
        List payment intents.

        Args:
            limit: Maximum number of results (default: 10)
            offset: Number of results to skip (default: 0)
            status: Filter by status
            start_date: Filter by start date (ISO 8601)
            end_date: Filter by end date (ISO 8601)

        Returns:
            Dictionary with 'items' and 'total' keys
        """
        params = {"limit": limit, "offset": offset}
        if status:
            params["status"] = status
        if start_date:
            params["start_date"] = start_date
        if end_date:
            params["end_date"] = end_date

        return self._client._request("GET", "/v1/payment-intents", params=params)

    def capture(
        self, payment_intent_id: str, amount: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Capture a payment intent.

        Args:
            payment_intent_id: The payment intent ID
            amount: Optional amount to capture (partial capture)

        Returns:
            Updated payment intent object
        """
        payload = {}
        if amount is not None:
            payload["amount"] = amount

        return self._client._request(
            "POST", f"/v1/payment-intents/{payment_intent_id}/capture", json=payload
        )

    def void(self, payment_intent_id: str) -> Dict[str, Any]:
        """
        Void (cancel) a payment intent.

        Args:
            payment_intent_id: The payment intent ID

        Returns:
            Updated payment intent object
        """
        return self._client._request(
            "POST", f"/v1/payment-intents/{payment_intent_id}/void"
        )

    def refund(
        self, payment_intent_id: str, amount: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Refund a payment intent.

        Args:
            payment_intent_id: The payment intent ID
            amount: Optional amount to refund (partial refund)

        Returns:
            Updated payment intent object
        """
        payload = {}
        if amount is not None:
            payload["amount"] = amount

        return self._client._request(
            "POST", f"/v1/payment-intents/{payment_intent_id}/refund", json=payload
        )

    def authorize(self, payment_intent_id: str) -> Dict[str, Any]:
        """
        Authorize a payment intent.

        Args:
            payment_intent_id: The payment intent ID

        Returns:
            Updated payment intent object
        """
        return self._client._request(
            "POST", f"/v1/payment-intents/{payment_intent_id}/authorize"
        )
