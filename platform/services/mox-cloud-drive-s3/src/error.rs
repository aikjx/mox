//! S3 兼容错误类型 — 严格映射 AWS S3 v20060301 错误码。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum S3Error {
    #[error("AccessDenied")]
    AccessDenied,
    #[error("BucketAlreadyExists")]
    BucketAlreadyExists,
    #[error("BucketNotEmpty")]
    BucketNotEmpty,
    #[error("InvalidArgument")]
    InvalidArgument,
    #[error("KeyTooLongError")]
    KeyTooLongError,
    #[error("NoSuchBucket")]
    NoSuchBucket,
    #[error("NoSuchKey")]
    NoSuchKey,
    #[error("NoSuchUpload")]
    NoSuchUpload,
    #[error("SignatureDoesNotMatch")]
    SignatureDoesNotMatch,
    #[error("NotImplemented: {0}")]
    NotImplemented(String),
    #[error("MethodNotAllowed")]
    MethodNotAllowed,
    #[error("InternalError: {0}")]
    InternalError(String),
    #[error("BadRequest: {0}")]
    BadRequest(String),
}

pub type S3Result<T> = Result<T, S3Error>;

impl S3Error {
    pub fn http_status(&self) -> u16 {
        match self {
            S3Error::AccessDenied => 403,
            S3Error::BucketAlreadyExists => 409,
            S3Error::BucketNotEmpty => 409,
            S3Error::InvalidArgument => 400,
            S3Error::KeyTooLongError => 400,
            S3Error::NoSuchBucket => 404,
            S3Error::NoSuchKey => 404,
            S3Error::NoSuchUpload => 404,
            S3Error::SignatureDoesNotMatch => 403,
            S3Error::NotImplemented(_) => 501,
            S3Error::MethodNotAllowed => 405,
            S3Error::InternalError(_) => 500,
            S3Error::BadRequest(_) => 400,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            S3Error::AccessDenied => "AccessDenied",
            S3Error::BucketAlreadyExists => "BucketAlreadyExists",
            S3Error::BucketNotEmpty => "BucketNotEmpty",
            S3Error::InvalidArgument => "InvalidArgument",
            S3Error::KeyTooLongError => "KeyTooLongError",
            S3Error::NoSuchBucket => "NoSuchBucket",
            S3Error::NoSuchKey => "NoSuchKey",
            S3Error::NoSuchUpload => "NoSuchUpload",
            S3Error::SignatureDoesNotMatch => "SignatureDoesNotMatch",
            S3Error::NotImplemented(_) => "NotImplemented",
            S3Error::MethodNotAllowed => "MethodNotAllowed",
            S3Error::InternalError(_) => "InternalError",
            S3Error::BadRequest(_) => "BadRequest",
        }
    }

    pub fn message(&self) -> String {
        match self {
            S3Error::AccessDenied => "Access Denied".into(),
            S3Error::BucketAlreadyExists => "The requested bucket name is not available.".into(),
            S3Error::BucketNotEmpty => "The bucket you tried to delete is not empty.".into(),
            S3Error::InvalidArgument => "Invalid Argument".into(),
            S3Error::KeyTooLongError => "Your key is too long.".into(),
            S3Error::NoSuchBucket => "The specified bucket does not exist.".into(),
            S3Error::NoSuchKey => "The specified key does not exist.".into(),
            S3Error::NoSuchUpload => "The specified upload does not exist.".into(),
            S3Error::SignatureDoesNotMatch => {
                "The request signature we calculated does not match the signature you provided."
                    .into()
            }
            S3Error::NotImplemented(s) => format!(
                "A header you provided implies functionality that is not implemented: {}",
                s
            ),
            S3Error::MethodNotAllowed => {
                "The specified method is not allowed against this resource.".into()
            }
            S3Error::InternalError(s) => format!("Internal Error: {}", s),
            S3Error::BadRequest(s) => format!("Bad Request: {}", s),
        }
    }

    /// 序列化为 AWS S3 XML 错误响应体。
    pub fn to_xml(&self, request_id: &str) -> String {
        let code = self.code();
        let msg = self.message();
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <Error>\n  \
               <Code>{}</Code>\n  \
               <Message>{}</Message>\n  \
               <RequestId>{}</RequestId>\n\
             </Error>",
            code, msg, request_id
        )
    }
}
