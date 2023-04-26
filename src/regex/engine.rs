use std::error::Error;

mod codegen;
mod evaluator;
mod parser;

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
    fn test_invalid_regex() {
        assert!(do_matching("+b", "bbb", true).is_err());
        assert!(do_matching("*b", "bbb", true).is_err());
        assert!(do_matching("|b", "bbb", true).is_err());
        assert!(do_matching("?b", "bbb", true).is_err());
        assert!(!do_matching("a^bc", "abc", true).unwrap());
        assert!(!do_matching("ab$c", "abc", true).unwrap());
    }

    #[test]
    fn test_matching() {
        assert!(do_matching("abc", "abc", true).unwrap());
        assert!(do_matching("abc", "dabc", true).unwrap());
        assert!(do_matching("abc|def", "def", true).unwrap());
        assert!(do_matching("(abc)*", "abcabcabc", true).unwrap());
        assert!(do_matching("(ab|cd)+", "abcdcd", true).unwrap());
        assert!(do_matching("a(bc)?", "a", true).unwrap());

        assert!(do_matching("a.c", "abc", true).unwrap());
        assert!(do_matching("a.*", "abc", true).unwrap());
        assert!(do_matching(".+", "abc", true).unwrap());
        assert!(do_matching("^abc", "abc", true).unwrap());
        assert!(do_matching("abc$", "abc", true).unwrap());
        assert!(do_matching("^abc$", "abc", true).unwrap());
        assert!(do_matching("^ab.*c$", "abababccccabc", true).unwrap());
        assert!(do_matching("bc|(d$)|((^a))", "abdc", true).unwrap());
    }

    #[test]
    fn test_not_matching() {
        assert!(!do_matching("abc|def", "bcd", true).unwrap());
        assert!(!do_matching("abc?", "ac", true).unwrap());
        assert!(!do_matching(".+", "", true).unwrap());
    }

    #[test]
    fn test_partial_matching() {
        assert!(do_matching("abc", "abcdef", true).unwrap());
        assert!(do_matching("abc|def", "abdef", true).unwrap());
        assert!(do_matching("(abc)*", "aaaaaabcabcabc", true).unwrap());
        assert!(do_matching("(ab|cd)+", "aaacbcbdcd", true).unwrap());
        assert!(do_matching("a(bc)?", "a", true).unwrap());
    }
}
