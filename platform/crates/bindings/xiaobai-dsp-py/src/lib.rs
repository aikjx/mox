//! PyO3 最小存根（真实实现会在下一个任务切片填入）。
use pyo3::prelude::*;

#[pymodule]
fn xiaobai_dsp_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
