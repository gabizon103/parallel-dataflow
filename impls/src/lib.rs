mod parallel;
mod passes;
mod sequential;

pub use parallel::ParallelExecutor;
pub use passes::{Executor, Pass};
pub use sequential::SequentialExecutor;
