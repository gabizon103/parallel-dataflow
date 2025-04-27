mod mutex_worklist;
use lesson2::ControlFlow;
use lesson4::{Analysis, Opts, framework::AnalysisFramework, log_time};
use mutex_worklist::MutexFixpoint;
use std::collections::{HashMap, HashSet};
use utils::{cfg::form_blocks_from_read, cli::read_input};

pub fn run(opts: Opts) {
    let input = opts.input;
    let analysis = &opts.analysis;
    let input = read_input(input);
    let suppress_output = opts.supress_output;

    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Stderr)
        .init();

    let (all_blocks, _) = form_blocks_from_read(input);
    let cfgs: Vec<ControlFlow> = all_blocks
        .into_iter()
        .map(|(name, blocks, map, args)| {
            let mut cfg = ControlFlow::new(name, blocks, map, args);
            cfg.build();
            cfg
        })
        .collect();

    cfgs.into_iter().for_each(|cfg| {
        match analysis {
            Analysis::ReachingDefsGeneric(rd) => {
                let mut framework: AnalysisFramework<HashMap<String, HashSet<(usize, usize)>>> =
                    AnalysisFramework::new(cfg, HashMap::new());

                // this is the real computation
                log_time!(MutexFixpoint::worklist(&mut framework, rd), "reaching-defs");
                // log_time!(framework.worklist(analysis);)
                // log_time!(framework.worklist(rd), "reaching-defs");

                if !suppress_output {
                    println!("{:#?}", framework);
                }
            }
            Analysis::LiveVars(lv) => {
                let mut framework: AnalysisFramework<HashSet<String>> =
                    AnalysisFramework::new(cfg, HashSet::new());

                log_time!(framework.worklist(lv), "live-vars");

                if !suppress_output {
                    println!("{:#?}", framework);
                }
            }
        }
    });
}

fn main() {
    let opts: Opts = argh::from_env();
    run(opts)
}
