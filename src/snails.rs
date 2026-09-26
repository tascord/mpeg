use std::collections::HashMap;
use std::sync::OnceLock;
use std::rc::Rc;
use crate::parser::Parser;
use crate::types::{Node, Result};
use crate::compiler::Compiler;
use crate::error::ParseError;

pub type SnailBuilder = fn(&mut Compiler) -> Rc<dyn Parser>;

pub fn get_snails() -> &'static HashMap<char, SnailBuilder> {
    static SNAILS: OnceLock<HashMap<char, SnailBuilder>> = OnceLock::new();
    SNAILS.get_or_init(|| {
        let mut m: HashMap<char, SnailBuilder> = HashMap::new();
        m.insert('\'', snail_single_quote);
        m.insert('"', snail_double_quote);
        m.insert('.', snail_number);
        m.insert('b', snail_bool);
        m.insert('B', snail_bool_ci);
        m.insert('>', snail_prec);
        m.insert('w', snail_word);
        m.insert('_', snail_whitespace);
        m
    })
}

struct SnailDoubleQuote;
impl Parser for SnailDoubleQuote {
    fn parse<'a>(&self, input: &'a str, _root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        if !input.starts_with('"') {
            return Err(ParseError { offset: original_input.len() - input.len(), expected: "double quote".to_string() });
        }
        let cur = &input[1..];
        let mut escaped = false;
        let mut end_idx = 0;
        for (i, c) in cur.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                end_idx = i;
                break;
            }
        }
        if end_idx == 0 && cur.is_empty() || (end_idx == 0 && !cur.starts_with('"')) {
            return Err(ParseError { offset: original_input.len() - input.len(), expected: "end of string".to_string() });
        }
        let val = &cur[..end_idx];
        let rest = &cur[end_idx + 1..];
        Ok((rest, Node::Leaf { label: "String".to_string(), value: val.to_string() }))
    }
}
fn snail_double_quote(_c: &mut Compiler) -> Rc<dyn Parser> { Rc::new(SnailDoubleQuote) }

struct SnailSingleQuote;
impl Parser for SnailSingleQuote {
    fn parse<'a>(&self, input: &'a str, _root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        if !input.starts_with('\'') {
            return Err(ParseError { offset: original_input.len() - input.len(), expected: "single quote".to_string() });
        }
        let cur = &input[1..];
        let mut escaped = false;
        let mut end_idx = 0;
        for (i, c) in cur.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '\'' {
                end_idx = i;
                break;
            }
        }
        if end_idx == 0 && cur.is_empty() || (end_idx == 0 && !cur.starts_with('\'')) {
            return Err(ParseError { offset: original_input.len() - input.len(), expected: "end of string".to_string() });
        }
        let val = &cur[..end_idx];
        let rest = &cur[end_idx + 1..];
        Ok((rest, Node::Leaf { label: "String".to_string(), value: val.to_string() }))
    }
}
fn snail_single_quote(_c: &mut Compiler) -> Rc<dyn Parser> { Rc::new(SnailSingleQuote) }

struct SnailNumber;
impl Parser for SnailNumber {
    fn parse<'a>(&self, input: &'a str, _root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let cur = input;
        let mut has_digits = false;
        let mut has_dot = false;
        let mut end_idx = 0;
        for (i, c) in cur.char_indices() {
            if c.is_ascii_digit() {
                has_digits = true;
            } else if c == '.' && !has_dot {
                has_dot = true;
            } else {
                end_idx = i;
                break;
            }
        }
        if end_idx == 0 {
            end_idx = cur.len();
        }
        if !has_digits {
            return Err(ParseError { offset: original_input.len() - input.len(), expected: "number".to_string() });
        }
        let val = &cur[..end_idx];
        let rest = &cur[end_idx..];
        Ok((rest, Node::Leaf { label: "Number".to_string(), value: val.to_string() }))
    }
}
fn snail_number(_c: &mut Compiler) -> Rc<dyn Parser> { Rc::new(SnailNumber) }

struct SnailBool(bool);
impl Parser for SnailBool {
    fn parse<'a>(&self, input: &'a str, _root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let target1 = "true";
        let target2 = "false";
        
        let m1 = if self.0 { input.to_lowercase().starts_with(target1) } else { input.starts_with(target1) };
        if m1 {
            return Ok((&input[4..], Node::Leaf { label: "Bool".to_string(), value: "true".to_string() }));
        }
        
        let m2 = if self.0 { input.to_lowercase().starts_with(target2) } else { input.starts_with(target2) };
        if m2 {
            return Ok((&input[5..], Node::Leaf { label: "Bool".to_string(), value: "false".to_string() }));
        }
        
        Err(ParseError { offset: original_input.len() - input.len(), expected: "boolean".to_string() })
    }
}
fn snail_bool(_c: &mut Compiler) -> Rc<dyn Parser> { Rc::new(SnailBool(false)) }
fn snail_bool_ci(_c: &mut Compiler) -> Rc<dyn Parser> { Rc::new(SnailBool(true)) }

struct SnailPrec(#[allow(dead_code)] String);
impl Parser for SnailPrec {
    fn parse<'a>(&self, _input: &'a str, _root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        Err(ParseError { offset: original_input.len(), expected: "prec not implemented".to_string() })
    }
}
fn snail_prec(c: &mut Compiler) -> Rc<dyn Parser> { 
    let label = c.eat().unwrap();
    Rc::new(SnailPrec(label.to_string()))
}

struct SnailWord;
impl Parser for SnailWord {
    fn parse<'a>(&self, input: &'a str, _root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let mut end_idx = 0;
        for (i, c) in input.char_indices() {
            if c.is_alphanumeric() || c == '_' {
                end_idx = i + c.len_utf8();
            } else {
                break;
            }
        }
        if end_idx == 0 {
            return Err(ParseError { offset: original_input.len() - input.len(), expected: "word".to_string() });
        }
        let val = &input[..end_idx];
        let rest = &input[end_idx..];
        Ok((rest, Node::Leaf { label: "Word".to_string(), value: val.to_string() }))
    }
}
fn snail_word(_c: &mut Compiler) -> Rc<dyn Parser> { Rc::new(SnailWord) }

struct SnailWhitespace;
impl Parser for SnailWhitespace {
    fn parse<'a>(&self, input: &'a str, _root: &dyn Parser, _original_input: &'a str) -> Result<'a> {
        let mut end_idx = 0;
        for (i, c) in input.char_indices() {
            if c.is_whitespace() {
                end_idx = i + c.len_utf8();
            } else {
                break;
            }
        }
        let val = &input[..end_idx];
        let rest = &input[end_idx..];
        Ok((rest, Node::Leaf { label: "Whitespace".to_string(), value: val.to_string() }))
    }
}
fn snail_whitespace(_c: &mut Compiler) -> Rc<dyn Parser> { Rc::new(SnailWhitespace) }
