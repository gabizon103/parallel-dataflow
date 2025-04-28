use std::str::FromStr;

use argh::FromArgs;
use passes::ReachingDefs;
use sequential::{ParallelExecutor, SequentialExecutor};
use simple_logger::SimpleLogger;
use utils::DataflowExecutor;

enum Executor {
    /// Basic sequential worklist algorithm
    Sequential,
    /// Parallel worklist algorithm
    Parallel,
}

impl FromStr for Executor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sequential" | "s" => Ok(Executor::Sequential),
            "parallel" | "p" => Ok(Executor::Parallel),
            _ => Err(format!("Unknown executor: {}", s)),
        }
    }
}

enum Pass {
    /// Reaching definitions
    ReachingDefinitions,
}

impl FromStr for Pass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "reaching-defs" | "rd" | "reaching-definitions" => Ok(Pass::ReachingDefinitions),
            _ => Err(format!("Unknown executor: {}", s)),
        }
    }
}

#[derive(FromArgs)]
/// Logger arguments
pub struct Args {
    /// the log level
    #[argh(option, short = 'l', default = "log::LevelFilter::Warn")]
    log: log::LevelFilter,
    /// the executor to use
    #[argh(option, short = 'a', default = "Executor::Sequential")]
    algorithm: Executor,
    /// the pass to run
    #[argh(option, short = 'p')]
    pass: Pass,
}

macro_rules! run {
    ($executor: expr, $pass: ident) => {
        match $executor {
            Executor::Sequential => SequentialExecutor::run(&$pass),
            Executor::Parallel => ParallelExecutor::run(&$pass),
        }
    };
}

fn main() {
    let args: Args = argh::from_env();

    SimpleLogger::new()
        .with_colors(true)
        .with_level(args.log)
        .without_timestamps()
        .init()
        .unwrap();

    let result = match args.pass {
        Pass::ReachingDefinitions => run!(args.algorithm, ReachingDefs),
    };

    println!("{}", result)
}
