use itertools::Itertools;
use lesson2::BasicBlock;
use lesson4::framework::{AnalysisFramework, DataFlowAnalysis};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread::{self},
};

const NUM_WORKERS: u8 = 5;

/// Trait required to extend functionality of externally-defined AnalysisFramework struct.
/// Generic type `T` is meant to represent the elements of the lattice. In addition
/// to existing trait impls, T is required to implement Send and Sync.
pub trait MutexFixpoint<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync,
{
    fn worklist(&mut self, analysis: impl (DataFlowAnalysis<T>) + Send + Sync) -> ();
}

impl<T> MutexFixpoint<T> for AnalysisFramework<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync,
{
    fn worklist(&mut self, analysis: impl (DataFlowAnalysis<T>) + Send + Sync) {
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

        // shared memory of basic blocks array
        let shareable_blocks: Arc<Vec<BasicBlock>> = Arc::new(self.cfg.blocks.clone());

        // init threads
        thread::scope(|spawner| {
            for _ in 0..NUM_WORKERS {
                // pass an atomic reference of each object to each thread
                let (worklist, succs, preds, ins, outs, analysis, blocks) = (
                    Arc::clone(&shareable_worklist),
                    Arc::clone(&shareable_succs),
                    Arc::clone(&shareable_preds),
                    Arc::clone(&shareable_ins),
                    Arc::clone(&shareable_outs),
                    Arc::clone(&shareable_analysis),
                    Arc::clone(&shareable_blocks),
                );
                spawner.spawn(move || {
                    // acquire lock to pop block off of worklist queue
                    let worklist_lock: MutexGuard<VecDeque<usize>> = worklist.lock().unwrap();

                    // now, we have a lock
                    match worklist_lock.front() {
                        None => drop(worklist_lock),
                        Some(block_ref) => {
                            // release lock on worklist queue
                            let block = *block_ref;
                            drop(worklist_lock);

                            // acquire read lock on out[p] for all p
                            let out_read_guards: Vec<RwLockReadGuard<T>> =
                                match preds.get(&block) {
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
                                .map(|guard: &RwLockReadGuard<T>| {
                                    let guard_deref = RwLockReadGuard::deref(guard);
                                    guard_deref.clone()
                                })
                                .collect_vec();

                            // read access to in[b]
                            let in_b_read_guard: RwLockReadGuard<T> = ins[block].read().unwrap();
                            let in_b: &T = in_b_read_guard.deref();

                            // merge and drop read access to in[b]
                            let merge_output: T = analysis.merge(in_b, incoming_values);
                            drop(in_b_read_guard);

                            // acquire write access to in[b] and write merge output to it
                            let mut in_b_write_guard = ins[block].write().unwrap();
                            let in_b: &mut T = in_b_write_guard.deref_mut();
                            *in_b = merge_output;
                            drop(in_b_write_guard);

                            // drop read access of each out[p] after finished merging
                            out_read_guards.into_iter().for_each(drop);

                            // acquire write access to out[b] & read access to in[b]
                            let mut out_b_write_guard: RwLockWriteGuard<T> =
                                outs[block].write().unwrap();
                            let in_b_read_guard: RwLockReadGuard<T> = ins[block].read().unwrap();

                            // do transfer
                            let in_b: &T = in_b_read_guard.deref();
                            let new_out =
                                analysis.transfer(in_b, blocks.get(block).unwrap(), block);

                            // compare results
                            if !(out_b_write_guard.deref().eq(&new_out)) {
                                // write to out[b]
                                let out_b = out_b_write_guard.deref_mut();
                                *out_b = new_out;

                                let block_succs = match succs.get(&block) {
                                    None => vec![],
                                    Some(p) => p.iter().map(|v| *v).collect(),
                                };

                                // write successors to worklist
                                let mut worklist_lock: MutexGuard<VecDeque<usize>> =
                                    worklist.lock().unwrap();
                                worklist_lock.extend(block_succs);
                            }
                        }
                    }
                });
            }
        });
    }
}
