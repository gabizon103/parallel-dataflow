use argh::FromArgs;
use impls::{Executor, ParallelExecutor, Pass, SequentialExecutor};
use passes::{ConstProp, LiveVars, ReachingDefs};
use simple_logger::SimpleLogger;
use strum::IntoEnumIterator;
use utils::DataflowExecutor;

#[derive(FromArgs)]
/// Generate performance statistics for all benchmarks in a directory
/// and all pass - executor combinations
struct Args {
    /// the log level
    #[argh(option, short = 'l', default = "log::LevelFilter::Info")]
    log: log::LevelFilter,
    /// directory of benchmarks
    #[argh(option, short = 'd', default = "String::from(\"core/\")")]
    dir: String,
}

/// Has to be done in a macro due to different types for different passes
macro_rules! test {
    ($pass: ident, $input: expr) => {{
        let mut expectation = None;

        for executor in Executor::iter() {
            let input = std::fs::File::open($input).unwrap();
            let (_, result) = match executor {
                Executor::Sequential => SequentialExecutor::run(&$pass, input.try_clone().unwrap()),
                Executor::Parallel => ParallelExecutor::run(&$pass, input.try_clone().unwrap()),
            };

            match expectation {
                None => {
                    expectation = Some(result);
                }
                Some(ref e) => {
                    if result != *e {
                        // Loop through each function and find the first one that is different

                        for (a, b) in result.iter().zip(e.iter()) {
                            if a != b {
                                log::error!("Expected:\n{:?}\n", b);
                                log::error!("Got:\n{:?}\n", a);

                                // Find the specific block that caused the issue
                                for (i, (x, y)) in
                                    a.in_vals.iter().zip(b.in_vals.iter()).enumerate()
                                {
                                    if x != y {
                                        log::error!("\n.{}:\n\tIn: {:?}", i, x);
                                        log::error!("\n.{}:\n\tIn: {:?}", i, y);
                                    }
                                }
                                // Find the specific block that caused the issue
                                for (i, (x, y)) in
                                    a.out_vals.iter().zip(b.out_vals.iter()).enumerate()
                                {
                                    if x != y {
                                        log::error!("\n.{}:\n\tOut: {:?}", i, x);
                                        log::error!("\n.{}:\n\tOut: {:?}", i, y);
                                    }
                                }
                                panic!("Executor {:?} produced different results", executor);
                            }
                        }

                        unreachable!();
                    }
                }
            }
        }
    }};
}

fn main() {
    // Loop through every *.bril file in the core/ directory
    // and run every pass with every executor

    let args: Args = argh::from_env();
    let dir = std::fs::read_dir(args.dir).unwrap();

    SimpleLogger::new()
        .with_colors(true)
        .with_level(args.log)
        .without_timestamps()
        .init()
        .unwrap();

    #[cfg(debug_assertions)]
    log::warn!("Running performance benchmarks in debug mode. This may be very slow.");

    for entry in dir {
        let entry = entry.unwrap();
        if entry.path().extension().unwrap() == "bril" {
            log::info!("Test {}", entry.path().display(),);
            for pass in Pass::iter() {
                match pass {
                    Pass::ReachingDefinitions => test!(ReachingDefs, entry.path()),
                    Pass::LiveVariables => test!(LiveVars, entry.path()),
                    Pass::ConstProp => test!(ConstProp, entry.path()),
                };
            }
        }
    }
}
