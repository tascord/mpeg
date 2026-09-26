use std::{cell::RefCell, rc::Rc};
use crate::parser::Parser;
use crate::error::ParseError;

pub type RootType = Rc<RefCell<Option<Rc<dyn Parser>>>>;

#[derive(Debug, PartialEq, Clone)]
pub enum Node {
    Named { label: String, children: Vec<Self> },
    Leaf { label: String, value: String },
    Group(Vec<Self>)
}

pub type Result<'a> = std::result::Result<(&'a str, Node), ParseError>;
