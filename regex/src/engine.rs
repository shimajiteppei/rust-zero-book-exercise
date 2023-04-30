use std::error::Error;

mod codegen;
mod evaluator;
mod parser;

pub type DynError = Box<dyn Error + 'static>;

pub fn do_matching(expr: &str, line: &str, is_depth: bool) -> Result<bool, DynError> {
    let ast = match parser::parse(expr) {
        Ok(ast) => ast,
        Err(parser::ParseError::Empty) => {
            if line.len() == 0 {
                return Ok(true);
            } else {
                return Err(Box::new(parser::ParseError::Empty));
            }
        }
        Err(it) => return Err(Box::new(it)),
    };
    let code = codegen::get_code(&ast)?;
    let line = line.chars().collect::<Vec<char>>();
    Ok(evaluator::eval(&code, &line, is_depth)?)
}

#[cfg(test)]
mod tests {
    use crate::engine::do_matching;
    use rstest::*;

    #[rstest]
    #[case("+b", "bbb")]
    #[case("|b", "bbb")]
    #[case("?b", "bbb")]
    #[case("+b", "bbb")]
    fn test_err(#[case] expr: &str, #[case] line: &str) {
        assert!(do_matching(expr, line, true).is_err());
        assert!(do_matching(expr, line, false).is_err());
    }

    #[rstest]
    #[case("", "")]
    #[case("()", "")]
    // #[case("(()|()|())", "")]
    #[case("abc", "abc")]
    #[case("abc", "dabc")]
    #[case("abc|def", "def")]
    #[case("(abc)*", "abcabcabc")]
    #[case("(ab|cd)+", "abcdcd")]
    #[case("a(bc)?", "a")]
    #[case("a.c", "abc")]
    #[case("a.*", "abc")]
    #[case(".+", "abc")]
    #[case("^abc", "abc")]
    #[case("abc$", "abc")]
    #[case("^abc$", "abc")]
    #[case("^ab.*c$", "abababccccabc")]
    #[case("bc|(d$)|((^a))", "abdc")]
    #[case("abc", "abcdef")]
    #[case("abc|def", "abdef")]
    #[case("(abc)*", "aaaaaabcabcabc")]
    #[case("(ab|cd)+", "aaacbcbdcd")]
    #[case("a(bc)?", "a")]
    fn test_match_success(#[case] expr: &str, #[case] line: &str) {
        assert!(do_matching(expr, line, true).unwrap());
        assert!(do_matching(expr, line, false).unwrap());
    }

    #[rstest]
    #[case("b^", "bbb")]
    #[case("$b", "bbb")]
    #[case("abc", "acb")]
    #[case("abc|def", "bcd")]
    #[case("abc?", "ac")]
    #[case(".+", "")]
    fn test_match_failed(#[case] expr: &str, #[case] line: &str) {
        assert!(!do_matching(expr, line, true).unwrap());
        assert!(!do_matching(expr, line, false).unwrap());
    }
}
