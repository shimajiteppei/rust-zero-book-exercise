use std::{error::Error, fmt::Display};

use super::Instruction;

#[derive(Debug)]
pub enum EvalError {
    PCOverFlow,
    SPOverFlow,
    InvalidPC,
    InvalidContext,
}

impl Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EvalError: {:?}", self)
    }
}

impl Error for EvalError {}

fn eval_depth(
    inst: &[Instruction],
    line: &[char],
    mut pc: usize,
    mut sp: usize,
) -> Result<bool, EvalError> {
    loop {
        let next = match inst.get(pc) {
            Some(i) => i,
            None => return Err(EvalError::InvalidPC),
        };

        match next {
            Instruction::Char(c) => match line.get(sp) {
                Some(sp_c) => {
                    if c == sp_c {
                        match pc.checked_add(1) {
                            Some(res) => pc = res,
                            None => return Err(EvalError::PCOverFlow),
                        };
                        match sp.checked_add(1) {
                            Some(res) => sp = res,
                            None => return Err(EvalError::SPOverFlow),
                        };
                    } else {
                        return Ok(false);
                    }
                }
                None => return Ok(false),
            },
            Instruction::Match => return Ok(true),
            Instruction::Jump(addr) => pc = *addr,
            Instruction::Split(addr1, addr2) => {
                if eval_depth(inst, line, *addr1, sp)? || eval_depth(inst, line, *addr2, sp)? {
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            }
        }
    }
}

fn eval_width(_inst: &[Instruction], _line: &[char]) -> Result<bool, EvalError> {
    todo!()
}

pub fn eval(inst: &[Instruction], line: &[char], is_depth: bool) -> Result<bool, EvalError> {
    if is_depth {
        eval_depth(inst, line, 0, 0)
    } else {
        eval_width(inst, line)
    }
}
