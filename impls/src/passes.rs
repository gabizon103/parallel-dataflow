use crate::{ParallelExecutor, SequentialExecutor};
use passes::ReachingDefs;
use strum::{Display, EnumIter, EnumString};
use utils::{DataflowExecutor, DataflowResults};

#[derive(EnumString, EnumIter, Display)]
pub enum Executor {
    /// Basic sequential worklist algorithm
    #[strum(serialize = "s", serialize = "sequential")]
    Sequential,
    /// Parallel worklist algorithm
    #[strum(serialize = "p", serialize = "parallel")]
    Parallel,
}

#[derive(EnumString, EnumIter, Display)]
pub enum Pass {
    /// Reaching definitions
    #[strum(
        serialize = "rd",
        serialize = "reaching-definitions",
        serialize = "reaching-defs"
    )]
    ReachingDefinitions,
}

macro_rules! run {
    ($executor: expr, $pass: ident, $input: ident) => {
        match $executor {
            Executor::Sequential => SequentialExecutor::run(&$pass, $input),
            Executor::Parallel => ParallelExecutor::run(&$pass, $input),
        }
    };
}

impl Pass {
    pub fn execute<R: std::io::Read>(&self, executor: &Executor, input: R) -> DataflowResults {
        match self {
            Pass::ReachingDefinitions => run!(executor, ReachingDefs, input),
        }
    }
}
