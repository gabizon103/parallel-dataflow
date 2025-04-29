use itertools::Itertools;
use lesson2::BasicBlock;
use lesson4::framework::{AnalysisFramework, DataFlowAnalysis};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread::{self},
};

const NUM_WORKERS: u8 = 2;

/// Trait required to extend functionality of externally-defined AnalysisFramework struct.
/// Generic type `T` is meant to represent the elements of the lattice. In addition
/// to existing trait impls, T is required to implement Send and Sync.
pub trait MutexFixpoint<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync,
{
    fn worklist(&mut self, analysis: &(impl (DataFlowAnalysis<T>) + Send + Sync)) -> ();
}

pub trait SequentialFixpoint<T>
where
    T: Default + Clone + PartialEq + Debug,
{
    fn worklist(&mut self, analysis: &impl (DataFlowAnalysis<T>)) -> ();
}

pub trait MutexFixpoint2<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync,
{
    fn worklist(&mut self, analysis: &(impl (DataFlowAnalysis<T>) + Send + Sync)) -> ();
}

impl<T> MutexFixpoint2<T> for AnalysisFramework<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync,
{
    fn worklist(&mut self, analysis: &(impl (DataFlowAnalysis<T>) + Send + Sync)) {}
}

impl<T> SequentialFixpoint<T> for AnalysisFramework<T>
where
    T: Default + Clone + PartialEq + Debug,
{
    fn worklist(&mut self, analysis: &impl (DataFlowAnalysis<T>)) -> () {
        let mut singleton: HashSet<usize> = HashSet::new();
        singleton.insert(0);

        let (mut worklist, mut visited, mut dependencies): (
            HashSet<usize>,
            HashSet<usize>,
            HashMap<usize, HashSet<usize>>,
        ) = (singleton.clone(), singleton, HashMap::new());

        loop {
            match worklist.iter().next() {
                None => break,
                Some(block_ref) => {
                    let (mut c, mut r, mut t): (HashSet<usize>, HashSet<usize>, HashSet<usize>) =
                        (HashSet::new(), HashSet::new(), HashSet::new());

                    // capture block
                    let block = *block_ref;
                    self.ins[block] = if block == 0 {
                        self.ins[block].clone()
                    } else {
                        T::default()
                    };
                    worklist.remove(&block);

                    print!("operating on: {block}");
                    worklist.iter().for_each(|f| print!(", {f}"));
                    println!();

                    let b_preds = self.cfg.preds(block);
                    let incoming_values = b_preds
                        .iter()
                        .map(|pred_idx: &usize| {
                            r.insert(*pred_idx);
                            self.outs[*pred_idx].clone()
                        })
                        .collect_vec();

                    r.insert(block);

                    self.ins[block] = analysis.merge(&self.ins[block], incoming_values);

                    let out = analysis.transfer(
                        &self.ins[block],
                        self.cfg.blocks.get(block).unwrap(),
                        block,
                    );

                    let mut flag = false;
                    if self.outs[block] != out {
                        flag = true;
                    }

                    let b_succs = self.cfg.succs(block);

                    self.outs[block] = out;
                    if flag {
                        t.extend(b_succs.clone());
                    }

                    c.extend(b_succs);

                    let c_mod_v: HashSet<usize> = c.difference(&visited).map(|v| *v).collect();
                    worklist = worklist.union(&c_mod_v).map(|v| *v).collect();
                    print!("worklist after uniting with C \\ V: ");
                    worklist.iter().for_each(|f| print!(", {f}"));
                    println!();
                    visited = visited.union(&c).map(|v| *v).collect();

                    r.iter().for_each(|dependency_of_block| {
                        dependencies
                            .entry(*dependency_of_block)
                            .and_modify(|set| {
                                set.insert(block);
                            })
                            .or_insert(HashSet::from_iter(vec![block]));
                    });
                    print!("\nt: ");

                    t.iter().for_each(|impacted_by_block| {
                        print!("{impacted_by_block}, ");

                        match dependencies.get(impacted_by_block) {
                            None => (),
                            Some(d_a) => {
                                worklist.extend(d_a.iter());
                            }
                        }
                    });
                    println!();

                    print!("\ndependency graph: \n");
                    dependencies.iter().for_each(|(key, set)| {
                        print!("  {key}: ");
                        set.iter().for_each(|s| print!("{s}, "));
                        println!()
                    });
                    println!();

                    print!("worklist after uniting with dependencies: ");
                    worklist.iter().for_each(|f| print!(", {f}"));
                    println!();
                }
            }
        }

        ()
    }
}

impl<T> MutexFixpoint<T> for AnalysisFramework<T>
where
    T: Default + Clone + PartialEq + Debug + Send + Sync,
{
    fn worklist(&mut self, analysis: &(impl (DataFlowAnalysis<T>) + Send + Sync)) {
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

        // array of locks to avoid two threads operating on same block b
        let shareable_collision_list: Arc<Vec<Mutex<()>>> =
            Arc::new(self.worklist.iter().map(|_| Mutex::new(())).collect());

        // shared memory of worklist
        let shareable_worklist: Arc<Mutex<VecDeque<usize>>> =
            Arc::new(Mutex::new(VecDeque::from_iter(self.worklist.drain(..))));

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
            for i in 0..NUM_WORKERS {
                // pass a reference of each object to each thread
                let (worklist, succs, preds, ins, outs, analysis, blocks, collision_list) = (
                    Arc::clone(&shareable_worklist),
                    Arc::clone(&shareable_succs),
                    Arc::clone(&shareable_preds),
                    Arc::clone(&shareable_ins),
                    Arc::clone(&shareable_outs),
                    Arc::clone(&shareable_analysis),
                    Arc::clone(&shareable_blocks),
                    Arc::clone(&shareable_collision_list),
                );
                spawner.spawn(move || {
                    loop {
                        // acquire lock to pop block off of worklist queue
                        let mut worklist_lock: MutexGuard<VecDeque<usize>> =
                            worklist.lock().unwrap();

                        // now, we have a lock
                        match worklist_lock.pop_front() {
                            None => {
                                drop(worklist_lock);
                                break;
                            }
                            Some(block) => {
                                // release lock on worklist queue
                                print!("Worklist before operation of thread {i}: ");
                                print!("{block}");
                                worklist_lock.iter().for_each(|v| print!(", {v}"));
                                println!();
                                println!("Thread {i} got block {block}");

                                drop(worklist_lock);

                                // acquire sole right to this block
                                let collision_guard = collision_list.get(block).unwrap().lock();
                                println!("Thread {i} got through!");

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
                                let in_b_read_guard: RwLockReadGuard<T> =
                                    ins[block].read().unwrap();
                                let in_b: &T = in_b_read_guard.deref();

                                // merge and drop read access to in[b]
                                let merge_output: T = analysis.merge(in_b, incoming_values);
                                drop(in_b_read_guard);

                                // acquire write access to in[b] and write merge output to it
                                let mut in_b_write_guard = ins[block].write().unwrap();
                                let in_b: &mut T = in_b_write_guard.deref_mut();

                                *in_b = merge_output;
                                println!("Thread {i} holding block {block} has the following for in[b]: {:#?}", in_b);
                                drop(in_b_write_guard);

                                // drop read access of each out[p] after finished merging
                                out_read_guards.into_iter().for_each(drop);

                                // acquire write access to out[b] & read access to in[b]
                                let mut out_b_write_guard: RwLockWriteGuard<T> =
                                    outs[block].write().unwrap();

                                let in_b_read_guard: RwLockReadGuard<T> =
                                    ins[block].read().unwrap();

                                // do transfer
                                let in_b: &T = in_b_read_guard.deref();
                                let new_out =
                                    analysis.transfer(in_b, blocks.get(block).unwrap(), block);

                                drop(in_b_read_guard);

                                // compare results
                                if !(out_b_write_guard.deref().eq(&new_out)) {
                                    // write successors to worklist
                                    let mut worklist_lock: MutexGuard<VecDeque<usize>> =
                                        worklist.lock().unwrap();

                                    // write to out[b]
                                    let out_b = out_b_write_guard.deref_mut();

                                    *out_b = new_out;

                                    let block_succs = match succs.get(&block) {
                                        None => vec![],
                                        Some(p) => p.iter().map(|v| *v).collect(),
                                    };

                                    worklist_lock.extend(block_succs);
                                    worklist_lock.push_back(block);
                                    print!("Worklist after {i} added successors: ");
                                    worklist_lock.iter().for_each(|v| print!(", {v}"));
                                    println!();
                                    drop(worklist_lock);
                                    drop(out_b_write_guard);
                                }
                                drop(collision_guard);
                                println!("Thread {i} finished!");
                            }
                        }
                    }
                });
            }
        });

        self.ins.extend(
            shareable_ins
                .deref()
                .iter()
                .map(|rwlock: &RwLock<T>| rwlock.read().unwrap().clone()),
        );

        self.outs.extend(
            shareable_outs
                .deref()
                .iter()
                .map(|rwlock| rwlock.read().unwrap().clone()),
        )
    }
}
