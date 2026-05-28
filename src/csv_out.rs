//! CSV writer on top of the shared record extraction.
//!
//! We reuse `record::extract_records` to obtain the column union and each
//! row's column-to-value map. Missing fields are written as empty strings.
//! Output uses the default `csv::Writer` dialect (comma separator, `\n`
//! line terminator, RFC 4180 quoting when needed).

use crate::error::Result;
use crate::node::Node;
use crate::record::extract_records;

/// Serialize the parsed document to a CSV string.
pub fn to_csv(
    root_tag: &str,
    root: &Node,
    include_attrs: bool,
    index_as_key: bool,
) -> Result<String> {
    let (cols, rows) = extract_records(root_tag, root, ".", include_attrs, index_as_key);

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::<u8>::with_capacity(256));

    wtr.write_record(&cols)?;
    for row in &rows {
        let record: Vec<&str> = cols
            .iter()
            .map(|c| row.get(c).map(String::as_str).unwrap_or(""))
            .collect();
        wtr.write_record(&record)?;
    }
    let buf = wtr
        .into_inner()
        .map_err(|e| crate::error::FlattenerError::Io(std::io::Error::other(e.to_string())))?;
    Ok(String::from_utf8(buf).expect("csv produces valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, ParserConfig};

    fn p(xml: &str) -> (Box<str>, crate::node::Node) {
        parse(xml, &ParserConfig::default()).unwrap()
    }

    #[test]
    fn single_record() {
        let (tag, node) = p("<x><a>1</a><b>2</b></x>");
        let out = to_csv(&tag, &node, true, false).unwrap();
        assert_eq!(out, "x.a,x.b\n1,2\n");
    }

    #[test]
    fn multi_record() {
        let (tag, node) = p("<xs><x><a>1</a></x><x><a>2</a></x></xs>");
        let out = to_csv(&tag, &node, true, false).unwrap();
        assert_eq!(out, "a\n1\n2\n");
    }

    #[test]
    fn include_attrs_false() {
        let (tag, node) = p(r#"<x a="1"><b>2</b></x>"#);
        let out = to_csv(&tag, &node, false, false).unwrap();
        assert_eq!(out, "x.b\n2\n");
    }

    #[test]
    fn quoting_special_chars() {
        let (tag, node) = p("<x><a>a,b</a></x>");
        let out = to_csv(&tag, &node, true, false).unwrap();
        assert!(out.contains("\"a,b\""));
    }

    #[test]
    fn missing_field_becomes_empty_string() {
        let (tag, node) = p("<xs><x><a>1</a></x><x><b>2</b></x></xs>");
        let out = to_csv(&tag, &node, true, false).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "a,b");
        assert_eq!(lines[1], "1,");
        assert_eq!(lines[2], ",2");
    }

    #[test]
    fn unicode_content() {
        let (tag, node) = p("<x><a>héllo wörld</a></x>");
        let out = to_csv(&tag, &node, true, false).unwrap();
        assert!(out.contains("héllo wörld"));
    }

    #[test]
    fn with_attributes_included() {
        let (tag, node) = p(r#"<x a="1"><b>2</b></x>"#);
        let out = to_csv(&tag, &node, true, false).unwrap();
        assert!(out.contains("@a"));
        assert!(out.contains("1"));
    }

    #[test]
    fn empty_element_in_csv() {
        let (tag, node) = p("<x><a/></x>");
        let out = to_csv(&tag, &node, true, false).unwrap();
        assert!(out.starts_with("x.a\n"));
    }

    #[test]
    fn index_as_key_in_csv_header() {
        let (tag, node) = p("<x><a>1</a><i>2</i><i>3</i></x>");
        let out = to_csv(&tag, &node, true, true).unwrap();
        assert!(out.lines().next().unwrap().contains("x.i.0"));
        assert!(!out.contains("x.i[0]"));
    }
}
