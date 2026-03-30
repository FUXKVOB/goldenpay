use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoldenPayError {
    #[error("missing golden key")]
    MissingGoldenKey,
    #[error("unauthorized")]
    Unauthorized,
    #[error("http error: {source}")]
    Http {
        #[from]
        source: reqwest::Error,
    },
    #[error("json error: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },
    #[error("io error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("parse error in {context}: {message}")]
    Parse {
        context: &'static str,
        message: String,
    },
    #[error("request failed: {method} {url} -> {status}: {body}")]
    RequestFailed {
        method: &'static str,
        url: String,
        status: u16,
        body: String,
    },
    #[error("delivery error: {0}")]
    Delivery(#[from] crate::automation::DeliveryError),
    #[error("state store error: {message}")]
    State { message: String },
}

impl GoldenPayError {
    pub fn parse(context: &'static str, message: impl Into<String>) -> Self {
        Self::Parse {
            context,
            message: message.into(),
        }
    }

    pub fn state(message: impl Into<String>) -> Self {
        Self::State {
            message: message.into(),
        }
    }
}
