use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Error,
};

use bril_rs::{EffectOps, Instruction};
use lesson2::{BasicBlock, CFGProgram, ControlFlow};
use utils::cfg::{get_dest, get_uses, is_call, is_ret, set_dest};

#[derive(Clone, Eq, PartialEq, Hash, Debug, Copy)]
pub struct InstrLoc {
    pub cfg_id: usize,
    pub blk_id: usize,
    pub instr_id: usize,
}

impl InstrLoc {
    pub fn new(cfg_id: usize, blk_id: usize, instr_id: usize) -> Self {
        Self {
            cfg_id,
            blk_id,
            instr_id,
        }
    }
}

pub struct Inliner<'a> {
    program: &'a mut CFGProgram,
    iter: u32,
    paths: HashSet<Vec<InstrLoc>>,
}

impl<'a> Inliner<'a> {
    pub fn new(program: &'a mut CFGProgram) -> Self {
        Self {
            program,
            iter: 0,
            paths: HashSet::new(),
        }
    }

    pub fn is_call(&self, instr: &InstrLoc) -> bool {
        let instr = &self.program.functions[instr.cfg_id].blocks[instr.blk_id].instrs[instr.instr_id];
        is_call(instr)
    }

    // Finds the function given a name
    fn find_cfg(&self, name: &String) -> Result<usize, Error> {
        for (idx, cfg) in self.program.functions.iter().enumerate() {
            if cfg.name == *name {
                return Ok(idx);
            }
        }
        panic!("Function {name} not found")
    }

    pub fn get_calls(func: &ControlFlow, func_id: usize) -> Vec<InstrLoc> {
        let mut calls = vec![];
        func.blocks.iter().enumerate().for_each(|(blk_idx, blk)| {
            blk.instrs
                .iter()
                .enumerate()
                .for_each(|(instr_idx, instr)| {
                    if is_call(instr) {
                        let instrloc = InstrLoc {
                            cfg_id: func_id,
                            blk_id: blk_idx,
                            instr_id: instr_idx,
                        };
                        calls.push(instrloc);
                    }
                });
        });

        calls
    }

    // gets the name of the function being called by `caller`
    pub fn get_callee_name(&self, caller: InstrLoc) -> Result<String, Error> {
        let call_blk = &self.program.functions[caller.cfg_id].blocks[caller.blk_id];

        let call_instr = &call_blk.instrs[caller.instr_id];
        let call_targ = match call_instr {
            bril_rs::Instruction::Constant { .. } => {
                panic!("Instruction {call_instr} is not a call")
            }
            bril_rs::Instruction::Value { op, funcs, .. } => {
                if matches!(op, bril_rs::ValueOps::Call) {
                    Ok(funcs[0].to_string())
                } else {
                    panic!("Instruction {call_instr} is not a call");
                }
            }
            bril_rs::Instruction::Effect { op, funcs, .. } => {
                if matches!(op, bril_rs::EffectOps::Call) {
                    Ok(funcs[0].to_string())
                } else {
                    panic!("Instruction {call_instr} is not a call")
                }
            }
        };

        call_targ
    }

    // gets the name of the function that `caller` is in
    pub fn get_caller_src(&self, caller: InstrLoc) -> String {
        let cfg_id = caller.cfg_id;
        self.program.functions[cfg_id].name.clone()
    }

    // traverses from a call to a leaf function and accumulates the path
    pub fn traverse_call_path(
        &mut self,
        caller: InstrLoc,
        cur_path: &mut Vec<InstrLoc>,
        visited: &mut HashSet<InstrLoc>,
    ) {
        let callee_name = self.get_callee_name(caller).expect("oops");
        let callee_cfg_idx = self.find_cfg(&callee_name).expect("oops");
        let callee_cfg = &self.program.functions[callee_cfg_idx];

        visited.insert(caller);
        // println!("visited: {:#?}", visited);
        cur_path.push(caller);

        // check if the last three calls are all recursive
        let last_idx = cur_path.len() - 1;
        let threshold = if last_idx >= 2 {
            last_idx - 2
        } else {
            usize::MAX
        };
        let is_recursive: bool = cur_path
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx >= last_idx)
            .map(|(_, instrloc)| {
                let caller_src = self.get_caller_src(caller);
                // println!("caller_src: {caller_src}");
                let callee_name = self.get_callee_name(*instrloc).expect("expected a call");
                // println!("callee_name: {callee_name}");
                caller_src == callee_name
            })
            .fold(true, |acc, b| acc && b);

        let calls_from_here = Self::get_calls(callee_cfg, callee_cfg_idx);
        if calls_from_here.is_empty() || is_recursive {
            self.paths.insert(cur_path.to_vec());
        } else {
            calls_from_here.iter().for_each(|instrloc| {
                if !visited.contains(instrloc) {
                    self.traverse_call_path(instrloc.clone(), cur_path, visited);
                }
            });
        };
    }

    pub fn run_pass(&mut self) -> Result<(), Error> {
        let main_idx = self.find_cfg(&"main".to_string())?;
        // println!("main_idx: {main_idx}");
        let main = &self.program.functions[main_idx].clone();
        let mut calls: VecDeque<InstrLoc> = Self::get_calls(main, main_idx).into();

        // while !calls.is_empty() {
        //     // println!("entering while: {:#?}", calls);
        //     let this_call = calls.pop_front().unwrap();
        //     let mut cur_path = vec![];
        //     let visited = &mut HashSet::new();
        //     self.traverse_call_path(this_call.clone(), &mut cur_path, visited);
        //     let all_paths = self.paths.clone();
        //     // println!("got all paths: {:#?}", all_paths);
        //     all_paths.iter().for_each(|path| {
        //         let rev_path: Vec<InstrLoc> = path.iter().copied().rev().collect();
        //         rev_path.iter().for_each(|caller| {
        //             // println!("inlining {:#?}", caller);
        //             let _ = self.inline_call(caller);
        //             println!("finished");
        //         });
        //     });
        //     let main = &self.program.functions[main_idx].clone();
        //     // println!("main: {:#?}", main.blocks);
        //     calls = Self::get_calls(main, main_idx).into();
        // }

        // let this_call = calls.pop_front().unwrap();
        // let mut cur_path = vec![];
        // let visited = &mut HashSet::new();
        // self.traverse_call_path(this_call.clone(), &mut cur_path, visited);
        // let all_paths = self.paths.clone();
        // // println!("got all paths: {:#?}", all_paths);
        // all_paths.iter().for_each(|path| {
        //     let rev_path: Vec<InstrLoc> = path.iter().copied().rev().collect();
        //     rev_path.iter().for_each(|caller| {
        //         if self.is_call(caller) {
        //             // println!("inlining {:#?}", caller);
        //             let _ = self.inline_call(caller);
        //             // println!("finished");
        //         }
        //     });
        // });

        let mut funcs = self.program.functions.clone();
        for i in 0..2 {
            // println!("outer loop iter {i}");
            funcs.iter().enumerate().for_each(|(func_id, func)| {
                let calls = Self::get_calls(func, func_id);
                calls.iter().rev().for_each(|caller| {
                    if self.is_call(caller) {
                        // println!("inlining {:#?}", caller);
                        let _ = self.inline_call(caller);
                    }
                });
            });
            funcs = self.program.functions.clone();
        }



        Ok(())
    }

    pub fn inline_call(&mut self, caller: &InstrLoc) -> Result<(), Error> {
        // steps for inlining a call:
        // 1. rename labels and variables
        // 2. add all the blocks to the caller
        // 3. add assignments to args
        // 4. add a jmp to the first block

        let call_blk = &mut self.program.functions[caller.cfg_id].blocks[caller.blk_id];
        let inline_ret_instrs = call_blk.instrs.split_off(caller.instr_id + 1);

        let call_instr = &mut call_blk.instrs[caller.instr_id];
        let call_dest = get_dest(call_instr);
        let call_targ = match call_instr {
            bril_rs::Instruction::Constant { .. } => {
                panic!("Instruction {call_instr} is not a call")
            }
            bril_rs::Instruction::Value { op, funcs, .. } => {
                if matches!(op, bril_rs::ValueOps::Call) {
                    Ok(funcs[0].to_string())
                } else {
                    panic!("Instruction {call_instr} is not a call");
                }
            }
            bril_rs::Instruction::Effect { op, funcs, .. } => {
                if matches!(op, bril_rs::EffectOps::Call) {
                    Ok(funcs[0].to_string())
                } else {
                    panic!("Instruction {call_instr} is not a call")
                }
            }
        }?;

        // where we jmp to after the inlined func
        let mut inline_ret_blk = BasicBlock::default();
        inline_ret_blk.name = format!("inline_ret_{call_targ}_{}", self.iter);
        inline_ret_blk.instrs = inline_ret_instrs;

        let call_args = match call_instr {
            bril_rs::Instruction::Constant { .. } => {
                panic!("Instruction {call_instr} is not a call")
            }
            bril_rs::Instruction::Value { op, args, .. } => {
                if matches!(op, bril_rs::ValueOps::Call) {
                    Ok(args.clone())
                } else {
                    panic!("Instruction {call_instr} is not a call")
                }
            }
            bril_rs::Instruction::Effect { op, args, .. } => {
                if matches!(op, bril_rs::EffectOps::Call) {
                    Ok(args.clone())
                } else {
                    panic!("Instruction {call_instr} is not a call")
                }
            }
        }?;

        let callee_idx = self.find_cfg(&call_targ)?;
        let callee_cfg = &self.program.functions[callee_idx];
        let callee_args = callee_cfg.args.clone();

        // stitch callee cfg into caller cfg at the point of the call

        let mut this_map = HashMap::new();

        // 1. get the blocks of the callee and generate new names
        let mut callee_blocks = callee_cfg.blocks.clone();
        callee_blocks.iter_mut().for_each(|blk| {
            blk.name = format!("{}_inlined_{}", blk.name, self.iter);
            blk.instrs.iter_mut().for_each(|instr| {
                if let Some(dest) = get_dest(instr) {
                    let new_dest = format!("{dest}_inlined_{}", self.iter);
                    set_dest(instr, new_dest.clone());
                    this_map.insert(dest, new_dest);
                } else {
                    // rename labels
                    match instr {
                        Instruction::Constant { .. } | Instruction::Value { .. } => (),
                        Instruction::Effect { labels, op, .. } => {
                            let new_labels = labels
                                .iter()
                                .map(|label| format!("{label}_inlined_{}", self.iter))
                                .collect();
                            *labels = new_labels;
                        }
                    }
                }
            })
        });

        let caller_cfg = &mut self.program.functions[caller.cfg_id];
        let start_of_stitching = caller_cfg.blocks.len();

        // 3. add assignments to args
        let mut prepend_blk = BasicBlock::default();
        prepend_blk.name = format!("prelog_{call_targ}_{}", self.iter);
        let mut prelog: Vec<Instruction> = call_args
            .iter()
            .zip(callee_args.iter())
            .map(|(item, arg)| {
                let new_name = format!("{}_inlined_{}", arg.name, self.iter);
                this_map.insert(arg.name.clone(), new_name.clone());

                Instruction::Value {
                    args: vec![item.clone()],
                    dest: new_name,
                    funcs: vec![],
                    labels: vec![],
                    op: bril_rs::ValueOps::Id,
                    pos: None,
                    op_type: arg.arg_type.clone(),
                }
            })
            .collect();
        prelog.extend(prepend_blk.instrs.clone());
        prepend_blk.instrs = prelog;
        caller_cfg.blocks.push(prepend_blk);

        // rename uses
        callee_blocks.iter_mut().for_each(|blk| {
            blk.instrs.iter_mut().for_each(|instr| {
                match instr {
                    Instruction::Constant { .. } => (),
                    Instruction::Value { args, .. } | Instruction::Effect { args, .. } => {
                        let new_args: Vec<String> = args
                            .iter()
                            .map(|arg| {
                                if let Some(renamed_arg) = this_map.get(arg) {
                                    renamed_arg.to_string()
                                } else {
                                    arg.to_string()
                                }
                            })
                            .collect();
                        *args = new_args;
                    }
                };
            })
        });

        // 2. add blocks to the caller

        let blk_w_inlining = caller.blk_id;

        // replace rets with assignments and jmps
        // (y = call foo; ... ret b) -> (y = b; jmp)
        // (call foo; ... ret) -> jmp
        let new_callee_blocks = callee_blocks.iter().map(|blk| {
            let mut new_blk = BasicBlock::default();
            new_blk.name = blk.name.clone();

            blk.instrs.iter().for_each(|instr| {
                if is_ret(instr) {
                    if let Some(dest) = &call_dest {
                        let op_type = match instr {
                            Instruction::Constant { .. } => panic!("ahh"),
                            Instruction::Value { op_type, .. } => op_type,
                            Instruction::Effect { .. } => &bril_rs::program::Type::Int, // TODO
                        };

                        let args = get_uses(instr);
                        let assign_ret = Instruction::Value {
                            args: vec![args[0].clone()],
                            dest: dest.clone(),
                            funcs: vec![],
                            labels: vec![],
                            op: bril_rs::ValueOps::Id,
                            pos: None,
                            op_type: op_type.clone(),
                        };
                        new_blk.instrs.push(assign_ret);
                    };
                    let jmp = Instruction::Effect {
                        args: vec![],
                        funcs: vec![],
                        labels: vec![format!("inline_ret_{call_targ}_{}", self.iter)],
                        op: EffectOps::Jump,
                        pos: None,
                    };
                    new_blk.instrs.push(jmp);
                } else {
                    new_blk.instrs.push(instr.clone());
                }
            });

            new_blk
        });

        caller_cfg.blocks.extend(new_callee_blocks);
        caller_cfg.blocks.push(inline_ret_blk); // also push the return blk

        // 4. replace call with a jmp
        let start_name = caller_cfg.blocks[start_of_stitching].name.clone();
        let jmp_targ = format!("{start_name}");
        let jmp = Instruction::Effect {
            args: vec![],
            funcs: vec![],
            labels: vec![jmp_targ], // lbl we jmp to
            op: EffectOps::Jump,
            pos: None,
        };
        self.program.functions[caller.cfg_id].blocks[caller.blk_id].instrs[caller.instr_id] = jmp;
        self.iter += 1;

        Ok(())
    }
}
