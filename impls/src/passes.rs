use passes::{AvailableExpr, ConstProp, LiveVars, ReachingDefs};
use regex::Regex;
use serde::Serialize;
use std::{fmt::Display, str::FromStr};
use strum::{Display, EnumIter, EnumString};
use utils::{DataflowExecutor, PassTiming};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Executor {
    /// Basic sequential worklist algorithm
    Sequential,
    /// Parallel worklist algorithm
    Parallel,
    /// Mixed worklist algorithm
    Mixed(usize),
}

impl FromStr for Executor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Simple pattern matching
        let simple = match s {
            "sequential" | "seq" => Some(Executor::Sequential),
            "parallel" | "par" => Some(Executor::Parallel),
            _ => None,
        };

        if let Some(executor) = simple {
            Ok(executor)
        } else {
            // Parse strings with arguments
            let re = Regex::new(r"^mixed-(\d+)$").unwrap();
            if let Some(caps) = re.captures(s) {
                let thresh = caps[1].parse().unwrap();
                Ok(Executor::Mixed(thresh))
            } else {
                Err(format!("Unknown executor {}", s))
            }
        }
    }
}

impl Display for Executor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Executor::Sequential => "sequential".fmt(f),
            Executor::Parallel => "parallel".fmt(f),
            Executor::Mixed(thresh) => write!(f, "mixed-{thresh}"),
        }
    }
}

impl Serialize for Executor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl Executor {
    pub fn iter() -> impl Iterator<Item = Self> {
        vec![
            Executor::Sequential,
            Executor::Parallel,
            Executor::Mixed(15),
            Executor::Mixed(20),
            Executor::Mixed(25),
            Executor::Mixed(30),
        ]
        .into_iter()
    }
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
    #[strum(
        serialize = "available-expr",
        serialize = "available-expressions",
        serialize = "available-exprs"
    )]
    AvailableExpr,
}

macro_rules! run {
    ($executor: ident, $pass: ident, $input: ident) => {{
        let (timings, data) = $crate::execute_pass!($pass, $executor, $input);

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
            Pass::AvailableExpr => run!(executor, AvailableExpr, input),
        }
    }
}
