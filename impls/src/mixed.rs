use bril_utils::{CFG, Dataflow};
use utils::{DataflowExecutor, DataflowSpec};

#[derive(Default)]
/// Dynamically choose between two executors based on the size of the CFG
pub struct MixedExecutor<Ex1, Ex2> {
    /// The threshold (in basic block count) to switch between executor1 and executor2
    threshold: usize,
    /// The first executor to use
    executor1: Ex1,
    /// The second executor to use
    executor2: Ex2,
}

impl<Ex1, Ex2> MixedExecutor<Ex1, Ex2> {
    /// Create a new MixedExecutor with the given threshold and executors
    pub fn new(threshold: usize, executor1: Ex1, executor2: Ex2) -> Self {
        Self {
            threshold,
            executor1,
            executor2,
        }
    }
}

impl<Pass, Ex1, Ex2> DataflowExecutor<Pass> for MixedExecutor<Ex1, Ex2>
where
    Pass: DataflowSpec + Send + Sync,
    Ex1: DataflowExecutor<Pass>,
    Ex2: DataflowExecutor<Pass>,
{
    fn cfg(&self, pass: &Pass, cfg: CFG) -> Dataflow<Pass::Val> {
        if cfg.len() > self.threshold {
            self.executor2.cfg(pass, cfg)
        } else {
            self.executor1.cfg(pass, cfg)
        }
    }
}
