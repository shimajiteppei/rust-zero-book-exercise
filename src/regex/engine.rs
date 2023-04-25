use std::{error::Error, fmt::Display};

mod codegen;
mod evaluator;
mod parser;

#[derive(Debug)]
pub enum Instruction {
    Char(char),
    Match,
    Jump(usize),
    Split(usize, usize),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Char(c) => write!(f, "char {c}"),
            Instruction::Match => write!(f, "match"),
            Instruction::Jump(addr) => write!(f, "jump {:>04}", addr),
            Instruction::Split(addr1, addr2) => write!(f, "split {:>04} {:>04}", addr1, addr2),
        }
    }
}

pub type DynError = Box<dyn Error + 'static>;

pub fn do_matching(expr: &str, line: &str, is_depth: bool) -> Result<bool, DynError> {
    let ast = parser::parse(expr)?;
    let code = codegen::get_code(&ast)?;
    let line = line.chars().collect::<Vec<char>>();
    Ok(evaluator::eval(&code, &line, is_depth)?)
}

#[cfg(test)]
mod tests {
    use crate::regex::engine::do_matching;

    #[test]
    fn test_matching() {
        assert!(do_matching("+b", "bbb", true).is_err());
        assert!(do_matching("*b", "bbb", true).is_err());
        assert!(do_matching("|b", "bbb", true).is_err());
        assert!(do_matching("?b", "bbb", true).is_err());

        assert!(do_matching("abc", "abc", true).unwrap());
        assert!(do_matching("abc|def", "def", true).unwrap());
        assert!(do_matching("(abc)*", "abcabcabc", true).unwrap());
        assert!(do_matching("(ab|cd)+", "abcdcd", true).unwrap());
        assert!(do_matching("a(bc)?", "a", true).unwrap());

        assert!(!do_matching("abc|def", "bcd", true).unwrap());
        assert!(!do_matching("abc?", "ac", true).unwrap());
    }
}
