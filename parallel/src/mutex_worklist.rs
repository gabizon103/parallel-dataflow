use itertools::Itertools;
use lesson2::ControlFlow;
use lesson4::framework::{AnalysisFramework, DataFlowAnalysis, Direction};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
    thread::{self, JoinHandle},
};

const NUM_WORKERS: u8 = 5;

/// Trait required to extend functionality of externally-defined AnalysisFramework struct.
/// Generic type `T` is meant to represent the elements of the lattice.
pub trait MutexFixpoint<T>
where
    T: Default + Clone + PartialEq + Debug,
{
    fn worklist(&mut self, analysis: &impl DataFlowAnalysis<T>) -> ();
}

impl<T> MutexFixpoint<T> for AnalysisFramework<T>
where
    T: Default + Clone + PartialEq + Debug,
{
    fn worklist(&mut self, analysis: &impl DataFlowAnalysis<T>) {
        // define vec to force completion of threads later
        let mut handles: Vec<JoinHandle<()>> = vec![];

        // read lock for successors and predecessors of each block
        let block_ids = self.cfg.lbl_to_block.values().map(|id| *id).collect_vec();
        let (mut succs_map, mut preds_map): (
            HashMap<usize, Vec<usize>>,
            HashMap<usize, Vec<usize>>,
        ) = (HashMap::new(), HashMap::new());
        block_ids.into_iter().for_each(|block_id| {
            succs_map
                .entry(block_id)
                .insert_entry(self.cfg.succs(block_id));
            preds_map
                .entry(block_id)
                .insert_entry(self.cfg.preds(block_id).into_iter().collect());
        });
        let shareable_succs = Arc::new(succs_map);
        let shareable_succs = Arc::new(preds_map);

        // shared memory of worklist
        let shareable_worklist: Arc<Mutex<VecDeque<usize>>> =
            Arc::new(Mutex::new(self.worklist.clone()));

        // shared memory of `in` and `out` arrays
        let (shareable_ins, shareable_outs): (Vec<RwLock<T>>, Vec<RwLock<T>>) =
            vec![self.ins.drain(..), self.outs.drain(..)]
                .into_iter()
                .map(|iterator| iterator.map(|v: T| RwLock::new(v)).collect_vec())
                .collect_tuple()
                .unwrap();

        // init threads; each thread in charge of computing in[b] and out[b] for
        // one b in worklist
        for _ in 0..NUM_WORKERS {
            let moveable_reference = Arc::clone(&shareable_worklist);
            let handle = thread::spawn(move || {
                let worklist_lock = moveable_reference.lock().unwrap();
                match worklist_lock.front() {
                    None => drop(worklist_lock),
                    Some(block) => {
                        drop(worklist_lock);
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
