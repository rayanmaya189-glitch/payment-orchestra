"""
Webhooks resource for managing webhook endpoints.
"""

from typing import Optional, Dict, Any, List


class Webhooks:
    """
    Manage webhook endpoints.

    Example:
        >>> # Create a webhook endpoint
        >>> webhook = client.webhooks.create(
        ...     url="https://yourapp.com/webhooks/payment",
        ...     events=["payment_intent.authorized", "payment_intent.captured"]
        ... )
    """

    def __init__(self, client):
        self._client = client

    def list(self) -> List[Dict[str, Any]]:
        """
        List all webhook endpoints.

        Returns:
            List of webhook endpoint objects
        """
        return self._client._request("GET", "/v1/webhooks")

    def get(self, webhook_id: str) -> Dict[str, Any]:
        """
        Retrieve a webhook endpoint by ID.

        Args:
            webhook_id: The webhook endpoint ID

        Returns:
            Webhook endpoint object
        """
        return self._client._request("GET", f"/v1/webhooks/{webhook_id}")

    def create(
        self,
        url: str,
        events: List[str],
        **kwargs,
    ) -> Dict[str, Any]:
        """
        Create a new webhook endpoint.

        Args:
            url: Webhook endpoint URL
            events: List of event types to subscribe to
            **kwargs: Additional parameters

        Returns:
            Created webhook endpoint object
        """
        payload = {
            "url": url,
            "events": events,
        }
        payload.update(kwargs)

        return self._client._request("POST", "/v1/webhooks", json=payload)

    def delete(self, webhook_id: str) -> None:
        """
        Delete a webhook endpoint.

        Args:
            webhook_id: The webhook endpoint ID
        """
        self._client._request("DELETE", f"/v1/webhooks/{webhook_id}")

    def test(self, webhook_id: str) -> Dict[str, Any]:
        """
        Send a test webhook event.

        Args:
            webhook_id: The webhook endpoint ID

        Returns:
            Test result
        """
        return self._client._request("POST", f"/v1/webhooks/{webhook_id}/test")
