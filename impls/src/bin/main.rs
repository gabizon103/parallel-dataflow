use argh::FromArgs;
use impls::{Executor, Pass};
use simple_logger::SimpleLogger;

#[derive(FromArgs)]
/// Run a pass with a specified executor
pub struct Args {
    /// the log level
    #[argh(option, short = 'l', default = "log::LevelFilter::Info")]
    log: log::LevelFilter,
    /// the executor to use
    #[argh(option, short = 'a', default = "Executor::Sequential")]
    algorithm: Executor,
    /// the pass to run
    #[argh(option, short = 'p')]
    pass: Pass,
}

fn main() {
    let args: Args = argh::from_env();

    SimpleLogger::new()
        .with_colors(true)
        .with_level(args.log)
        .without_timestamps()
        .init()
        .unwrap();

    let result = args.pass.execute(&args.algorithm, std::io::stdin().lock());

    println!("{}", result.result);

    println!("Load time: {:?}", result.loadtime);
    println!("Runtime: {:?}", result.runtime);
    println!("Write time: {:?}", result.writetime);
}
