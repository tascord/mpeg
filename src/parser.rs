use std::rc::Rc;
use crate::types::{Node, Result, RootType};
use crate::error::ParseError;

pub trait Parser {
    fn parse<'a>(&self, input: &'a str, root: &dyn Parser, original_input: &'a str) -> Result<'a>;
}

pub struct Lazy(pub RootType);
impl Parser for Lazy {
    fn parse<'a>(&self, input: &'a str, _: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let borrow = self.0.borrow();
        match borrow.as_ref() {
            Some(v) => v.parse(input, v.as_ref(), original_input),
            None => Err(ParseError { offset: original_input.len() - input.len(), expected: "uninitialized root".to_string() })
        }
    }
}

pub struct Literal(pub String);
impl Parser for Literal {
    fn parse<'a>(&self, input: &'a str, _root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        if input.starts_with(&self.0) {
            Ok((&input[self.0.len()..], Node::Leaf { label: "Literal".to_owned(), value: self.0.clone() }))
        } else {
            Err(ParseError { offset: original_input.len() - input.len(), expected: format!("'{}'", self.0) })
        }
    }
}

pub struct Choice(pub Vec<Rc<dyn Parser>>);
impl Parser for Choice {
    fn parse<'a>(&self, input: &'a str, root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let mut furthest_err: Option<ParseError> = None;

for alt in &self.0 {
            match alt.parse(input, root, original_input) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if let Some(err) = &mut furthest_err {
                        if e.offset > err.offset {
                            *err = e;
                        } else if e.offset == err.offset {
                            if !err.expected.contains(&e.expected) {
                                err.expected.push_str(" or ");
                                err.expected.push_str(&e.expected);
                            }
                        }
                    } else {
                        furthest_err = Some(e);
                    }
                }
            }
        }
        Err(furthest_err.unwrap_or(ParseError { offset: original_input.len() - input.len(), expected: "choice".to_string() }))
    }
}

pub struct Sequence(pub Vec<Rc<dyn Parser>>);
impl Parser for Sequence {
    fn parse<'a>(&self, input: &'a str, root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let mut cur = input;
        let mut sub = Vec::new();
        for step in &self.0 {
            match step.parse(cur, root, original_input) {
                Ok((rest, n)) => {
                    cur = rest;
                    sub.push(n);
                },
                Err(e) => return Err(e),
            }
        }
        Ok((cur, Node::Group(sub)))
    }
}

pub struct ZeroOrMore(pub Rc<dyn Parser>);
impl Parser for ZeroOrMore {
    fn parse<'a>(&self, input: &'a str, root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let mut cur = input;
        let mut sub = Vec::new();
        loop {
            match self.0.parse(cur, root, original_input) {
                Ok((rest, n)) => {
                    if cur == rest { break; }
                    cur = rest;
                    sub.push(n);
                },
                Err(_) => break, // zero or more never fails
            }
        }
        Ok((cur, Node::Group(sub)))
    }
}

pub struct Labeled(pub String, pub Rc<dyn Parser>);
impl Parser for Labeled {
    fn parse<'a>(&self, input: &'a str, root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let (rest, node) = self.1.parse(input, root, original_input)?;
        Ok((rest, Node::Named { label: self.0.clone(), children: vec![node] }))
    }
}

pub struct RootRef(pub RootType);
impl Parser for RootRef {
    fn parse<'a>(&self, input: &'a str, root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let borrow = self.0.borrow();
        match borrow.as_ref() {
            Some(r) => r.parse(input, root, original_input),
            None => Err(ParseError { offset: original_input.len() - input.len(), expected: "uninitialized root".to_string() })
        }
    }
}

pub struct CharSet(pub Vec<char>);
impl Parser for CharSet {
    fn parse<'a>(&self, input: &'a str, _root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let mut chars = input.chars();
        if let Some(c) = chars.next() {
            if self.0.contains(&c) {
                return Ok((&input[c.len_utf8()..], Node::Leaf { label: "CharSet".to_owned(), value: c.to_string() }));
            }
        }
        Err(ParseError { offset: original_input.len() - input.len(), expected: format!("one of {:?}", self.0) })
    }
}

pub struct WsWrapper(pub Rc<dyn Parser>);
impl Parser for WsWrapper {
    fn parse<'a>(&self, input: &'a str, root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let mut cur = input;
        for (i, c) in input.char_indices() {
            if !c.is_whitespace() {
                cur = &input[i..];
                break;
            }
        }
        if cur.is_empty() && !input.is_empty() && input.chars().all(|c| c.is_whitespace()) {
            cur = &input[input.len()..];
        }
        self.0.parse(cur, root, original_input)
    }
}

pub struct RuleRef(pub String, pub RootType);
impl Parser for RuleRef {
    fn parse<'a>(&self, input: &'a str, root: &dyn Parser, original_input: &'a str) -> Result<'a> {
        let borrow = self.1.borrow();
        match borrow.as_ref() {
            Some(r) => r.parse(input, root, original_input),
            None => Err(ParseError { offset: original_input.len() - input.len(), expected: format!("uninitialized rule ${}", self.0) })
        }
    }
}
