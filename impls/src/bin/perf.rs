use argh::FromArgs;
use impls::{Executor, Pass};
use strum::IntoEnumIterator;

#[derive(FromArgs)]
/// Generate performance statistics for all benchmarks in a directory
/// and all pass - executor combinations
pub struct Args {
    /// directory of benchmarks
    #[argh(option, short = 'd', default = "String::from(\"core/\")")]
    dir: String,
}

fn main() {
    // Loop through every *.bril file in the core/ directory
    // and run every pass with every executor

    let args: Args = argh::from_env();
    let dir = std::fs::read_dir(args.dir).unwrap();

    for entry in dir {
        let entry = entry.unwrap();
        if entry.path().extension().unwrap() == "bril" {
            println!("Running benchmarks on {}", entry.path().display());
            let input = std::fs::File::open(entry.path()).unwrap();
            for pass in Pass::iter() {
                for executor in Executor::iter() {
                    let result = pass.execute(&executor, input.try_clone().unwrap());
                    println!("Pass: {} with {}", pass, executor);
                    println!("Load time: {:?}", result.loadtime);
                    println!("Runtime: {:?}", result.runtime);
                    println!("Write time: {:?}", result.writetime);
                }
            }
        }
    }
}
