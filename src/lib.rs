mod strategies;
pub use strategies::*;

use std::path::Path;

pub use skip_if_macros::skip_if;

pub trait Strategy<O> {
    /// Should we skip generating the `output`?
    /// This can take into account the hash of the arguments and the code, provided as arguments.
    fn skip(&self, output: &Path, args_hash: u64, code_hash: u64) -> bool;
    /// When the output is not skipped, this is called after processing, right before returning.
    fn callback(&self, _fn_output: &O, _output: &Path, _args_hash: u64, _code_hash: u64) {}
}
