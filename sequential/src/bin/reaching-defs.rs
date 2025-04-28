use passes::ReachingDefs;
use sequential::SequentialExecutor;
use utils::DataflowExecutor;

fn main() {
    SequentialExecutor::run(&ReachingDefs);
}
