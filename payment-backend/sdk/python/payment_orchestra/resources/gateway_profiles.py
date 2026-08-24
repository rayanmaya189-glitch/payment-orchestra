"""
Gateway Profiles resource for managing payment gateway connections.
"""

from typing import Optional, Dict, Any, List


class GatewayProfiles:
    """
    Manage gateway profiles.

    Example:
        >>> # List gateway profiles
        >>> gateways = client.gateway_profiles.list()
        >>>
        >>> # Create a gateway profile
        >>> gateway = client.gateway_profiles.create(
        ...     connector_id="stripe",
        ...     display_name="My Stripe Account",
        ...     environment="sandbox",
        ...     credentials={"api_key": "sk_test_..."}
        ... )
    """

    def __init__(self, client):
        self._client = client

    def list(self) -> List[Dict[str, Any]]:
        """
        List all gateway profiles.

        Returns:
            List of gateway profile objects
        """
        return self._client._request("GET", "/v1/gateway-profiles")

    def get(self, gateway_id: str) -> Dict[str, Any]:
        """
        Retrieve a gateway profile by ID.

        Args:
            gateway_id: The gateway profile ID

        Returns:
            Gateway profile object
        """
        return self._client._request("GET", f"/v1/gateway-profiles/{gateway_id}")

    def create(
        self,
        connector_id: str,
        display_name: str,
        environment: str,
        credentials: Dict[str, str],
        **kwargs,
    ) -> Dict[str, Any]:
        """
        Create a new gateway profile.

        Args:
            connector_id: Connector identifier (e.g., 'stripe', 'checkout_com')
            display_name: Human-readable name
            environment: 'sandbox' or 'production'
            credentials: Dictionary of credential key-value pairs
            **kwargs: Additional parameters

        Returns:
            Created gateway profile object
        """
        payload = {
            "connector_id": connector_id,
            "display_name": display_name,
            "environment": environment,
            "credentials": credentials,
        }
        payload.update(kwargs)

        return self._client._request("POST", "/v1/gateway-profiles", json=payload)

    def update(self, gateway_id: str, **kwargs) -> Dict[str, Any]:
        """
        Update a gateway profile.

        Args:
            gateway_id: The gateway profile ID
            **kwargs: Fields to update

        Returns:
            Updated gateway profile object
        """
        return self._client._request(
            "PATCH", f"/v1/gateway-profiles/{gateway_id}", json=kwargs
        )

    def delete(self, gateway_id: str) -> None:
        """
        Delete a gateway profile.

        Args:
            gateway_id: The gateway profile ID
        """
        self._client._request("DELETE", f"/v1/gateway-profiles/{gateway_id}")

    def test(self, gateway_id: str) -> Dict[str, Any]:
        """
        Test a gateway connection.

        Args:
            gateway_id: The gateway profile ID

        Returns:
            Test result with 'success' and 'message' keys
        """
        return self._client._request(
            "POST", f"/v1/gateway-profiles/{gateway_id}/test"
        )
