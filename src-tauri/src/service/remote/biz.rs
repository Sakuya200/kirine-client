use anyhow::bail;

use crate::Result;

pub(crate) fn unsupported<T>(operation: &str) -> Result<T> {
    bail!("远端存储暂未实现 {} 接口", operation)
}
