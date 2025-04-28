use bril_utils::{BBFunction, BasicBlock};
use std::fmt::Debug;

/// Specifies a dataflow pass to be executed by a DataflowExecutor
pub trait DataflowSpec<Val>
where
    Val: Eq + Clone + Debug,
{
    /// Whether this dataflow pass is reversed
    fn reversed(&self) -> bool {
        false
    }

    /// Initial values generated from arguments
    fn entry(&self, func: &BBFunction) -> Val {
        self.init(func)
    }

    /// Initial values for entry blocks
    fn init(&self, func: &BBFunction) -> Val;

    /// Meet function
    fn meet(&self, in_vals: &[Val]) -> Val;

    /// Transfer function
    fn transfer(&self, block: &BasicBlock, in_val: &Val) -> Val;

    /// Transfer function for the exit block
    fn finish(&self, _func: &BBFunction, exit_val: Val) -> Val {
        exit_val
    }
}
