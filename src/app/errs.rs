use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("the object `{0}` is unknown")]
    NoSuchObject(String),

    #[error("internal error: the operation did not get the proper arguments")]
    BadOperationContext,

    #[error("object without id")]
    PartHasNoId,

    #[error("IO error '{0}'")]
    IoError(io::Error),

    #[error("yaml serialization error '{0}'")]
    ObjectSerializationError(serde_yaml::Error),

    #[error("ledger serialization error '{0}'")]
    LedgerSerializationError(crate::store::serializer::LedgerError),
}
