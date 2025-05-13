use crate::DataflowSpec;
use bril_utils::{CFG, CanonicalizeLiterals, Dataflow, Pass, bril_rs::Program};
use bril2json::parse_abstract_program_from_read;
use itertools::Itertools;
use std::{
    time::{Duration, Instant},
};
use rayon::prelude::*;

pub struct PassTiming {
    pub loadtime: Duration,
    pub runtime: Duration,
}

pub trait DataflowExecutor<Pass>
where
    Pass: DataflowSpec + Send + Sync,
    Self: Send + Sync,
{
    /// Run the dataflow pass on the input program and perform performance measurements
    fn run<R: std::io::Read>(
        &self,
        pass: &Pass,
        input: R,
        par_func_analysis: bool,
    ) -> (PassTiming, Vec<Dataflow<Pass::Val>>) {
        let start = Instant::now();

        // Read stdin and parse it into a Program using serde
        let prog: Program = parse_abstract_program_from_read(input, false, false, None)
            .try_into()
            .unwrap();

        // Perform CanonicalizeLiterals always just to make sure things are canonical
        let mut prog = CanonicalizeLiterals.run(prog);

        let loadtime = start.elapsed();

        if par_func_analysis {
            // log::info!("par func analysis");
            // println!("hi");
            let start = Instant::now();
            // let shared_results = Arc::new(Mutex::new(Vec::new()));

            // thread::scope(|thread_spawner| {
            //     prog.functions
            //         .iter()
            //         .map(|f| CFG::from(f.clone()))
            //         .enumerate()
            //         .for_each(|(i, cfg)| {
            //             let local_results_ref = Arc::clone(&shared_results);
            //             thread_spawner.spawn(move || {
            //                 local_results_ref
            //                     .lock()
            //                     .unwrap()
            //                     .push((i, self.cfg(pass, cfg)));
            //             });
            //         })
            // });
            let results: Vec<_> = std::mem::take(&mut prog.functions).into_par_iter().map(|f| {
                let cfg = CFG::from(f.clone());
                self.cfg(pass, cfg)
            }).collect();

            // let results = Arc::try_unwrap(shared_results)
            //     .unwrap()
            //     .into_inner()
            //     .unwrap()
            //     .into_iter()
            //     .sorted_by(|(i, _), (j, _)| i.cmp(j))
            //     .map(|(_, r)| r)
            //     .collect_vec();

            let runtime = start.elapsed();

            // todo!()
            (PassTiming { loadtime, runtime }, results)
        } else {
            let start = Instant::now();

            let results = prog
                .functions
                .iter()
                .map(|f| self.cfg(pass, CFG::from(f.clone())))
                .collect_vec();

            let runtime = start.elapsed();

            (PassTiming { loadtime, runtime }, results)
        }
    }

    fn cfg(&self, pass: &Pass, cfg: CFG) -> Dataflow<Pass::Val>;
}
