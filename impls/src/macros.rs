#[macro_export]
/// Execute a pass with a given executor
macro_rules! execute_pass {
    ($pass: expr, $executor: ident, $input: ident) => {
        match $executor {
            Executor::Sequential => $crate::SequentialExecutor.run(&$pass, $input, false),
            Executor::Parallel => $crate::ParallelExecutor.run(&$pass, $input, false),
            Executor::Mixed(thresh) => $crate::MixedExecutor::new(
                *thresh,
                $crate::SequentialExecutor,
                $crate::ParallelExecutor,
            )
            .run(&$pass, $input, false),
        }
    };
}
