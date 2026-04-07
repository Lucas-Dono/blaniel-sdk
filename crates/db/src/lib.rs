pub mod models;
pub mod pool;
pub mod queries;

pub use pool::create_pool;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

pub type DbResult<T> = Result<T, DbError>;
