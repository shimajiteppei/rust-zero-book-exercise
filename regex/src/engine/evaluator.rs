use std::{
    collections::{HashSet, VecDeque},
    error::Error,
    fmt::Display,
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

#[derive(Debug, PartialEq, Eq)]
struct RegisterContext {
    /// program counter
    pc: usize,
    /// string pointer
    sp: usize,
}

impl RegisterContext {
    #[inline]
    fn calculate_hash(&self) -> u64 {
        (self.sp as u64) << 32 | (self.pc as u64)
    }

    #[inline]
    fn incr_pc(&mut self) -> Result<(), EvalError> {
        match self.pc.checked_add(1) {
            Some(res) => self.pc = res,
            None => return Err(EvalError::PCOverFlow),
        }
        Ok(())
    }

    #[inline]
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
    Continue(Option<(RegisterContext, RegisterContext)>),
}

impl Instruction {
    #[inline]
    fn eval_inst<F>(
        self,
        line: &[char],
        ctx: &mut RegisterContext,
        split_fn: F,
    ) -> Result<MatchStatus, EvalError>
    where
        F: FnOnce(RegisterContext, RegisterContext) -> Result<MatchStatus, EvalError>,
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

#[inline]
fn exact_eval(
    inst: &[Instruction],
    line: &[char],
    init_sp: usize,
    is_depth: bool,
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
            Ok(MatchStatus::Continue(Some((reg1, reg2))))
        })?;

        match status {
            MatchStatus::Success => return Ok(true),
            MatchStatus::Failed => {}
            MatchStatus::Continue(it) => match it {
                Some((ctx1, ctx2)) => {
                    if is_depth {
                        if ctx_set.insert(ctx2.calculate_hash()) {
                            ctx_queue.push_front(ctx2);
                        }
                        if ctx_set.insert(ctx1.calculate_hash()) {
                            ctx_queue.push_front(ctx1);
                        }
                    } else {
                        if ctx_set.insert(ctx1.calculate_hash()) {
                            ctx_queue.push_back(ctx1);
                        }
                        if ctx_set.insert(ctx2.calculate_hash()) {
                            ctx_queue.push_back(ctx2);
                        }
                    }
                }
                None => {
                    if ctx_set.insert(ctx.calculate_hash()) {
                        if is_depth {
                            ctx_queue.push_front(ctx);
                        } else {
                            ctx_queue.push_back(ctx);
                        }
                    }
                }
            },
        };
    }
}

pub fn eval(inst: &[Instruction], line: &[char], is_depth: bool) -> Result<bool, EvalError> {
    for (i, _) in line.iter().enumerate() {
        if exact_eval(inst, line, i, is_depth)? {
            return Ok(true);
        }
    }
    Ok(false)
}
