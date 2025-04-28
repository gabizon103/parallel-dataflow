use crate::DataflowSpec;
use bril_utils::{
    CFG, Dataflow,
    bril_rs::{Program, load_abstract_program_from_read},
};
use itertools::Itertools;
use std::fmt::Debug;
use std::time::{Duration, Instant};

pub struct DataflowResults {
    pub result: String,
    pub loadtime: Duration,
    pub runtime: Duration,
    pub writetime: Duration,
}

pub trait DataflowExecutor<Pass, Val>
where
    Pass: DataflowSpec<Val>,
    Val: Eq + Clone + Debug,
{
    /// Run the dataflow pass on the input program and perform performance measurements
    fn run(pass: &Pass) -> DataflowResults {
        let start = Instant::now();
        let input = std::io::stdin();

        // Read stdin and parse it into a Program using serde
        let prog: Program = load_abstract_program_from_read(input.lock())
            .try_into()
            .unwrap();

        let loadtime = start.elapsed();
        let start = Instant::now();

        let results = prog
            .functions
            .iter()
            .map(|f| Self::cfg(pass, CFG::from(f.clone())))
            .collect_vec();

        let runtime = start.elapsed();
        let start = Instant::now();

        let results = results.into_iter().map(|x| format!("{:?}", x)).join("\n\n");

        let writetime = start.elapsed();

        DataflowResults {
            result: results,
            loadtime,
            runtime,
            writetime,
        }
    }

    fn cfg(pass: &Pass, cfg: CFG) -> Dataflow<Val>;
}
