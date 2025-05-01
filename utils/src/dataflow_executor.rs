use crate::DataflowSpec;
use bril_utils::{CFG, CanonicalizeLiterals, Dataflow, Pass, bril_rs::Program};
use bril2json::parse_abstract_program_from_read;
use itertools::Itertools;
use std::time::{Duration, Instant};

pub struct PassTiming {
    pub loadtime: Duration,
    pub runtime: Duration,
}

pub trait DataflowExecutor<Pass>
where
    Pass: DataflowSpec,
{
    /// Run the dataflow pass on the input program and perform performance measurements
    fn run<R: std::io::Read>(pass: &Pass, input: R) -> (PassTiming, Vec<Dataflow<Pass::Val>>) {
        let start = Instant::now();

        // Read stdin and parse it into a Program using serde
        let prog: Program = parse_abstract_program_from_read(input, false, false, None)
            .try_into()
            .unwrap();

        // Perform CanonicalizeLiterals always just to make sure things are canonical
        let prog = CanonicalizeLiterals.run(prog);

        let loadtime = start.elapsed();
        let start = Instant::now();

        let results = prog
            .functions
            .iter()
            .map(|f| Self::cfg(pass, CFG::from(f.clone())))
            .collect_vec();

        let runtime = start.elapsed();

        (PassTiming { loadtime, runtime }, results)
    }

    fn cfg(pass: &Pass, cfg: CFG) -> Dataflow<Pass::Val>;
}
