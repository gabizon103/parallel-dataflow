use itertools::Itertools;
use lesson4::framework::{AnalysisFramework, DataFlowAnalysis};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread::{self, JoinHandle},
};

const NUM_WORKERS: u8 = 5;

/// Trait required to extend functionality of externally-defined AnalysisFramework struct.
/// Generic type `T` is meant to represent the elements of the lattice.
pub trait MutexFixpoint<'b, 'c, T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync,
{
    fn worklist(&'b mut self, analysis: impl (DataFlowAnalysis<T>) + Send + Sync + 'c) -> ();
}

impl<'b, 'c, T> MutexFixpoint<'b, 'c, T> for AnalysisFramework<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync + 'c,
    'c: 'static,
{
    fn worklist(&'b mut self, analysis: impl (DataFlowAnalysis<T>) + Send + Sync + 'c) {
        // define vec to force completion of threads later
        let mut handles: Vec<JoinHandle<()>> = vec![];

        // use access to functions in `analysis`
        let shareable_analysis = Arc::new(analysis);

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

        // init threads
        for _ in 0..NUM_WORKERS {
            // pass an atomic reference of each object to each thread
            let (worklist, succs, preds, ins, outs, analysis) = (
                Arc::clone(&shareable_worklist),
                Arc::clone(&shareable_succs),
                Arc::clone(&shareable_preds),
                Arc::clone(&shareable_ins),
                Arc::clone(&shareable_outs),
                Arc::clone(&shareable_analysis),
            );
            let handle = thread::spawn(move || {
                let worklist_lock = worklist.lock().unwrap();
                match worklist_lock.front() {
                    None => drop(worklist_lock),
                    Some(block_ref) => {
                        // release lock on worklist queue
                        let block = *block_ref;
                        drop(worklist_lock);

                        // // acquire read lock on out[p] for all p
                        let out_read_guards: Vec<RwLockReadGuard<T>> = match preds.get(&block) {
                            None => vec![],
                            Some(p) => p.iter().map(|v| *v).collect(),
                        }
                        .into_iter()
                        .map(|pred: usize| {
                            let read_guard = outs[pred].read().unwrap();
                            read_guard
                        })
                        .collect();

                        // get ownership over these by cloning
                        let incoming_values = out_read_guards
                            .iter()
                            .map(|guard| {
                                let guard_deref = RwLockReadGuard::deref(guard);
                                guard_deref.clone()
                            })
                            .collect_vec();

                        // read access to in[b]
                        let in_b_read_guard = ins[block].read().unwrap();
                        let in_b = RwLockReadGuard::deref(&in_b_read_guard);

                        // merge
                        let merge_output = analysis.merge(in_b, incoming_values);
                        drop(in_b_read_guard);

                        // write access to in[b]
                        let mut in_b_write_guard = ins[block].write().unwrap();
                        let in_b = RwLockWriteGuard::deref_mut(&mut in_b_write_guard);
                        *in_b = merge_output;
                        drop(in_b_write_guard);

                        // drop read access of each out[p] after finished merging
                        out_read_guards.into_iter().for_each(drop);

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
