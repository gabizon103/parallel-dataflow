use itertools::Itertools;
use lesson2::ControlFlow;
use lesson4::framework::{AnalysisFramework, DataFlowAnalysis, Direction};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    hash::Hash,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard},
    thread::{self, JoinHandle},
};

const NUM_WORKERS: u8 = 5;

/// Trait required to extend functionality of externally-defined AnalysisFramework struct.
/// Generic type `T` is meant to represent the elements of the lattice.
pub trait MutexFixpoint<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync + 'static,
{
    fn worklist(&mut self, analysis: &impl DataFlowAnalysis<T>) -> ();
}

impl<T> MutexFixpoint<T> for AnalysisFramework<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync + 'static,
{
    fn worklist(&mut self, analysis: &impl DataFlowAnalysis<T>) {
        // define vec to force completion of threads later
        let mut handles: Vec<JoinHandle<()>> = vec![];

        // read access for successors and predecessors of each block
        let block_ids = self.cfg.lbl_to_block.values().collect_vec();
        let (mut succs_map, mut preds_map): (
            HashMap<usize, Vec<usize>>,
            HashMap<usize, Vec<usize>>,
        ) = (HashMap::new(), HashMap::new());
        block_ids.into_iter().for_each(|block_id| {
            let deref = *block_id;
            succs_map.entry(deref).insert_entry(self.cfg.succs(deref));
            preds_map
                .entry(deref)
                .insert_entry(self.cfg.preds(deref).into_iter().collect());
        });
        let shareable_succs = Arc::new(succs_map);
        let shareable_preds = Arc::new(preds_map);

        // shared memory of worklist
        let shareable_worklist: Arc<Mutex<VecDeque<usize>>> =
            Arc::new(Mutex::new(self.worklist.clone()));

        // shared memory of `in` and `out` arrays
        let (shareable_ins, shareable_outs) = vec![self.ins.drain(..), self.outs.drain(..)]
            .into_iter()
            .map(|iterator| Arc::new(iterator.map(|v: T| RwLock::new(v)).collect_vec()))
            .collect_tuple()
            .unwrap();

        // init threads; each thread in charge of computing in[b] and out[b] for
        // one b in worklist
        for _ in 0..NUM_WORKERS {
            let (worklist, succs, preds, ins, outs) = (
                Arc::clone(&shareable_worklist),
                Arc::clone(&shareable_succs),
                Arc::clone(&shareable_preds),
                Arc::clone(&shareable_ins),
                Arc::clone(&shareable_outs),
            );
            let handle = thread::spawn(move || {
                let worklist_lock = worklist.lock().unwrap();
                match worklist_lock.front() {
                    None => drop(worklist_lock),
                    Some(block_ref) => {
                        // release lock on worklist queue
                        let block = *block_ref;
                        drop(worklist_lock);

                        // read lock on outs of all predecessors
                        let incoming_values: Vec<RwLockReadGuard<T>> = match preds.get(&block) {
                            None => vec![],
                            Some(p) => p.iter().map(|v| *v).collect(),
                        }
                        .into_iter()
                        .map(|pred: usize| outs[pred].read().unwrap())
                        .collect_vec();

                        ()
                    }
                }
            });
            handles.push(handle);
        }

        // thunk
        for handle in handles.into_iter() {
            handle.join().unwrap();
        }
    }
}
