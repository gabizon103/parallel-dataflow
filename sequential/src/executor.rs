use bril_utils::{CFG, Dataflow};
use itertools::Itertools;
use std::{collections::LinkedList, fmt::Debug};
use utils::{DataflowExecutor, DataflowSpec};

#[derive(Default)]
pub struct SequentialExecutor;

impl<Pass, Val> DataflowExecutor<Pass, Val> for SequentialExecutor
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

        let mut worklist: LinkedList<_> = (0..n).collect();
        while let Some(i) = worklist.pop_front() {
            in_vals[i] = if cfg.func().get(i).is_entry() {
                pass.entry(cfg.func())
            } else {
                let inputs = cfg
                    .preds(i)
                    .iter()
                    .map(|&j| out_vals[j].clone())
                    .collect_vec();
                pass.meet(&inputs)
            };

            let new_vals = pass.transfer(cfg.func().get(i), &in_vals[i]);

            if new_vals != out_vals[i] {
                out_vals[i] = new_vals;
                for j in cfg.succs(i) {
                    worklist.push_back(j);
                }
            }
        }

        // The exit value can be computed by meeting all the out values of exit block(s)
        let exit_val = cfg
            .exits()
            .into_iter()
            .map(|i| out_vals[i].clone())
            .collect_vec();
        let exit_val = pass.meet(&exit_val);
        let exit_val = pass.finish(cfg.func(), exit_val);

        Dataflow {
            cfg,
            in_vals,
            out_vals,
            exit_val,
        }
    }
}
