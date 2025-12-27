"""
minimemory SDK Exceptions
"""

from __future__ import annotations


class MiniMemoryError(Exception):
    """Base exception for minimemory SDK errors."""

    def __init__(
        self,
        message: str,
        status: int | None = None,
        code: str | None = None,
    ) -> None:
        super().__init__(message)
        self.message = message
        self.status = status
        self.code = code

    def __str__(self) -> str:
        parts = [self.message]
        if self.status:
            parts.append(f"(status={self.status})")
        if self.code:
            parts.append(f"(code={self.code})")
        return " ".join(parts)


class AuthenticationError(MiniMemoryError):
    """Raised when authentication fails."""

    def __init__(self, message: str = "Authentication failed") -> None:
        super().__init__(message, status=401, code="AUTH_ERROR")


class NotFoundError(MiniMemoryError):
    """Raised when a resource is not found."""

    def __init__(self, message: str = "Resource not found") -> None:
        super().__init__(message, status=404, code="NOT_FOUND")


class RateLimitError(MiniMemoryError):
    """Raised when rate limit is exceeded."""

    def __init__(
        self,
        message: str = "Rate limit exceeded",
        retry_after: int | None = None,
    ) -> None:
        super().__init__(message, status=429, code="RATE_LIMIT")
        self.retry_after = retry_after


class ValidationError(MiniMemoryError):
    """Raised when request validation fails."""

    def __init__(self, message: str) -> None:
        super().__init__(message, status=400, code="VALIDATION_ERROR")


class TimeoutError(MiniMemoryError):
    """Raised when a request times out."""

    def __init__(self, message: str = "Request timed out") -> None:
        super().__init__(message, code="TIMEOUT")


class NetworkError(MiniMemoryError):
    """Raised when a network error occurs."""

    def __init__(self, message: str = "Network error") -> None:
        super().__init__(message, code="NETWORK_ERROR")
