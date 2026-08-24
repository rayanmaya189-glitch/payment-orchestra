"""
Routing Policies resource for managing payment routing rules.
"""

from typing import Optional, Dict, Any, List


class RoutingPolicies:
    """
    Manage routing policies.

    Example:
        >>> # Create a routing policy
        >>> policy = client.routing_policies.create(
        ...     name="US Card Routing",
        ...     rules=[...],
        ...     rotation_strategy="success_rate_based"
        ... )
        >>>
        >>> # Activate a policy
        >>> client.routing_policies.activate("rp_abc123")
    """

    def __init__(self, client):
        self._client = client

    def list(self) -> List[Dict[str, Any]]:
        """
        List all routing policies.

        Returns:
            List of routing policy objects
        """
        return self._client._request("GET", "/v1/routing-policies")

    def get(self, policy_id: str) -> Dict[str, Any]:
        """
        Retrieve a routing policy by ID.

        Args:
            policy_id: The routing policy ID

        Returns:
            Routing policy object
        """
        return self._client._request("GET", f"/v1/routing-policies/{policy_id}")

    def create(
        self,
        name: str,
        rules: List[Dict[str, Any]],
        rotation_strategy: str,
        failover_config: Optional[Dict[str, Any]] = None,
        **kwargs,
    ) -> Dict[str, Any]:
        """
        Create a new routing policy.

        Args:
            name: Policy name
            rules: List of routing rules
            rotation_strategy: Strategy type ('priority', 'round_robin', 'success_rate_based', etc.)
            failover_config: Optional failover configuration
            **kwargs: Additional parameters

        Returns:
            Created routing policy object
        """
        payload = {
            "name": name,
            "rules": rules,
            "rotation_strategy": rotation_strategy,
        }
        if failover_config:
            payload["failover_config"] = failover_config
        payload.update(kwargs)

        return self._client._request("POST", "/v1/routing-policies", json=payload)

    def activate(self, policy_id: str) -> Dict[str, Any]:
        """
        Activate a routing policy.

        Args:
            policy_id: The routing policy ID

        Returns:
            Updated routing policy object
        """
        return self._client._request(
            "POST", f"/v1/routing-policies/{policy_id}/activate"
        )

    def deactivate(self, policy_id: str) -> Dict[str, Any]:
        """
        Deactivate a routing policy.

        Args:
            policy_id: The routing policy ID

        Returns:
            Updated routing policy object
        """
        return self._client._request(
            "POST", f"/v1/routing-policies/{policy_id}/deactivate"
        )
