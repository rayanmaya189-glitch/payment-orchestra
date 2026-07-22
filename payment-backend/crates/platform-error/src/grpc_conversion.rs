use tonic::{Code, Status};
use crate::{error_types::PlatformError, error_code::InternalErrorCode};

impl From<PlatformError> for Status {
    fn from(err: PlatformError) -> Self {
        let code = err.internal_error_code().to_grpc_code();
        Status::new(code, err.to_string())
    }
}

impl PlatformError {
    pub fn to_grpc_status(&self) -> Status {
        Status::new(self.internal_error_code().to_grpc_code(), self.to_string())
    }
}
