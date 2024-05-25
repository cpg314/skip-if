mod strategies;
pub use strategies::*;

use std::path::Path;

pub use skip_if_macros::skip_if;

pub trait Strategy<O> {
    fn skip(&self, output: &Path, args_hash: u64, code_hash: u64) -> bool;
    fn callback(
        &self,
        fn_output: &O,
        output: &Path,
        args_hash: u64,
        code_hash: u64,
    ) -> anyhow::Result<()>;
}
