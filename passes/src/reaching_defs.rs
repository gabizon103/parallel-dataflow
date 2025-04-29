use bril_utils::{BBFunction, BasicBlock, InstrExt};
use std::{collections::HashSet, fmt::Debug, hash::Hash};
use utils::DataflowSpec;

#[derive(Clone, Eq, Hash, PartialEq, PartialOrd)]
pub struct ReachingDefinition {
    name: String,
    block: usize,
}

impl Debug for ReachingDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ {}", self.name, self.block)
    }
}

#[derive(Default)]
pub struct ReachingDefs;

impl DataflowSpec for ReachingDefs {
    type Val = HashSet<ReachingDefinition>;

    fn init(&self, func: &BBFunction) -> HashSet<ReachingDefinition> {
        func.args
            .iter()
            .map(|arg| ReachingDefinition {
                block: 0,
                name: arg.name.clone(),
            })
            .collect()
    }

    fn meet(&self, in_vals: &[HashSet<ReachingDefinition>]) -> HashSet<ReachingDefinition> {
        // The meet in reaching definitions is set union
        in_vals.iter().flatten().cloned().collect()
    }

    fn transfer(
        &self,
        block: &BasicBlock,
        in_val: &HashSet<ReachingDefinition>,
    ) -> HashSet<ReachingDefinition> {
        // Set of defined names in this block
        let defines: HashSet<_> = block.iter().filter_map(|insn| insn.dest()).collect();

        // Kill all ReachingDefinitions in in_vals that write to this name
        let mut out_vals: HashSet<_> = in_val
            .iter()
            .filter(|def| !defines.contains(&def.name))
            .cloned()
            .collect();

        // Add ReachingDefinitions defined in the block
        out_vals.extend(defines.into_iter().map(|name| ReachingDefinition {
            name,
            block: block.idx,
        }));

        out_vals
    }
}
