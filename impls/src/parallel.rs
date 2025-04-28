use bril_utils::{CFG, Dataflow};
use std::fmt::Debug;
use utils::{DataflowExecutor, DataflowSpec};

#[derive(Default)]
pub struct ParallelExecutor;

impl<Pass, Val> DataflowExecutor<Pass, Val> for ParallelExecutor
where
    Val: Eq + Clone + Debug,
    Pass: DataflowSpec<Val>,
{
    fn cfg(pass: &Pass, cfg: CFG) -> Dataflow<Val> {
        let cfg = if cfg.reversed() != pass.reversed() {
            cfg.reverse()
        } else {
            cfg
        };

        let n = cfg.len();

        let mut in_vals = vec![pass.init(cfg.func()); n];
        let mut out_vals = vec![pass.init(cfg.func()); n];

        unimplemented!()
    }
}
