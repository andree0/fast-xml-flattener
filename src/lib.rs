use pyo3::prelude::*;

#[pyfunction]
fn flatten_xml(xml: &str) -> PyResult<String> {
    Ok(format!("Flattened: {}", xml))
}

#[pymodule]
fn fast_xml_flattener(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(flatten_xml, m)?)?;
    Ok(())
}
