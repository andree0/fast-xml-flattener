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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_variant_display() {
        let e = FlattenerError::Xml("bad tag".to_string());
        assert!(e.to_string().contains("bad tag"));
    }

    #[test]
    fn invalid_variant_display() {
        let e = FlattenerError::Invalid("missing root".to_string());
        assert!(e.to_string().contains("missing root"));
    }

    #[test]
    fn parquet_variant_display() {
        let e = FlattenerError::Parquet("schema mismatch".to_string());
        assert!(e.to_string().contains("schema mismatch"));
    }

    #[test]
    fn io_variant_display() {
        let e = FlattenerError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "no file"));
        assert!(e.to_string().contains("I/O"));
    }

    #[test]
    fn utf8_variant_display() {
        let utf8_err = std::str::from_utf8(b"\xff").unwrap_err();
        let e = FlattenerError::Utf8(utf8_err);
        assert!(e.to_string().contains("UTF-8"));
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let e: FlattenerError = io_err.into();
        assert!(matches!(e, FlattenerError::Io(_)));
    }

    #[test]
    fn from_utf8_error() {
        let utf8_err = std::str::from_utf8(b"\xff").unwrap_err();
        let e: FlattenerError = utf8_err.into();
        assert!(matches!(e, FlattenerError::Utf8(_)));
    }

    #[test]
    fn from_parquet_error() {
        let pe = parquet::errors::ParquetError::General("test parquet".to_string());
        let e: FlattenerError = pe.into();
        assert!(matches!(e, FlattenerError::Parquet(_)));
        assert!(e.to_string().contains("test parquet"));
    }

    #[test]
    fn from_arrow_error() {
        let ae = arrow::error::ArrowError::NotYetImplemented("test arrow".to_string());
        let e: FlattenerError = ae.into();
        assert!(matches!(e, FlattenerError::Parquet(_)));
    }

    #[test]
    fn from_csv_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe");
        let csv_err = csv::Error::from(io_err);
        let e: FlattenerError = csv_err.into();
        assert!(matches!(e, FlattenerError::Io(_)));
    }

    #[test]
    fn from_quick_xml_error_gives_xml_variant() {
        let result = crate::parser::parse("<a><b></a>");
        let err = result.unwrap_err();
        assert!(matches!(err, FlattenerError::Xml(_)));
    }

    #[test]
    fn from_quick_xml_escape_error_gives_xml_variant() {
        let result = crate::parser::parse("<r>&unknown_entity;</r>");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, FlattenerError::Xml(_)));
        }
    }
}
