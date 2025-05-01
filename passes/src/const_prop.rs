use bril_utils::{BBFunction, BasicBlock, Foldable, HashableLiteral};
use std::{collections::HashMap, fmt::Display};
use utils::DataflowSpec;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Value {
    Const(HashableLiteral),
    Any,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Any => "T".fmt(f),
            Value::Const(l) => l.fmt(f),
        }
    }
}
#[derive(Default)]
pub struct ConstProp;

impl DataflowSpec for ConstProp {
    type Val = HashMap<String, Value>;

    fn init(&self, _: &BBFunction) -> Self::Val {
        HashMap::default()
    }

    fn meet(&self, in_vals: &[Self::Val]) -> Self::Val {
        let mut out_vals = HashMap::new();

        // For every key, if it has multiple different bindings, set it to Any
        // Otherwise, set it to the value
        for (name, bind) in in_vals.iter().flat_map(|v| v.iter()) {
            if let Some(v) = out_vals.get(name) {
                if v != bind {
                    out_vals.insert(name.clone(), Value::Any);
                }
            } else {
                out_vals.insert(name.clone(), bind.clone());
            }
        }

        out_vals
    }

    fn transfer(&self, block: &BasicBlock, in_val: &Self::Val) -> Self::Val {
        let mut out_vals = in_val.clone();

        for insn in block.iter() {
            if let Some((dest, val)) = insn.fold(|arg| {
                in_val.get(arg).and_then(|v| match v {
                    Value::Const(c) => Some(c.clone().into()),
                    Value::Any => None,
                })
            }) {
                out_vals.insert(
                    dest,
                    match val {
                        Some(v) => Value::Const(v.into()),
                        None => Value::Any,
                    },
                );
            }
        }

        out_vals
    }
}
