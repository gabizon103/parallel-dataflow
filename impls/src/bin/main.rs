use argh::FromArgs;
use impls::{Executor, Pass};
use simple_logger::SimpleLogger;

#[derive(FromArgs)]
/// Run a pass with a specified executor
pub struct Args {
    /// the log level
    #[argh(option, short = 'l', default = "log::LevelFilter::Warn")]
    log: log::LevelFilter,
    /// the executor to use
    #[argh(option, short = 'a', default = "Executor::Sequential")]
    algorithm: Executor,
    /// the pass to run
    #[argh(option, short = 'p')]
    pass: Pass,
}

fn fmt_duration(duration: std::time::Duration) -> String {
    if duration.as_millis() > 100 {
        format!("{}ms", duration.as_millis())
    } else if duration.as_micros() > 100 {
        format!("{}us", duration.as_micros())
    } else {
        format!("{}ns", duration.as_nanos())
    }
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

    println!("Load time: {}", fmt_duration(result.loadtime));
    println!("Runtime: {}", fmt_duration(result.runtime));
    println!("Write time: {}", fmt_duration(result.writetime));
}
