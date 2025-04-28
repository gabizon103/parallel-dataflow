use bril_utils::{BBFunction, BasicBlock, InstrExt};
use sequential::SequentialExecutor;
use std::collections::HashSet;
use utils::{DataflowExecutor, DataflowSpec};

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
struct Definition {
    name: String,
    block: usize,
}

#[derive(Default)]
struct ReachingDefs;

impl DataflowSpec<HashSet<Definition>> for ReachingDefs {
    fn init(&self, func: &BBFunction) -> HashSet<Definition> {
        func.args
            .iter()
            .map(|arg| Definition {
                block: 0,
                name: arg.name.clone(),
            })
            .collect()
    }

    fn meet(&self, in_vals: &[HashSet<Definition>]) -> HashSet<Definition> {
        // The meet in reaching definitions is set union
        in_vals.iter().flatten().cloned().collect()
    }

    fn transfer(&self, block: &BasicBlock, in_val: &HashSet<Definition>) -> HashSet<Definition> {
        // Set of defined names in this block
        let defines: HashSet<_> = block.iter().filter_map(|insn| insn.dest()).collect();

        // Kill all definitions in in_vals that write to this name
        let mut out_vals: HashSet<_> = in_val
            .iter()
            .filter(|def| !defines.contains(&def.name))
            .cloned()
            .collect();

        // Add definitions defined in the block
        out_vals.extend(defines.into_iter().map(|name| Definition {
            name,
            block: block.idx,
        }));

        out_vals
    }
}

fn main() {
    SequentialExecutor::run(&ReachingDefs);
}
