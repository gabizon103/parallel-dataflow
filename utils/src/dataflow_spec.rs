use bril_utils::{BBFunction, BasicBlock};
use std::fmt::Debug;

/// Specifies a dataflow pass to be executed by a DataflowExecutor
pub trait DataflowSpec {
    type Val: Eq + Clone + Debug + Send + Sync + Sized;

    /// Whether this dataflow pass is reversed
    fn reversed(&self) -> bool {
        false
    }

    /// Initial values generated from arguments
    fn entry(&self, func: &BBFunction) -> Self::Val {
        self.init(func)
    }

    /// Initial values for entry blocks
    fn init(&self, func: &BBFunction) -> Self::Val;

    /// Meet function
    fn meet(&self, in_vals: &[Self::Val]) -> Self::Val;

    /// Transfer function
    fn transfer(&self, block: &BasicBlock, in_val: &Self::Val) -> Self::Val;

    /// Transfer function for the exit block
    fn finish(&self, _func: &BBFunction, exit_val: Self::Val) -> Self::Val {
        exit_val
    }
}
