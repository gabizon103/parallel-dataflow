use argh::FromArgs;
use impls::{Executor, Pass};
use serde::Serialize;
use simple_logger::SimpleLogger;
use strum::IntoEnumIterator;

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
    /// output file
    #[argh(option, short = 'o', default = "String::from(\"perf.csv\")")]
    output: String,
    /// number of iterations per benchmark
    #[argh(option, short = 'i', default = "10")]
    iterations: usize,
}

#[derive(Serialize)]
struct Record {
    pass: Pass,
    executor: Executor,
    iteration: usize,
    loadtime: u128,
    runtime: u128,
    writetime: u128,
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

    log::info!("Writing results to {}", args.output);
    let mut wtr = csv::Writer::from_path(args.output).unwrap();

    for entry in dir {
        let entry = entry.unwrap();
        if entry.path().extension().unwrap() == "bril" {
            log::info!(
                "Running ({}x)  benchmarks for {}",
                args.iterations,
                entry.path().display(),
            );
            let input = std::fs::File::open(entry.path()).unwrap();
            for iter in 0..args.iterations {
                for pass in Pass::iter() {
                    for executor in Executor::iter() {
                        let result = pass.execute(&executor, input.try_clone().unwrap());

                        wtr.serialize(Record {
                            pass,
                            executor,
                            iteration: iter,
                            loadtime: result.loadtime.as_nanos(),
                            runtime: result.runtime.as_nanos(),
                            writetime: result.writetime.as_nanos(),
                        })
                        .unwrap();
                    }
                }
            }
        }
    }

    wtr.flush().unwrap();
}
