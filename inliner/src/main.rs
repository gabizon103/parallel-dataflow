use std::path::PathBuf;

use argh::FromArgs;
use inliner::Inliner;
use lesson2::{form_blocks_from_read, CFGProgram, ControlFlow, FlatProgram};
use utils::cli::read_input;

/// inliner
#[derive(FromArgs)]
pub struct InlinerOpts {
    /// input bril file
    #[argh(positional)]
    pub input: Option<PathBuf>,
    /// output directory for graphs
    #[argh(option, short = 'o')]
    pub output: Option<PathBuf>,
}

pub fn main() {
    let opts: InlinerOpts = argh::from_env();
    let input = opts.input;
    let output = opts.output;
    let input = read_input(input);

    let (all_blocks, _) = form_blocks_from_read(input);
    let cfgs: Vec<ControlFlow> = all_blocks
        .into_iter()
        .map(|(name, blocks, map, args, ret_type)| {
            let mut cfg = ControlFlow::new(name, blocks, map, args, ret_type);
            cfg.build();
            cfg
        })
        .collect();

    let mut program = CFGProgram { functions: cfgs };

    let mut inliner = Inliner::new(&mut program, 0);
    let _ = inliner.run_pass();

    let program_flat = program.flatten_blocks();

    let flat_program = FlatProgram {
        functions: program_flat,
    };
    let program_str = serde_json::to_string(&flat_program).unwrap();
    println!("{program_str}")
}
