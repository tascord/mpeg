use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
#[error("Grammar compilation error: {msg}")]
pub struct CompileError {
    #[source_code]
    pub src: String,
    
    #[label("{msg}")]
    pub span: SourceSpan,
    
    pub msg: String,
}

#[derive(Error, Diagnostic, Debug)]
#[error("Parse error: Expected {expected}")]
pub struct MParseError {
    #[source_code]
    pub src: String,

    #[label("Expected {expected}")]
    pub span: SourceSpan,

    pub expected: String,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub offset: usize,
    pub expected: String,
}
