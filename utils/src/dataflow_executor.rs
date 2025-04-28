use crate::DataflowSpec;
use bril_utils::{
    CFG, Dataflow,
    bril_rs::{Program, load_abstract_program_from_read},
};
use std::fmt::Debug;

pub trait DataflowExecutor<Pass, Val>
where
    Pass: DataflowSpec<Val>,
    Val: Eq + Clone + Debug,
{
    fn run(pass: &Pass) -> Vec<Dataflow<Val>> {
        let input = std::io::stdin();

        // Read stdin and parse it into a Program using serde
        let prog: Program = load_abstract_program_from_read(input.lock())
            .try_into()
            .unwrap();

        prog.functions
            .iter()
            .map(|f| Self::cfg(pass, CFG::from(f.clone())))
            .collect()
    }

    fn cfg(pass: &Pass, cfg: CFG) -> Dataflow<Val>;
}
