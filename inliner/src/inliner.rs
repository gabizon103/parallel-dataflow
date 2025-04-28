use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Error,
};

use bril_rs::{EffectOps, Instruction};
use itertools::Itertools;
use lesson2::{BasicBlock, CFGProgram, ControlFlow};
use multiset::HashMultiSet;
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
    // map calls to maps of var renamings
    rename_map: HashMap<InstrLoc, HashMap<String, String>>,
    new_program: CFGProgram,
}

impl<'a> Inliner<'a> {
    pub fn new(program: &'a mut CFGProgram) -> Self {
        Self {
            program,
            iter: 0,
            paths: HashSet::new(),
            rename_map: HashMap::new(),
            new_program: CFGProgram { functions: Vec::new() },
        }
    }

    pub fn is_call(&self, instr: &InstrLoc) -> bool {
        let instr =
            &self.program.functions[instr.cfg_id].blocks[instr.blk_id].instrs[instr.instr_id];
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
    pub fn get_callee_name(&self, caller: &InstrLoc) -> Result<String, Error> {
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

    fn set_contains_name(&self, set: &HashMultiSet<InstrLoc>, caller: &InstrLoc) -> usize {
        let caller_func_name = &self.program.functions[caller.cfg_id].name;
        let set_named: HashMultiSet<String> = set
            .iter()
            .map(|set_loc| {
                let name = &self.program.functions[set_loc.cfg_id].name;
                name.clone()
            })
            .collect();
        let count = set_named.count_of(caller_func_name);
        count
    }

    fn named_path(&self, path: &Vec<InstrLoc>) -> Vec<String> {
        path.iter().map(|instrloc| {
            self.program.functions[instrloc.cfg_id].name.clone()
        }).collect()
    }

    // traverses from a call to a leaf function and accumulates the path
    // a dfs where the nodes are callers
    pub fn traverse_call_path(
        &mut self,
        caller: &InstrLoc,
        cur_path: &mut Vec<InstrLoc>,
        visited: &mut HashMultiSet<InstrLoc>,
    ) {
        let callee_name = self.get_callee_name(caller).expect("oops");
        let callee_cfg_idx = self.find_cfg(&callee_name).expect("oops");
        let callee_cfg = &self.program.functions[callee_cfg_idx];

        // set of visited calls
        visited.insert(*caller);

        // let last_idx = cur_path.len() - 1;
        // let is_recursive: bool = cur_path
        //     .iter()
        //     .enumerate()
        //     .filter(|(idx, _)| *idx >= last_idx)
        //     .map(|(_, instrloc)| {
        //         let caller_src = self.get_caller_src(*caller);
        //         let callee_name = self.get_callee_name(instrloc).expect("expected a call");
        //         caller_src == callee_name
        //     })
        //     .fold(true, |acc, b| acc && b);

        let is_recursive = self.get_caller_src(*caller) == self.get_callee_name(caller).expect("");
        let caller_src = self.get_caller_src(*caller);
        let callee_name = self.get_callee_name(caller).expect("");

        let calls_from_callee = Self::get_calls(callee_cfg, callee_cfg_idx);
        let calls_filtered: Vec<&InstrLoc> = calls_from_callee.iter().filter(|call| {
            self.set_contains_name(visited, call) <= 2
        }).collect();
        let all_calls_exceed = calls_from_callee.iter().map(|call| {
            self.set_contains_name(visited, call) > 2
        }).fold(true, |acc, b| acc && b);

        // println!("calls_filtered: {:#?}", calls_filtered);
        if calls_filtered.is_empty() {
            self.paths.insert(cur_path.to_vec());
        } else {
            cur_path.push(*caller);

            calls_from_callee.iter().for_each(|instrloc| {
                // if recursive call, check that depth isnt exceeded
                if self.set_contains_name(visited, instrloc) <= 2 {
                    self.traverse_call_path(&instrloc, cur_path, visited);
                }
            });
        };
    }

    pub fn run_pass(&mut self) -> Result<(), Error> {
        let main_idx = self.find_cfg(&"main".to_string())?;
        // println!("main_idx: {main_idx}");
        let main = &self.program.functions[main_idx].clone();

        let calls: VecDeque<InstrLoc> = Self::get_calls(main, main_idx).into();
        // let mut cur_path = vec![];
        // let visited = &mut HashMultiSet::new();

        // no calls in main
        if calls.is_empty() {
            return Ok(());
        }

        calls.iter().for_each(|call| {
            let mut cur_path = vec![];
            let visited = &mut HashMultiSet::new();
            self.traverse_call_path(call, &mut cur_path, visited);
        });

        // println!("{:#?}", self.paths);

        let init_paths = self.paths.clone();
        let mut paths_as_vec: Vec<Vec<InstrLoc>> = self.paths.clone().into_iter().collect();
        
        let named_paths: Vec<Vec<String>> = paths_as_vec.iter().map(|path| {
            self.named_path(path)
        }).collect();
        println!("{:#?}", named_paths);
        
        while !paths_as_vec.is_empty() {
            let mut this_path = paths_as_vec.pop().unwrap();
            let this_path_named: Vec<String> = self.named_path(&this_path);

            this_path.iter().rev().for_each(|caller| {
                let _ = self.inline_call(caller);
            });
            // println!("inlined path {:#?}", this_path_named);

            // did inlining, now need to reconstruct call graph thingy
            let main = &self.program.functions[main_idx];
            let calls = Self::get_calls(main, main_idx);
            calls.iter().for_each(|call| {
                self.paths = HashSet::new();
                let mut cur_path = vec![];
                let visited = &mut HashMultiSet::new();
                self.traverse_call_path(call, &mut cur_path, visited);
            });
            paths_as_vec = self.paths.clone().into_iter().filter(|path| {
                let named = self.named_path(path);
                named != this_path_named && init_paths.contains(path)
            }).collect();
            // println!("{}", paths_as_vec.len())
        }

        // while !calls.is_empty() {
        //     // println!("entering while: {:#?}", calls);
        //     let this_call = calls.pop_front().unwrap();
        //     let mut cur_path = vec![];
        //     let visited = &mut HashSet::new();
        //     self.traverse_call_path(&this_call, &mut cur_path, visited);
        //     println!("paths: {:#?}", self.paths);
        //     let all_paths = self.paths.clone();
        //     // println!("got all paths: {:#?}", all_paths);
        //     all_paths.iter().for_each(|path| {
        //         let rev_path: Vec<InstrLoc> = path.iter().copied().rev().collect();
        //         rev_path.iter().for_each(|caller| {
        //             // println!("inlining {:#?}", caller);
        //             let _ = self.inline_call(caller);
        //             // println!("finished");
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

        // let mut funcs = self.program.functions.clone();
        // for i in 0..1 {
        //     // println!("outer loop iter {i}");
        //     funcs.iter().enumerate().for_each(|(func_id, func)| {
        //         let calls = Self::get_calls(func, func_id);
        //         calls.iter().rev().for_each(|caller| {
        //             if self.is_call(caller) {
        //                 // println!("inlining {:#?}", caller);
        //                 let instr = &funcs[caller.cfg_id].blocks[caller.blk_id].instrs[caller.instr_id];
        //                 println!("inlining call {instr} in {}", &funcs[caller.cfg_id].name);
        //                 let _ = self.inline_call(caller);
        //             }
        //         });
        //     });
        //     funcs = self.program.functions.clone();
        // }

        Ok(())
    }

    fn is_jump(instr: &Instruction) -> bool {
        match instr {
            Instruction::Constant { .. } => false,
            Instruction::Value { .. } => false,
            Instruction::Effect { op, .. } => matches!(op, EffectOps::Jump),
        }
    }

    pub fn inline_call(&mut self, caller: &InstrLoc) -> Result<(), Error> {
        // steps for inlining a call:
        // 1. rename labels and variables
        // 2. add all the blocks to the caller
        // 3. add assignments to args
        // 4. add a jmp to the first block
        // println!("call is in {}", self.program.functions[caller.cfg_id].name);

        // need to make ret blk jmp back to rest of program
        // first find the successor of the block the call is in, create a jmp the succ
        let caller_cfg = &self.program.functions[caller.cfg_id];
        let caller_cfg_name = caller_cfg.name.clone();

        let last_instr_call_blk = caller_cfg.blocks[caller.blk_id].instrs.last().unwrap();
        let succ = if Self::is_jump(last_instr_call_blk) {
            last_instr_call_blk.clone()
        } else {
            Instruction::Effect {
                args: vec![],
                funcs: vec![],
                labels: vec![],
                op: EffectOps::Return,
                pos: None,
            }
        };

        // let succ = match &caller_cfg.edges[caller.blk_id] {
        //     lesson2::Edge::Uncond(targ) => {
        //         let targ_blk_name = &caller_cfg.blocks[*targ].name;
        //         Instruction::Effect {
        //             args: vec![],
        //             funcs: vec![],
        //             labels: vec![targ_blk_name.clone()],
        //             op: EffectOps::Jump,
        //             pos: None,
        //         }
        //     },
        //     lesson2::Edge::Cond { true_targ, false_targ } => todo!(),
        //     lesson2::Edge::None => Instruction::Effect {
        //         args: vec![],
        //         funcs: vec![],
        //         labels: vec![],
        //         op: EffectOps::Return,
        //         pos: None
        //     },
        // };

        let call_blk = &mut self.program.functions[caller.cfg_id].blocks[caller.blk_id];
        let mut inline_ret_instrs = call_blk.instrs.split_off(caller.instr_id + 1);
        // add the jmp to succ to the ret blk
        inline_ret_instrs.push(succ);

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
                    Err(std::fmt::Error)
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
                    let new_dest = if let Some(call_dest) = &call_dest {
                        if *call_dest == dest {
                            dest.clone()
                        } else {
                            format!("{dest}_inlined_{}", self.iter)
                        }
                    } else {
                        format!("{dest}_inlined_{}", self.iter)
                    };
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
                                    // println!("found {arg}->{renamed_arg} in rename map");
                                    renamed_arg.to_string()
                                } else {
                                    // println!("didn't find {arg} in rename map");
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

        // replace rets with assignments and jmps
        // (y = call foo; ... ret b) -> (y = b; jmp)
        // (call foo; ... ret) -> jmp
        // println!("callee is {call_targ}");
        let old_blk_offset = caller_cfg.blocks.len();
        let new_callee_blocks = callee_blocks.iter().enumerate().map(|(blk_id, blk)| {
            let mut new_blk = BasicBlock::default();
            new_blk.name = blk.name.clone();
            let this_blk_id = old_blk_offset + blk_id;

            blk.instrs.iter().for_each(|instr| {
                if is_ret(instr) {
                    // println!("{instr} is a ret");
                    if let Some(dest) = &call_dest {
                        let op_type = match instr {
                            Instruction::Constant { .. } => panic!("ahh"),
                            Instruction::Value { op_type, .. } => op_type,
                            Instruction::Effect { .. } => &bril_rs::program::Type::Int, // TODO
                        };

                        let args = get_uses(instr);
                        if !args.is_empty() {
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
                        
                    };
                    let jmp = Instruction::Effect {
                        args: vec![],
                        funcs: vec![],
                        labels: vec![format!("inline_ret_{call_targ}_{}", self.iter)],
                        op: EffectOps::Jump,
                        pos: None,
                    };
                    caller_cfg.edges.push(lesson2::Edge::Uncond(this_blk_id));
                    new_blk.instrs.push(jmp);
                } else {
                    // println!("pushing {instr}");
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
        // caller_cfg.edges.insert(caller.blk_id, lesson2::Edge::Uncond(start_of_stitching));
        self.program.functions[caller.cfg_id].blocks[caller.blk_id].instrs[caller.instr_id] = jmp;
        self.iter += 1;

        Ok(())
    }
}
