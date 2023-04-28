use std::{error::Error, fmt::Display};

use super::parser::AST;

#[derive(Debug)]
pub enum CodeGenError {
    PCOverFlow,
    FailStar,
    FailOr,
    FailQuestion,
}

impl Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CodeGenError: {:?}", self)
    }
}

impl Error for CodeGenError {}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Char(char),
    Match,
    Jump(usize),
    Split(usize, usize),
    AnyChar,
    AssertHead,
    AssertTail,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Char(c) => write!(f, "char {c}"),
            Instruction::Match => write!(f, "match"),
            Instruction::Jump(addr) => write!(f, "jump {:>04}", addr),
            Instruction::Split(addr1, addr2) => write!(f, "split {:>04} {:>04}", addr1, addr2),
            Instruction::AnyChar => write!(f, "period"),
            Instruction::AssertHead => write!(f, "caret"),
            Instruction::AssertTail => write!(f, "dollar"),
        }
    }
}

#[derive(Default, Debug)]
struct Generator {
    pc: usize,
    insts: Vec<Instruction>,
}

impl Generator {
    fn inc_pc(&mut self) -> Result<(), CodeGenError> {
        match self.pc.checked_add(1) {
            Some(res) => {
                self.pc = res;
                Ok(())
            }
            None => Err(CodeGenError::PCOverFlow),
        }
    }

    fn gen_expr(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        match ast {
            AST::Char(c) => self.gen_single_inst(Instruction::Char(*c))?,
            AST::Plus(e) => self.gen_plus(e)?,
            AST::Star(e) => self.gen_star(e)?,
            AST::Question(e) => self.gen_question(e)?,
            AST::Or(e1, e2) => self.gen_or(e1, e2)?,
            AST::Seq(seq) => self.gen_seq(seq)?,
            AST::Period => self.gen_single_inst(Instruction::AnyChar)?,
            AST::Caret => self.gen_single_inst(Instruction::AssertHead)?,
            AST::Dollar => self.gen_single_inst(Instruction::AssertTail)?,
        };
        Ok(())
    }

    fn gen_single_inst(&mut self, inst: Instruction) -> Result<(), CodeGenError> {
        self.insts.push(inst);
        self.inc_pc()?;
        Ok(())
    }

    ///	L1: codes for e
    /// L2: split L1, L3
    /// L3:
    fn gen_plus(&mut self, e: &AST) -> Result<(), CodeGenError> {
        let l1 = self.pc;
        self.gen_expr(e)?;

        self.inc_pc()?;
        let l3 = self.pc;
        self.insts.push(Instruction::Split(l1, l3));

        Ok(())
    }

    /// L1: split L2, L4
    /// L2: codes for e
    /// L3: jmp L1
    /// L4:
    fn gen_star(&mut self, e: &AST) -> Result<(), CodeGenError> {
        let l1 = self.pc;
        self.inc_pc()?;
        let l2 = self.pc;
        self.insts.push(Instruction::Split(l2, 0));

        self.gen_expr(e)?;

        self.insts.push(Instruction::Jump(l1));
        self.inc_pc()?;
        match self.insts.get_mut(l1) {
            Some(Instruction::Split(_, l4)) => {
                *l4 = self.pc;
            }
            _ => return Err(CodeGenError::FailStar),
        }

        Ok(())
    }

    /// L1: split L2, L3
    /// L2: codes for e
    /// L3:
    fn gen_question(&mut self, e: &AST) -> Result<(), CodeGenError> {
        let l1 = self.pc;
        self.inc_pc()?;
        let l2 = self.pc;
        self.insts.push(Instruction::Split(l2, 0));

        self.gen_expr(e)?;
        match self.insts.get_mut(l1) {
            Some(Instruction::Split(_, l3)) => {
                *l3 = self.pc;
            }
            _ => return Err(CodeGenError::FailQuestion),
        }

        Ok(())
    }

    /// L1: split L2, L4
    /// L2: codes for e1
    /// L3: jmp L5
    /// L4: codes for e2
    /// L5:
    fn gen_or(&mut self, e1: &AST, e2: &AST) -> Result<(), CodeGenError> {
        let l1 = self.pc;
        self.inc_pc()?;
        let l2 = self.pc;
        self.insts.push(Instruction::Split(l2, 0));

        self.gen_expr(e1)?;

        let l3 = self.pc;
        self.insts.push(Instruction::Jump(0));
        self.inc_pc()?;
        match self.insts.get_mut(l1) {
            Some(Instruction::Split(_, l4)) => {
                *l4 = self.pc;
            }
            _ => return Err(CodeGenError::FailOr),
        }

        self.gen_expr(e2)?;
        match self.insts.get_mut(l3) {
            Some(Instruction::Jump(l5)) => {
                *l5 = self.pc;
            }
            _ => return Err(CodeGenError::FailOr),
        }

        Ok(())
    }

    fn gen_seq(&mut self, exprs: &[AST]) -> Result<(), CodeGenError> {
        for e in exprs {
            self.gen_expr(e)?;
        }
        Ok(())
    }

    fn gen_code(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        self.gen_expr(ast)?;
        self.inc_pc()?;
        self.insts.push(Instruction::Match);
        Ok(())
    }
}

pub fn get_code(ast: &AST) -> Result<Vec<Instruction>, CodeGenError> {
    let mut generaotr = Generator::default();
    generaotr.gen_code(ast)?;
    Ok(generaotr.insts)
}
