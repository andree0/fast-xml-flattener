//! Parquet writer using `arrow` builders and `parquet::arrow::ArrowWriter`.
//!
//! Schema: every column is `Utf8` (nullable). This keeps the writer simple
//! while still allowing downstream consumers (pandas, duckdb, pyarrow) to
//! cast as needed. Missing values for a given row become NULL.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use arrow::array::{ArrayRef, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;

use crate::error::Result;
use crate::node::Node;
use crate::record::extract_records;

/// Serialize the parsed document as a Parquet file at `path`.
pub fn to_parquet(root_tag: &str, root: &Node, path: &Path, include_attrs: bool) -> Result<()> {
    let (cols, rows) = extract_records(root_tag, root, ".", include_attrs);

    let fields: Vec<Field> = cols
        .iter()
        .map(|c| Field::new(c, DataType::Utf8, true))
        .collect();
    let schema = Arc::new(Schema::new(fields));

    let mut builders: Vec<StringBuilder> = (0..cols.len())
        .map(|_| StringBuilder::with_capacity(rows.len(), rows.len() * 16))
        .collect();

    for row in &rows {
        for (i, col) in cols.iter().enumerate() {
            match row.get(col) {
                Some(v) => builders[i].append_value(v),
                None => builders[i].append_null(),
            }
        }
    }

    let arrays: Vec<ArrayRef> = builders
        .into_iter()
        .map(|mut b| Arc::new(b.finish()) as ArrayRef)
        .collect();

    let batch = RecordBatch::try_new(schema.clone(), arrays)?;

    let file = File::create(path)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn writes_parquet_file() {
        let (tag, node) = parse("<r><a>1</a></r>").unwrap();
        let tmp = tempfile_path("fxf_test_single.parquet");
        to_parquet(&tag, &node, &tmp, true).unwrap();
        assert!(tmp.exists());
        let _ = std::fs::remove_file(&tmp);
    }

    fn tempfile_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(name);
        p
    }

    #[test]
    fn multi_record_parquet() {
        let (tag, node) = parse("<xs><x><a>1</a></x><x><a>2</a></x></xs>").unwrap();
        let tmp = tempfile_path("fxf_test_multi.parquet");
        to_parquet(&tag, &node, &tmp, true).unwrap();
        assert!(tmp.exists());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parquet_exclude_attrs() {
        let (tag, node) = parse(r#"<r><x a="1"><b>2</b></x></r>"#).unwrap();
        let tmp = tempfile_path("fxf_test_noattrs.parquet");
        to_parquet(&tag, &node, &tmp, false).unwrap();
        assert!(tmp.exists());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parquet_with_sparse_rows() {
        let (tag, node) = parse("<xs><x><a>1</a></x><x><b>2</b></x></xs>").unwrap();
        let tmp = tempfile_path("fxf_test_sparse.parquet");
        to_parquet(&tag, &node, &tmp, true).unwrap();
        assert!(tmp.exists());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parquet_invalid_path_errors() {
        let (tag, node) = parse("<r><a>1</a></r>").unwrap();
        let bad_path = std::path::Path::new("/nonexistent_dir_fxf/out.parquet");
        assert!(to_parquet(&tag, &node, bad_path, true).is_err());
    }
}
