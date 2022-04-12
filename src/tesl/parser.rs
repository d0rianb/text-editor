use crate::tesl::lexer::{Lexer, LexerItem};

#[derive(Debug, Clone, PartialEq)]
pub enum Token { // TODO: keep track of the source coords
    Identifier(String), // variable name
    Keyword(Keyword),  // for, if, else
    Separator(Separator), // ( ) { } ;
    Type(Type), // int, str, float, bool
    Operator(Operator), // = + / *
    Value(Value), // true, 2.3,
    NewLine,
    NoOp
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    If,
    Else,
    For,
    In,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(String)
    // TODO: Array
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,
    Substract,
    Mutliply,
    Divide,
    Modulo,
    Equal,
    Arrow, // =>
    DotDot, // .. for range
}

#[derive(Debug, Clone, PartialEq)]
pub enum Separator {
    LParenthesis,
    RParenthesis,
    LBracket,
    RBracket,
    LCurlyBracket,
    RCurlyBracket,
    LAngleBracket,
    RAngleBracket,
    Quote,
    Dot,
}

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Self {}
    }

    fn is_quote(token: &Token) -> bool { token == &Token::Separator(Separator::Quote) }

    fn is_numeric(token: &Token) -> bool {
        if let Token::Identifier(value) = token {
            let mut is_numeric = true;
            for c in value.chars() {
                if matches!(c, '0'..='9' | '-' | '.') { continue; }
                is_numeric = false;
                break;
            }
            return is_numeric;
        }
        matches!(token, Token::Value(Value::Int(_)) | Token::Value(Value::Float(_)))
    }

    fn is_bool(token: &Token) -> bool {
        if let Token::Identifier(value) = token { return value == "true" || value == "false" }
        false
    }

    fn set_token_as_string(token: &mut Token) {
        if let Token::Identifier(val) = token {
            *token = Token::Value(Value::Str(val.clone()));
        }
    }

    fn set_token_as_parsed_numeric(token: &mut Token) {
        if let Token::Identifier(val) = token {
            if val.contains('.') {
                *token = Token::Value(Value::Float(val.parse::<f32>().expect("Unparsable token")));
            } else {
                *token = Token::Value(Value::Int(val.parse::<i32>().expect("Unparsable token")));
            }
        }
    }

    fn set_token_as_parsed_bool(token: &mut Token) {
        if let Token::Identifier(val) = token {
            if val == "true" {
                *token = Token::Value(Value::Bool(true));
            } else if val == "false" {
                *token = Token::Value(Value::Bool(false));
            } else {
                panic!("[Parser] : Unexpected bool : {}", val)
            }
        }
    }

    pub fn tokenize(&mut self, input: Vec<LexerItem>) -> Vec<Token> {
        let mut tokens: Vec<Token> = input
            .iter()
            .map(|el| Self::identify(el))
            .collect();

        let ident_index: Vec<usize> = tokens
            .iter()
            .enumerate()
            .filter(|(i, el)| matches!(*el, Token::Identifier(_)))
            .map(|(i, el)| { i })
            .collect();

        // Check if identifier is instead a value and assign it
        for i in ident_index {
            if Self::is_quote(&tokens[i - 1]) && Self::is_quote(&tokens[i + 1]) { Self::set_token_as_string(&mut tokens[i]); } // value is strings
            if Self::is_numeric(&tokens[i]) { Self::set_token_as_parsed_numeric(&mut tokens[i]); } // value is num
            if Self::is_bool(&tokens[i]) { Self::set_token_as_parsed_bool(&mut tokens[i]); } // value is bool
        }

        let token_len = tokens.len();
        for i in 0..token_len {
            if &tokens[i] == &Token::Separator(Separator::Dot) {
                if i + 1 < token_len && tokens[i + 1] == Token::Separator(Separator::Dot) { // handle '..'
                    tokens[i] = Token::Operator(Operator::DotDot);
                    tokens[i + 1] = Token::NoOp;
                }
                if i > 0 && i + 1 < token_len && Self::is_numeric(&tokens[i - 1]) && Self::is_numeric(&tokens[i + 1]) { // handle floats
                    if let Token::Value(Value::Int(decimal_part)) = &tokens[i - 1] {
                        if let Token::Value(Value::Int(float_part)) = &tokens[i + 1] {
                            let float_value = format!("{}.{}", decimal_part, float_part).parse::<f32>().expect("[Parser] Unable to parse float");
                            tokens[i - 1] = Token::NoOp;
                            tokens[i] = Token::Value(Value::Float(float_value));
                            tokens[i + 1] = Token::NoOp;
                        }
                    }
                }
            }
            if &tokens[i] == &Token::Operator(Operator::Equal) && &tokens[i + 1] == &Token::Separator(Separator::RAngleBracket) {
                tokens[i] = Token::Operator(Operator::Arrow);
                tokens[i + 1] = Token::NoOp;
            }
        }

        tokens = tokens // Remove the NoOp tokens
            .into_iter()
            .filter(|t| t != &Token::NoOp)
            .collect::<Vec<Token>>();

        tokens.clone()
    }

    pub fn parse(input: &str) {}

    /// Identify the details of a literal item
    pub fn identify(item: &LexerItem) -> Token {
        match item {
            LexerItem::Operator(op) => {
                match op.as_str() {
                    "+" => Token::Operator(Operator::Add),
                    "-" => Token::Operator(Operator::Substract),
                    "*" => Token::Operator(Operator::Mutliply),
                    "/" => Token::Operator(Operator::Divide),
                    "%" => Token::Operator(Operator::Modulo),
                    "=" => Token::Operator(Operator::Equal),
                    _ => panic!("[Parser] Unexpected operator {}", op)
                }
            },
            LexerItem::Separator(sep) => {
                match sep.as_str() {
                    "\"" | "'" => Token::Separator(Separator::Quote),
                    "(" => Token::Separator(Separator::LParenthesis),
                    ")" => Token::Separator(Separator::RParenthesis),
                    "[" => Token::Separator(Separator::LBracket),
                    "]" => Token::Separator(Separator::RBracket),
                    "{" => Token::Separator(Separator::LCurlyBracket),
                    "}" => Token::Separator(Separator::RCurlyBracket),
                    "<" => Token::Separator(Separator::LAngleBracket),
                    ">" => Token::Separator(Separator::RAngleBracket),
                    "." => Token::Separator(Separator::Dot),
                    _ => panic!("[Parser] Unexpected separator {}", sep)
                }
            },
            LexerItem::Literal(lit) => {
                match lit.as_str() {
                    "int" => Token::Type(Type::Int),
                    "float" => Token::Type(Type::Float),
                    "bool" => Token::Type(Type::Bool),
                    "str" => Token::Type(Type::Str),
                    "if" => Token::Keyword(Keyword::If),
                    "else" => Token::Keyword(Keyword::Else),
                    "for" => Token::Keyword(Keyword::For),
                    "in" => Token::Keyword(Keyword::In),
                    _ => Token::Identifier(lit.clone()) // arbitrary
                }
            }
            LexerItem::NewLine => Token::NewLine
        }
    }
}