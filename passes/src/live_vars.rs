use bril_utils::{BBFunction, BasicBlock, InstrExt};
use std::collections::HashSet;
use utils::DataflowSpec;

#[derive(Default)]
pub struct LiveVars;

impl DataflowSpec for LiveVars {
    type Val = HashSet<String>;

    fn reversed(&self) -> bool {
        true
    }

    fn init(&self, _: &BBFunction) -> HashSet<String> {
        HashSet::default()
    }

    fn meet(&self, in_vals: &[HashSet<String>]) -> HashSet<String> {
        // The meet in live vars is set union
        in_vals.iter().flatten().cloned().collect()
    }

    fn transfer(&self, block: &BasicBlock, in_val: &HashSet<String>) -> HashSet<String> {
        let mut out_vals = in_val.clone();

        for insn in block.iter().rev() {
            // Remove the destination from the set
            if let Some(dest) = insn.dest() {
                out_vals.remove(&dest);
            }

            // Add the arguments to the set
            if let Some(args) = insn.args() {
                for arg in args {
                    out_vals.insert(arg);
                }
            }
        }

        out_vals
    }
}
