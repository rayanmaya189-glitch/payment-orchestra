"""
Payment Orchestra Python SDK Client
"""

from typing import Optional, Dict, Any, List
from urllib.parse import urljoin
import requests

from .exceptions import PaymentOrchestraError


class PaymentOrchestra:
    """Payment Orchestra API client."""

    def __init__(
        self,
        api_key: str,
        environment: str = "sandbox",
        base_url: Optional[str] = None,
        timeout: int = 30,
        max_retries: int = 3,
    ):
        """
        Initialize the Payment Orchestra client.

        Args:
            api_key: Your API key (pk_live_... or pk_test_...)
            environment: 'production' or 'sandbox'
            base_url: Custom base URL (overrides environment)
            timeout: Request timeout in seconds
            max_retries: Maximum number of retries for failed requests
        """
        self.api_key = api_key
        self.timeout = timeout
        self.max_retries = max_retries

        if base_url:
            self.base_url = base_url.rstrip("/")
        elif environment == "sandbox":
            self.base_url = "https://sandbox.payment-orchestra.com"
        else:
            self.base_url = "https://api.payment-orchestra.com"

        self.session = requests.Session()
        self.session.headers.update({
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "X-API-Version": "1",
            "User-Agent": "PaymentOrchestra-SDK-Python/0.1.0",
        })

    @property
    def payment_intents(self) -> "PaymentIntentsResource":
        """Access payment intents API."""
        return PaymentIntentsResource(self)

    @property
    def gateway_profiles(self) -> "GatewayProfilesResource":
        """Access gateway profiles API."""
        return GatewayProfilesResource(self)

    @property
    def routing_policies(self) -> "RoutingPoliciesResource":
        """Access routing policies API."""
        return RoutingPoliciesResource(self)

    @property
    def webhooks(self) -> "WebhooksResource":
        """Access webhooks API."""
        return WebhooksResource(self)

    @property
    def api_keys(self) -> "ApiKeysResource":
        """Access API keys API."""
        return ApiKeysResource(self)

    def _request(
        self,
        method: str,
        path: str,
        data: Optional[Dict] = None,
        params: Optional[Dict] = None,
    ) -> Dict[str, Any]:
        """Make an API request."""
        url = f"{self.base_url}{path}"

        for attempt in range(self.max_retries):
            try:
                response = self.session.request(
                    method=method,
                    url=url,
                    json=data,
                    params=params,
                    timeout=self.timeout,
                )

                if response.status_code >= 400:
                    error_data = response.json() if response.content else {}
                    raise PaymentOrchestraError(
                        message=error_data.get("detail", f"HTTP {response.status_code}"),
                        status_code=response.status_code,
                        response=error_data,
                    )

                if response.status_code == 204:
                    return {}

                return response.json()

            except requests.exceptions.RequestException as e:
                if attempt == self.max_retries - 1:
                    raise PaymentOrchestraError(
                        message=f"Request failed: {str(e)}",
                        status_code=0,
                    )
                continue

        return {}

    def _get(self, path: str, params: Optional[Dict] = None) -> Dict[str, Any]:
        return self._request("GET", path, params=params)

    def _post(self, path: str, data: Optional[Dict] = None) -> Dict[str, Any]:
        return self._request("POST", path, data=data)

    def _delete(self, path: str) -> Dict[str, Any]:
        return self._request("DELETE", path)


class PaymentIntentsResource:
    """Payment intents API resource."""

    def __init__(self, client: PaymentOrchestra):
        self._client = client

    def create(
        self,
        amount: int,
        currency: str,
        purpose: str = "payment",
        idempotency_key: Optional[str] = None,
        metadata: Optional[Dict] = None,
    ) -> Dict[str, Any]:
        """Create a payment intent."""
        data = {
            "amount_minor_units": amount,
            "currency": currency,
            "purpose": purpose,
        }
        if idempotency_key:
            data["idempotency_key"] = idempotency_key
        if metadata:
            data["metadata"] = metadata
        return self._client._post("/v1/payment-intents", data)

    def get(self, intent_id: str) -> Dict[str, Any]:
        """Get a payment intent."""
        return self._client._get(f"/v1/payment-intents/{intent_id}")

    def list(
        self,
        limit: int = 20,
        offset: int = 0,
        status: Optional[str] = None,
    ) -> Dict[str, Any]:
        """List payment intents."""
        params = {"limit": limit, "offset": offset}
        if status:
            params["status"] = status
        return self._client._get("/v1/payment-intents", params=params)

    def authorize(
        self,
        intent_id: str,
        payment_method_token: str,
        card_scheme: str,
    ) -> Dict[str, Any]:
        """Authorize a payment intent."""
        return self._client._post(
            f"/v1/payment-intents/{intent_id}/authorize",
            {
                "payment_method_token": payment_method_token,
                "card_scheme": card_scheme,
            },
        )

    def capture(
        self,
        intent_id: str,
        amount: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Capture a payment intent."""
        data = {}
        if amount is not None:
            data["amount_minor_units"] = amount
        return self._client._post(f"/v1/payment-intents/{intent_id}/capture", data)

    def refund(
        self,
        intent_id: str,
        amount: int,
        reason: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Refund a payment intent."""
        data = {"amount_minor_units": amount}
        if reason:
            data["reason"] = reason
        return self._client._post(f"/v1/payment-intents/{intent_id}/refund", data)

    def void(self, intent_id: str) -> Dict[str, Any]:
        """Void a payment intent."""
        return self._client._post(f"/v1/payment-intents/{intent_id}/void", {})


class GatewayProfilesResource:
    """Gateway profiles API resource."""

    def __init__(self, client: PaymentOrchestra):
        self._client = client

    def create(self, **kwargs) -> Dict[str, Any]:
        return self._client._post("/v1/gateway-profiles", kwargs)

    def get(self, profile_id: str) -> Dict[str, Any]:
        return self._client._get(f"/v1/gateway-profiles/{profile_id}")

    def list(self) -> list:
        return self._client._get("/v1/gateway-profiles")

    def test(self, profile_id: str) -> Dict[str, Any]:
        return self._client._post(f"/v1/gateway-profiles/{profile_id}/test", {})


class RoutingPoliciesResource:
    """Routing policies API resource."""

    def __init__(self, client: PaymentOrchestra):
        self._client = client

    def create(self, **kwargs) -> Dict[str, Any]:
        return self._client._post("/v1/routing-policies", kwargs)

    def get(self, policy_id: str) -> Dict[str, Any]:
        return self._client._get(f"/v1/routing-policies/{policy_id}")

    def list(self) -> list:
        return self._client._get("/v1/routing-policies")

    def activate(self, policy_id: str) -> Dict[str, Any]:
        return self._client._post(f"/v1/routing-policies/{policy_id}/activate", {})

    def deactivate(self, policy_id: str) -> Dict[str, Any]:
        return self._client._post(f"/v1/routing-policies/{policy_id}/deactivate", {})


class WebhooksResource:
    """Webhooks API resource."""

    def __init__(self, client: PaymentOrchestra):
        self._client = client

    def create(self, url: str, events: List[str]) -> Dict[str, Any]:
        return self._client._post("/v1/webhooks", {"url": url, "events": events})

    def list(self) -> list:
        return self._client._get("/v1/webhooks")

    def delete(self, webhook_id: str) -> None:
        self._client._delete(f"/v1/webhooks/{webhook_id}")


class ApiKeysResource:
    """API keys API resource."""

    def __init__(self, client: PaymentOrchestra):
        self._client = client

    def create(self, name: str, scopes: List[str]) -> Dict[str, Any]:
        return self._client._post("/v1/api-keys", {"name": name, "scopes": scopes})

    def list(self) -> list:
        return self._client._get("/v1/api-keys")

    def revoke(self, key_id: str) -> Dict[str, Any]:
        return self._client._post(f"/v1/api-keys/{key_id}/revoke", {})

    def rotate(self, key_id: str) -> Dict[str, Any]:
        return self._client._post(f"/v1/api-keys/{key_id}/rotate", {})
