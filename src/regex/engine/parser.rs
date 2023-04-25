use std::{error::Error, fmt::Display, mem::take};

#[derive(Debug)]
pub enum AST {
    Char(char),
    Plus(Box<AST>),
    Star(Box<AST>),
    Question(Box<AST>),
    Or(Box<AST>, Box<AST>),
    Seq(Vec<AST>),
}

#[derive(Debug)]
pub enum ParseError {
    InvalidEscape(usize, char),
    InvalidRightParen(usize),
    NoPrev(usize),
    NoRightParen,
    Empty,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidEscape(pos, c) => {
                write!(f, "invalid escape: pos = {pos}, char = '{c}'")
            }
            ParseError::InvalidRightParen(pos) => {
                write!(f, "invalid right parenthesis: pos = {pos}")
            }
            ParseError::NoPrev(pos) => {
                write!(f, "no previous expression: pos = {pos}")
            }
            ParseError::NoRightParen => {
                write!(f, "no right parenthesis")
            }
            ParseError::Empty => {
                write!(f, "empty expression")
            }
        }
    }
}

impl Error for ParseError {}

fn fold_or(or_seq: Vec<AST>) -> Option<AST> {
    or_seq
        .into_iter()
        .rev()
        .reduce(|a, b| AST::Or(Box::new(a), Box::new(b)))
}

pub fn parse(expr: &str) -> Result<AST, ParseError> {
    #[derive(Default)]
    struct State {
        escape: bool,
        ast_seq: Vec<AST>,
        or_seq: Vec<AST>,
    }
    let mut state: State = State::default();
    let mut state_stack = Vec::new();

    for (i, c) in expr.chars().enumerate() {
        if state.escape {
            match c {
                '\\' | '(' | ')' | '|' | '+' | '*' | '?' => state.ast_seq.push(AST::Char(c)),
                _ => return Err(ParseError::InvalidEscape(i, c)),
            }
            state.escape = false;
        } else {
            match c {
                '+' => match state.ast_seq.pop() {
                    Some(prev) => state.ast_seq.push(AST::Plus(Box::new(prev))),
                    None => return Err(ParseError::NoPrev(i)),
                },
                '*' => match state.ast_seq.pop() {
                    Some(prev) => state.ast_seq.push(AST::Star(Box::new(prev))),
                    None => return Err(ParseError::NoPrev(i)),
                },
                '?' => match state.ast_seq.pop() {
                    Some(prev) => state.ast_seq.push(AST::Question(Box::new(prev))),
                    None => return Err(ParseError::NoPrev(i)),
                },
                '(' => {
                    state_stack.push(take(&mut state));
                }
                ')' => match state_stack.pop() {
                    Some(mut parent_state) => {
                        if !state.ast_seq.is_empty() {
                            state.or_seq.push(AST::Seq(state.ast_seq));
                        }
                        match fold_or(state.or_seq) {
                            Some(ast) => {
                                parent_state.ast_seq.push(ast);
                            }
                            None => {}
                        }
                        state = parent_state;
                    }
                    None => return Err(ParseError::InvalidRightParen(i)),
                },
                '|' => {
                    if state.ast_seq.is_empty() {
                        return Err(ParseError::NoPrev(i));
                    } else {
                        state.or_seq.push(AST::Seq(take(&mut state.ast_seq)))
                    }
                }
                '\\' => {
                    state.escape = true;
                }
                _ => {
                    state.ast_seq.push(AST::Char(c));
                }
            }
        }
    }

    if !state_stack.is_empty() {
        return Err(ParseError::NoRightParen);
    }

    if !state.ast_seq.is_empty() {
        state.or_seq.push(AST::Seq(state.ast_seq));
    }

    match fold_or(state.or_seq) {
        Some(ast) => Ok(ast),
        None => Err(ParseError::Empty),
    }
}
