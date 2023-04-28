use std::{
    collections::{hash_map::DefaultHasher, HashSet, VecDeque},
    error::Error,
    fmt::Display,
    hash::{Hash, Hasher},
};

use super::codegen::Instruction;

#[derive(Debug)]
pub enum EvalError {
    PCOverFlow,
    SPOverFlow,
    InvalidPC,
}

impl Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EvalError: {:?}", self)
    }
}

impl Error for EvalError {}

#[derive(Debug, PartialEq, Eq, Hash)]
struct RegisterContext {
    /// program counter
    pc: usize,
    /// string pointer
    sp: usize,
}

impl RegisterContext {
    fn calculate_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }

    fn incr_pc(&mut self) -> Result<(), EvalError> {
        match self.pc.checked_add(1) {
            Some(res) => self.pc = res,
            None => return Err(EvalError::PCOverFlow),
        }
        Ok(())
    }

    fn incr_sp(&mut self) -> Result<(), EvalError> {
        match self.sp.checked_add(1) {
            Some(res) => self.sp = res,
            None => return Err(EvalError::SPOverFlow),
        }
        Ok(())
    }
}

enum MatchStatus {
    Success,
    Failed,
    Continue(Option<Vec<RegisterContext>>),
}

impl Instruction {
    #[inline(always)]
    fn eval_inst<F>(
        self,
        line: &[char],
        ctx: &mut RegisterContext,
        split_fn: F,
    ) -> Result<MatchStatus, EvalError>
    where
        F: Fn(RegisterContext, RegisterContext) -> Result<MatchStatus, EvalError>,
    {
        match self {
            Instruction::Char(c) => match line.get(ctx.sp) {
                Some(sp_c) => {
                    if c == *sp_c {
                        ctx.incr_pc()?;
                        ctx.incr_sp()?;
                    } else {
                        return Ok(MatchStatus::Failed);
                    }
                }
                None => return Ok(MatchStatus::Failed),
            },
            Instruction::AnyChar => match line.get(ctx.sp) {
                Some(_) => {
                    ctx.incr_pc()?;
                    ctx.incr_sp()?;
                }
                None => return Ok(MatchStatus::Failed),
            },
            Instruction::Match => return Ok(MatchStatus::Success),
            Instruction::Jump(addr) => ctx.pc = addr,
            Instruction::Split(addr1, addr2) => {
                return split_fn(
                    RegisterContext {
                        pc: addr1,
                        sp: ctx.sp,
                    },
                    RegisterContext {
                        pc: addr2,
                        sp: ctx.sp,
                    },
                );
            }
            Instruction::AssertHead => {
                if ctx.sp == 0 {
                    ctx.incr_pc()?;
                } else {
                    return Ok(MatchStatus::Failed);
                }
            }
            Instruction::AssertTail => {
                if ctx.sp == line.len() {
                    ctx.incr_pc()?;
                } else {
                    return Ok(MatchStatus::Failed);
                }
            }
        }
        Ok(MatchStatus::Continue(None))
    }
}

#[inline(always)]
fn exact_eval_depth(
    inst: &[Instruction],
    line: &[char],
    ctx: &mut RegisterContext,
) -> Result<bool, EvalError> {
    loop {
        let eval = match inst.get(ctx.pc) {
            Some(i) => *i,
            None => return Err(EvalError::InvalidPC),
        }
        .eval_inst(line, ctx, |mut reg1, mut reg2| {
            if exact_eval_depth(inst, line, &mut reg1)? || exact_eval_depth(inst, line, &mut reg2)?
            {
                Ok(MatchStatus::Success)
            } else {
                Ok(MatchStatus::Failed)
            }
        })?;

        match eval {
            MatchStatus::Success => return Ok(true),
            MatchStatus::Failed => return Ok(false),
            _ => {}
        }
    }
}

fn eval_depth(inst: &[Instruction], line: &[char]) -> Result<bool, EvalError> {
    for (i, _) in line.iter().enumerate() {
        let matched = exact_eval_depth(inst, line, &mut RegisterContext { pc: 0, sp: i })?;
        if matched {
            return Ok(true);
        }
    }
    Ok(false)
}

#[inline(always)]
fn exact_eval_width(
    inst: &[Instruction],
    line: &[char],
    init_sp: usize,
) -> Result<bool, EvalError> {
    let init_reg = RegisterContext { pc: 0, sp: init_sp };
    let init_reg_hash = init_reg.calculate_hash();
    let mut ctx_queue = VecDeque::from([init_reg]);
    let mut ctx_set = HashSet::from([init_reg_hash]);
    loop {
        let mut ctx = match ctx_queue.pop_front() {
            Some(it) => it,
            None => return Ok(false),
        };

        let status = match inst.get(ctx.pc) {
            Some(i) => *i,
            None => return Err(EvalError::InvalidPC),
        }
        .eval_inst(line, &mut ctx, |reg1, reg2| {
            Ok(MatchStatus::Continue(Some(vec![reg1, reg2])))
        })?;

        match status {
            MatchStatus::Success => return Ok(true),
            MatchStatus::Failed => {}
            MatchStatus::Continue(it) => match it {
                Some(it) => {
                    for ctx in it {
                        if ctx_set.insert(ctx.calculate_hash()) {
                            ctx_queue.push_back(ctx);
                        }
                    }
                }
                None => {
                    if ctx_set.insert(ctx.calculate_hash()) {
                        ctx_queue.push_back(ctx);
                    }
                }
            },
        };
    }
}

fn eval_width(inst: &[Instruction], line: &[char]) -> Result<bool, EvalError> {
    for (i, _) in line.iter().enumerate() {
        let matched = exact_eval_width(inst, line, i)?;
        if matched {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn eval(inst: &[Instruction], line: &[char], is_depth: bool) -> Result<bool, EvalError> {
    if is_depth {
        eval_depth(inst, line)
    } else {
        eval_width(inst, line)
    }
}
