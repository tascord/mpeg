use std::rc::Rc;
use crate::types::RootType;
use crate::parser::{Parser, RootRef, Labeled, Literal, ZeroOrMore, Sequence, Choice, CharSet, WsWrapper};
use crate::snails::get_snails;
use crate::error::CompileError;
use miette::SourceSpan;

use std::collections::HashMap;
use std::cell::RefCell;

pub struct Compiler {
    chars: Vec<char>,
    pos: usize,
    root: RootType,
    src: String,
    flag_w: bool,
    env: Rc<RefCell<HashMap<char, RootType>>>,
}

impl Compiler {
    pub fn new(input: &str, root: RootType) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            root,
            src: input.to_string(),
            flag_w: false,
            env: Rc::new(RefCell::new(HashMap::new())),
        }
    }
    
    fn get_rule(&self, name: char) -> RootType {
        self.env.borrow_mut().entry(name).or_insert_with(Default::default).clone()
    }

    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    pub fn eat(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.eat();
            } else if c == '/' {
                if let Some(&'/') = self.chars.get(self.pos + 1) {
                    self.eat();
                    self.eat();
                    while let Some(ch) = self.eat() {
                        if ch == '\n' { break; }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn compile_term(&mut self) -> Result<Rc<dyn Parser>, CompileError> {
        self.skip_whitespace_and_comments();
        let start_pos = self.pos;
        let c = match self.eat() {
            Some(c) => c,
            None => return Err(CompileError {
                src: self.src.clone(),
                span: SourceSpan::from((start_pos, 1)),
                msg: "Unexpected EOF".to_string(),
            }),
        };

        match c {
            '$' => {
                let next = self.eat();
                if next == Some('_') {
                    Ok(Rc::new(RootRef(self.root.clone())))
                } else if let Some(label) = next {
                    if label.is_ascii_lowercase() {
                        let rule_ref = self.get_rule(label.to_ascii_uppercase());
                        Ok(Rc::new(crate::parser::RuleRef(label.to_ascii_uppercase().to_string(), rule_ref)))
                    } else {
                        // Uppercase or other characters -> Declaration / Label
                        let parser = self.compile_term()?;
                        
                        if label.is_ascii_uppercase() {
                            let rule_ref = self.get_rule(label);
                            *rule_ref.borrow_mut() = Some(parser.clone());
                        }
                        
                        Ok(Rc::new(Labeled(label.to_string(), parser)))
                    }
                } else {
                    Err(CompileError { src: self.src.clone(), span: SourceSpan::from((start_pos, 2)), msg: "Invalid syntax after $".to_string() })
                }
            },
            '"' => {
                let mut lit = String::new();
                while let Some(ch) = self.peek() {
                    if ch == '"' {
                        self.eat();
                        break;
                    }
                    lit.push(ch);
                    self.eat();
                }
                Ok(Rc::new(Literal(lit)))
            },
            '(' => {
                let expr = self.compile_expr()?;
                self.skip_whitespace_and_comments();
                if self.eat() != Some(')') {
                    return Err(CompileError { src: self.src.clone(), span: SourceSpan::from((self.pos, 1)), msg: "Expected ')'".to_string() });
                }
                Ok(expr)
            },
            '[' => {
                let mut chars = Vec::new();
                let mut is_charset = true;
                let mut lit = String::new();
                
                while let Some(ch) = self.peek() {
                    if ch == ']' {
                        self.eat();
                        break;
                    }
                    if ch == '$' || ch == '(' || ch == ')' || ch == '_' {
                        is_charset = false;
                    }
                    lit.push(ch);
                    chars.push(ch);
                    self.eat();
                }
                
                // Hack: if it contains special parser chars, it's probably meant to be a literal `[` then rules then `]`.
                // Wait! If it's a literal `[`, then the `[` was eaten. We should return a Sequence!
                // But the contents were already eaten!
                if !is_charset {
                    // It was a literal `[`! Let's reconstruct it.
                    // Actually, if it's not a charset, maybe it's just a sequence of rules inside literal `[` and `]`?
                    // This means `[` is just a grouping like `(` but implies literal brackets.
                    // This perfectly matches `json.mpeg`!
                    // Let's compile the string `lit` as an expression!
                    let mut sub_compiler = Compiler::new(&lit, self.root.clone());
                    let sub_expr = sub_compiler.compile_expr()?;
                    Ok(Rc::new(Sequence(vec![
                        Rc::new(Literal("[".to_string())),
                        sub_expr,
                        Rc::new(Literal("]".to_string()))
                    ])))
                } else {
                    Ok(Rc::new(CharSet(chars)))
                }
            },
            '{' => {
                // Similar to `[` grouping with literals!
                let mut lit = String::new();
                while let Some(ch) = self.peek() {
                    if ch == '}' {
                        self.eat();
                        break;
                    }
                    lit.push(ch);
                    self.eat();
                }
                let mut sub_compiler = Compiler::new(&lit, self.root.clone());
                let sub_expr = sub_compiler.compile_expr()?;
                Ok(Rc::new(Sequence(vec![
                    Rc::new(Literal("{".to_string())),
                    sub_expr,
                    Rc::new(Literal("}".to_string()))
                ])))
            },
            '@' => {
                let next = match self.eat() {
                    Some(n) => n,
                    None => return Err(CompileError { src: self.src.clone(), span: SourceSpan::from((start_pos, 1)), msg: "Unexpected EOF after @".to_string() }),
                };
                if let Some(builder) = get_snails().get(&next) {
                    Ok(builder(self))
                } else {
                    Err(CompileError { src: self.src.clone(), span: SourceSpan::from((start_pos, 2)), msg: format!("Unknown snail '@{}'", next) })
                }
            },
            ':' | ',' => {
                // these are just literals in json.mpeg!
                Ok(Rc::new(Literal(c.to_string())))
            },
            oth => Err(CompileError { src: self.src.clone(), span: SourceSpan::from((start_pos, 1)), msg: format!("Unexpected char '{}'", oth) }),
        }
    }

    fn compile_seq(&mut self) -> Result<Rc<dyn Parser>, CompileError> {
        let mut steps = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let c = match self.peek() {
                Some(ch) => ch,
                None => break,
            };
            if c == ')' || c == ']' || c == '}' || c == '|' {
                break;
            }

            let mut parser = self.compile_term()?;

            self.skip_whitespace_and_comments();
            if self.peek() == Some('*') {
                self.eat();
                parser = Rc::new(ZeroOrMore(parser))
            }
            if self.flag_w {
                parser = Rc::new(WsWrapper(parser));
            }

            steps.push(parser);
        }

        if steps.len() == 1 {
            Ok(steps.pop().unwrap())
        } else {
            {
Ok(Rc::new(Sequence(steps)))
}
        }
    }

    pub fn compile_expr(&mut self) -> Result<Rc<dyn Parser>, CompileError> {
        self.skip_whitespace_and_comments();
        let mut alts = vec![self.compile_seq()?];

        loop {
            self.skip_whitespace_and_comments();
            if self.peek() == Some('|') {
                self.eat();
                {
let seq = self.compile_seq()?;
alts.push(seq);
}
            } else {
                break;
            }
        }

        if alts.len() == 1 {
            Ok(alts.pop().unwrap())
        } else {
            Ok(Rc::new(Choice(alts)))
        }
    }
}

pub fn compile(rules: &str) -> Result<Rc<dyn Parser>, CompileError> {
    let root: RootType = Default::default();
    let mut compiler = Compiler::new(rules, root.clone());
    
    compiler.skip_whitespace_and_comments();
    while compiler.peek() == Some('!') {
        let start_pos = compiler.pos;
        compiler.eat();
        if let Some(flag) = compiler.eat() {
            match flag {
                'W' => compiler.flag_w = true,
                oth => return Err(CompileError { src: compiler.src.clone(), span: SourceSpan::from((start_pos, 2)), msg: format!("Unknown flag !{}", oth) }),
            }
        }
        compiler.skip_whitespace_and_comments();
    }
    
    let engine = compiler.compile_expr()?;
    *root.borrow_mut() = Some(engine.clone());

    Ok(engine)
}

pub fn parse<'a>(parser: &Rc<dyn Parser>, input: &'a str) -> std::result::Result<crate::types::Node, crate::error::MParseError> {
    match parser.parse(input, parser.as_ref(), input) {
        Ok((rest, node)) => {
            if rest.trim().is_empty() {
                Ok(node)
            } else {
                Err(crate::error::MParseError {
                    src: input.to_string(),
                    span: miette::SourceSpan::from((input.len() - rest.len(), 1)),
                    expected: "EOF".to_string(),
                })
            }
        },
        Err(e) => {
            Err(crate::error::MParseError {
                src: input.to_string(),
                span: miette::SourceSpan::from((e.offset, 1)),
                expected: e.expected,
            })
        }
    }
}
