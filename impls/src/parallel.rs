use bril_utils::{CFG, Dataflow};
use itertools::Itertools;
use rayon::prelude::*;
use std::{collections::HashSet, fmt::Debug};
use utils::{DataflowExecutor, DataflowSpec};

#[derive(Default)]
pub struct ParallelExecutor;

impl<Pass, Val> DataflowExecutor<Pass, Val> for ParallelExecutor
where
    Val: Eq + Clone + Debug + Send + Sync,
    Pass: DataflowSpec<Val> + Send + Sync,
{
    fn cfg(pass: &Pass, cfg: CFG) -> Dataflow<Val> {
        let cfg = if cfg.reversed() != pass.reversed() {
            cfg.reverse()
        } else {
            cfg
        };

        let n = cfg.len();

        let mut out_vals = vec![pass.init(cfg.func()); n];
        let mut in_vals = vec![pass.init(cfg.func()); n];

        let mut worklist: HashSet<_> = (0..n).collect();

        while !worklist.is_empty() {
            // Dispatch the worklist to multiple threads
            let results: Vec<_> = std::mem::take(&mut worklist)
                .into_par_iter()
                .filter_map(|i| {
                    let i_vals = if cfg.func().get(i).is_entry() {
                        pass.entry(cfg.func())
                    } else {
                        let inputs = cfg
                            .preds(i)
                            .iter()
                            .map(|&j| out_vals[j].clone())
                            .collect_vec();
                        pass.meet(&inputs)
                    };

                    let o_vals = pass.transfer(cfg.func().get(i), &i_vals);

                    if o_vals != out_vals[i] {
                        Some((i, cfg.succs(i), i_vals, o_vals))
                    } else {
                        None
                    }
                })
                .collect();

            for (i, result_succs, i_vals, o_vals) in results {
                out_vals[i] = o_vals;
                in_vals[i] = i_vals;
                for j in result_succs {
                    worklist.insert(j);
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
