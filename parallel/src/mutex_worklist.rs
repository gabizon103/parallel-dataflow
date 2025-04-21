use lesson4::framework::{AnalysisFramework, DataFlowAnalysis};
use std::fmt::Debug;

/// Trait required to extend functionality of externally-defined AnalysisFramework struct
pub trait ParallelDataFlow<T>
where
    T: Default + Clone + PartialEq + Debug,
{
    fn parallel_worklist(&mut self, analysis: &impl DataFlowAnalysis<T>) -> ();
}

impl<T> ParallelDataFlow<T> for AnalysisFramework<T>
where
    T: Default + Clone + PartialEq + Debug,
{
    fn parallel_worklist(&mut self, analysis: &impl DataFlowAnalysis<T>) -> () {}
}
