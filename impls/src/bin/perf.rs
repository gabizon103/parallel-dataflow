use argh::FromArgs;
use impls::{Executor, Pass};
use itertools::Itertools;
use serde::Serialize;
use simple_logger::SimpleLogger;
use std::process::Command;
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
    name: String,
    pass: Pass,
    executor: Executor,
    iteration: usize,
    loadtime: u128,
    runtime: u128,
}

// Path to the main executable
#[cfg(debug_assertions)]
const MAIN_EXECUTABLE: &str = "./target/debug/main";
#[cfg(not(debug_assertions))]
const MAIN_EXECUTABLE: &str = "./target/release/main";

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

    log::info!("Writing results to {}", args.output);
    let mut wtr = csv::Writer::from_path(args.output).unwrap();

    for entry in dir {
        let entry = entry.unwrap();
        let entry_name = entry.file_name().clone();
        let entry_name = entry_name.to_str().unwrap().split(".").collect_vec()[0];
        if entry.path().extension().unwrap() == "bril" {
            log::info!(
                "Running ({}x) benchmarks for {}",
                args.iterations,
                entry.path().display(),
            );
            for pass in Pass::iter().filter(|pass| !matches!(pass, Pass::ConstProp)) {
                for executor in Executor::iter() {
                    for iter in 0..args.iterations {
                        // Dispatch a new process for each pass and executor to avoid cache
                        // pollution. The process is located in /target/release/main

                        let output = Command::new(MAIN_EXECUTABLE)
                            .stdin(std::fs::File::open(entry.path()).unwrap())
                            .arg("-r") // raw output
                            .arg("-a") // algorithm
                            .arg(executor.to_string())
                            .arg("-p") // pass
                            .arg(pass.to_string())
                            .output()
                            .unwrap();

                        let output = std::str::from_utf8(&output.stdout).unwrap();

                        // Output consists of the 2 times in nanoseconds separated by newlines
                        let times: Vec<u128> = output
                            .lines()
                            .map(|line| line.parse::<u128>().unwrap())
                            .collect();

                        if times.len() != 2 {
                            log::error!("Invalid output: {}", output);
                            continue;
                        }

                        wtr.serialize(Record {
                            name: entry_name.into(),
                            pass,
                            executor,
                            iteration: iter,
                            loadtime: times[0],
                            runtime: times[1],
                        })
                        .unwrap();
                    }
                }
            }
        }
    }

    wtr.flush().unwrap();
}
