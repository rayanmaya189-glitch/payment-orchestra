"""
Payment Orchestra Python SDK Exceptions
"""


class PaymentOrchestraError(Exception):
    """Base exception for Payment Orchestra SDK."""

    def __init__(
        self,
        message: str,
        status_code: int = 0,
        response: dict = None,
    ):
        super().__init__(message)
        self.message = message
        self.status_code = status_code
        self.response = response or {}

    def __str__(self) -> str:
        if self.status_code:
            return f"[{self.status_code}] {self.message}"
        return self.message


class AuthenticationError(PaymentOrchestraError):
    """Raised when authentication fails (401)."""
    pass


class AuthorizationError(PaymentOrchestraError):
    """Raised when authorization fails (403)."""
    pass


class NotFoundError(PaymentOrchestraError):
    """Raised when a resource is not found (404)."""
    pass


class ValidationError(PaymentOrchestraError):
    """Raised when validation fails (400)."""
    pass


class RateLimitError(PaymentOrchestraError):
    """Raised when rate limit is exceeded (429)."""
    pass


class IdempotencyError(PaymentOrchestraError):
    """Raised when idempotency key conflicts (409)."""
    pass
