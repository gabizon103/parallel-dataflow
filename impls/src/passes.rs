use crate::{ParallelExecutor, SequentialExecutor};
use passes::{ConstProp, LiveVars, ReachingDefs};
use serde::Serialize;
use strum::{Display, EnumIter, EnumString};
use utils::{DataflowExecutor, PassTiming};

#[derive(EnumString, EnumIter, Debug, Display, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Executor {
    /// Basic sequential worklist algorithm
    #[strum(serialize = "s", serialize = "sequential")]
    Sequential,
    /// Parallel worklist algorithm
    #[strum(serialize = "p", serialize = "parallel")]
    Parallel,
}

#[derive(EnumString, EnumIter, Debug, Display, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    /// Reaching definitions
    #[strum(
        serialize = "rd",
        serialize = "reaching-definitions",
        serialize = "reaching-defs"
    )]
    ReachingDefinitions,
    #[strum(
        serialize = "lv",
        serialize = "live-vars",
        serialize = "live-variables"
    )]
    LiveVariables,
    #[strum(serialize = "const-prop", serialize = "const-propagation")]
    ConstProp,
}

macro_rules! run {
    ($executor: expr, $pass: ident, $input: ident) => {{
        let (timings, data) = match $executor {
            Executor::Sequential => SequentialExecutor::run(&$pass, $input),
            Executor::Parallel => ParallelExecutor::run(&$pass, $input),
        };

        let result = data
            .into_iter()
            .map(|d| format!("{:?}", d))
            .collect::<Vec<_>>()
            .join("\n");
        (timings, result)
    }};
}

impl Pass {
    pub fn execute<R: std::io::Read>(&self, executor: &Executor, input: R) -> (PassTiming, String) {
        match self {
            Pass::ReachingDefinitions => run!(executor, ReachingDefs, input),
            Pass::LiveVariables => run!(executor, LiveVars, input),
            Pass::ConstProp => run!(executor, ConstProp, input),
        }
    }
}
