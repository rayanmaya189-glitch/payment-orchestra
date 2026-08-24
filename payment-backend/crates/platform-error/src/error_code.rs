use tonic::Code;

/// Unified internal error codes for service-to-service communication.
/// Maps to gRPC status codes but provides finer granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InternalErrorCode {
    TransientFailure,
    PermanentFailure,
    DegradedMode,
    RateLimited,
    AuthorizationDenied,
    ValidationError,
    Unavailable,
}

impl InternalErrorCode {
    pub fn to_grpc_code(&self) -> Code {
        match self {
            Self::TransientFailure => Code::Unavailable,
            Self::PermanentFailure => Code::Internal,
            Self::DegradedMode => Code::Unavailable,
            Self::RateLimited => Code::ResourceExhausted,
            Self::AuthorizationDenied => Code::PermissionDenied,
            Self::ValidationError => Code::InvalidArgument,
            Self::Unavailable => Code::Unavailable,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::TransientFailure | Self::Unavailable | Self::RateLimited)
    }
}

impl std::fmt::Display for InternalErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
