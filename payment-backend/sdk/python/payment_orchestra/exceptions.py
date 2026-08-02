"""
Exception classes for the Payment Orchestra SDK.
"""


class PaymentOrchestraError(Exception):
    """Base exception for Payment Orchestra API errors."""

    def __init__(self, status_code: int, message: str, details: dict = None):
        self.status_code = status_code
        self.message = message
        self.details = details or {}
        super().__init__(self.message)

    def __str__(self) -> str:
        if self.status_code:
            return f"[{self.status_code}] {self.message}"
        return self.message


class AuthenticationError(PaymentOrchestraError):
    """Raised when API key is invalid or missing."""

    def __init__(self, message: str = "Authentication failed"):
        super().__init__(status_code=401, message=message)


class RateLimitError(PaymentOrchestraError):
    """Raised when rate limit is exceeded."""

    def __init__(self, message: str = "Rate limit exceeded"):
        super().__init__(status_code=429, message=message)


class ValidationError(PaymentOrchestraError):
    """Raised when request validation fails."""

    def __init__(self, message: str = "Validation error", details: dict = None):
        super().__init__(status_code=422, message=message, details=details)


class NotFoundError(PaymentOrchestraError):
    """Raised when requested resource is not found."""

    def __init__(self, message: str = "Resource not found"):
        super().__init__(status_code=404, message=message)


class APIError(PaymentOrchestraError):
    """Raised for general API errors."""

    def __init__(self, status_code: int, message: str, details: dict = None):
        super().__init__(status_code=status_code, message=message, details=details)
