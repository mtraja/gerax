use gerax_http::HttpServerError;

use crate::traits::AuthError;

impl From<AuthError> for HttpServerError {
    fn from(err: AuthError) -> Self {
        HttpServerError::HandlerError(err.to_string())
    }
}
