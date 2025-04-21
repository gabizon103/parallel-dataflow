use std::{collections::{HashMap, HashSet, VecDeque}, fmt::Error};

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
    instr_threshold: u32,
    iter: u32,
    // func name -> (var name -> inlined var name)
    name_map: HashMap<String, HashMap<String, String>>
}

impl<'a> Inliner<'a> {
    pub fn new(program: &'a mut CFGProgram, instr_threshold: u32) -> Self {
        Self {
            program,
            instr_threshold,
            iter: 0,
            name_map: HashMap::new()
        }
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
            },
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


    // traverses from a call to a leaf function and accumulates the path
    pub fn traverse_call_path(&self, caller: InstrLoc, cur_path: Vec<InstrLoc>, paths: &mut HashSet<Vec<InstrLoc>>) -> Result<HashSet<Vec<InstrLoc>>, Error> {
        let callee_name = self.get_callee_name(caller)?;
        let callee_cfg_idx = self.find_cfg(&callee_name)?;
        let callee_cfg = &self.program.functions[callee_cfg_idx];
        let mut new_paths = paths.clone();

        let calls_from_here = Self::get_calls(callee_cfg, callee_cfg_idx);
        if calls_from_here.is_empty() {
            new_paths.insert(cur_path);
        } else {
            calls_from_here.iter().for_each(|instrloc| {
                let mut this_path = cur_path.clone();
                this_path.push(instrloc.clone());
                new_paths = self.traverse_call_path(instrloc.clone(), this_path, paths).expect("oops");
            });
        };

        Ok(new_paths)
    }

    pub fn run_pass(&mut self) -> Result<(), Error> {
        let main_idx = self.find_cfg(&"main".to_string())?;
        let main = &self.program.functions[main_idx].clone();
        let mut calls: VecDeque<InstrLoc> = Self::get_calls(main, main_idx).into();

        while !calls.is_empty() {
            // println!("entering while: calls = {:#?}", calls);
            let this_call = calls.pop_front().unwrap();
            let cur_path = vec![this_call.clone()];
            let paths = &mut HashSet::new();
            let all_paths = self.traverse_call_path(this_call.clone(), cur_path, paths).expect("oops");
            all_paths.iter().for_each(|path| {
                let rev_path: Vec<InstrLoc> = path.iter().copied().rev().collect();
                rev_path.iter().for_each(|caller| {
                    // println!("inlining {:#?}", caller);
                    let _ = self.inline_call(caller);
                    // println!("finished");
                });
            });
            let main = &self.program.functions[main_idx].clone();
            calls = Self::get_calls(main, main_idx).into();
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
            },
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
            },
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
                            let new_labels = labels.iter().map(|label| {
                                format!("{label}_inlined_{}", self.iter)
                            }).collect();
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
                    Instruction::Value { args, .. } | Instruction::Effect { args, ..} => {
                        let new_args: Vec<String> = args.iter().map(|arg| {
                            if let Some(renamed_arg) = this_map.get(arg) {
                                renamed_arg.to_string()
                            } else {
                                arg.to_string()
                            }
                        }).collect();
                        *args = new_args;
                    },
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
