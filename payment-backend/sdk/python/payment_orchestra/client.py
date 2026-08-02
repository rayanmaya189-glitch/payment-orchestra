"""
Payment Orchestration Platform Client
"""

from typing import Optional
import requests

from .resources import PaymentIntents, GatewayProfiles, RoutingPolicies, Webhooks
from .exceptions import PaymentOrchestraError, AuthenticationError, RateLimitError


class PaymentOrchestra:
    """
    Main client for interacting with the Payment Orchestra API.

    Example:
        >>> from payment_orchestra import PaymentOrchestra
        >>>
        >>> client = PaymentOrchestra(
        ...     api_key="pk_test_your_key",
        ...     secret_key="sk_test_your_secret",
        ...     environment="sandbox"
        ... )
        >>>
        >>> # Create a payment intent
        >>> payment = client.payment_intents.create(
        ...     amount=10000,
        ...     currency="USD"
        ... )
    """

    def __init__(
        self,
        api_key: str,
        secret_key: str,
        environment: str = "sandbox",
        base_url: Optional[str] = None,
        timeout: int = 30,
        max_retries: int = 3,
    ):
        """
        Initialize the Payment Orchestra client.

        Args:
            api_key: Your API key (pk_test_* or pk_live_*)
            secret_key: Your secret key (sk_test_* or sk_live_*)
            environment: 'sandbox' or 'production'
            base_url: Custom base URL (optional)
            timeout: Request timeout in seconds
            max_retries: Maximum number of retries for failed requests
        """
        self.api_key = api_key
        self.secret_key = secret_key
        self.environment = environment
        self.timeout = timeout
        self.max_retries = max_retries

        if base_url:
            self.base_url = base_url
        elif environment == "production":
            self.base_url = "https://api.paymentorchestra.com"
        else:
            self.base_url = "https://sandbox.api.paymentorchestra.com"

        self._session = requests.Session()
        self._session.headers.update(
            {
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_key}",
                "X-Secret-Key": secret_key,
                "X-SDK-Version": "1.0.0",
                "X-SDK-Name": "python",
            }
        )

        self._payment_intents = PaymentIntents(self)
        self._gateway_profiles = GatewayProfiles(self)
        self._routing_policies = RoutingPolicies(self)
        self._webhooks = Webhooks(self)

    @property
    def payment_intents(self) -> PaymentIntents:
        """Access the Payment Intents resource."""
        return self._payment_intents

    @property
    def gateway_profiles(self) -> GatewayProfiles:
        """Access the Gateway Profiles resource."""
        return self._gateway_profiles

    @property
    def routing_policies(self) -> RoutingPolicies:
        """Access the Routing Policies resource."""
        return self._routing_policies

    @property
    def webhooks(self) -> Webhooks:
        """Access the Webhooks resource."""
        return self._webhooks

    def _request(
        self,
        method: str,
        path: str,
        json: Optional[dict] = None,
        params: Optional[dict] = None,
    ) -> dict:
        """
        Make an API request.

        Args:
            method: HTTP method (GET, POST, PATCH, DELETE)
            path: API endpoint path
            json: Request body (optional)
            params: Query parameters (optional)

        Returns:
            Response JSON as dictionary

        Raises:
            AuthenticationError: If API key is invalid
            RateLimitError: If rate limit is exceeded
            PaymentOrchestraError: For other API errors
        """
        url = f"{self.base_url}{path}"

        for attempt in range(self.max_retries):
            try:
                response = self._session.request(
                    method=method,
                    url=url,
                    json=json,
                    params=params,
                    timeout=self.timeout,
                )

                if response.status_code == 401:
                    raise AuthenticationError("Invalid API key")

                if response.status_code == 429:
                    retry_after = int(response.headers.get("Retry-After", 1))
                    if attempt < self.max_retries - 1:
                        import time
                        time.sleep(retry_after)
                        continue
                    raise RateLimitError("Rate limit exceeded")

                if response.status_code >= 400:
                    error_data = response.json() if response.text else {}
                    message = error_data.get("error", {}).get("message", response.text)
                    raise PaymentOrchestraError(
                        status_code=response.status_code,
                        message=message,
                    )

                return response.json()

            except requests.exceptions.RequestException as e:
                if attempt == self.max_retries - 1:
                    raise PaymentOrchestraError(
                        status_code=0,
                        message=f"Request failed: {str(e)}",
                    )
                continue

        return {}
