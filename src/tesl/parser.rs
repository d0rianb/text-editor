pub struct Parser;

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Identifier(&'a str), // variable name
    Keyword(&'a str),  // for, if, else
    Separator(&'a str), // ( ) { } ;
    Type(&'a str), // int, str, float, bool
    Operator(&'a str), // = + / *
    Value(&'a str), // true, 2.3,
}

#[derive(Debug)]
pub enum Type<'a> {
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(&'a str)
}

impl Parser {
    pub fn parse(_input: &str) -> Vec<Token> {
        vec![]
    }
}