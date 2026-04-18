//! Error types for the flattener. All variants convert to `PyValueError` or
//! `PyIOError` when crossing the PyO3 boundary.

use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::PyErr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlattenerError {
    #[error("XML parse error: {0}")]
    Xml(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parquet error: {0}")]
    Parquet(String),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Invalid input: {0}")]
    Invalid(String),
}

impl From<quick_xml::Error> for FlattenerError {
    fn from(err: quick_xml::Error) -> Self {
        FlattenerError::Xml(err.to_string())
    }
}

impl From<quick_xml::events::attributes::AttrError> for FlattenerError {
    fn from(err: quick_xml::events::attributes::AttrError) -> Self {
        FlattenerError::Xml(err.to_string())
    }
}

impl From<quick_xml::encoding::EncodingError> for FlattenerError {
    fn from(err: quick_xml::encoding::EncodingError) -> Self {
        FlattenerError::Xml(err.to_string())
    }
}

impl From<quick_xml::escape::EscapeError> for FlattenerError {
    fn from(err: quick_xml::escape::EscapeError) -> Self {
        FlattenerError::Xml(err.to_string())
    }
}

impl From<csv::Error> for FlattenerError {
    fn from(err: csv::Error) -> Self {
        FlattenerError::Io(std::io::Error::other(err.to_string()))
    }
}

impl From<parquet::errors::ParquetError> for FlattenerError {
    fn from(err: parquet::errors::ParquetError) -> Self {
        FlattenerError::Parquet(err.to_string())
    }
}

impl From<arrow::error::ArrowError> for FlattenerError {
    fn from(err: arrow::error::ArrowError) -> Self {
        FlattenerError::Parquet(err.to_string())
    }
}

impl From<FlattenerError> for PyErr {
    fn from(err: FlattenerError) -> PyErr {
        match err {
            FlattenerError::Io(_) => PyIOError::new_err(err.to_string()),
            _ => PyValueError::new_err(err.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, FlattenerError>;
