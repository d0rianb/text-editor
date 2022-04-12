use crate::tesl::lexer::Lexer;
use crate::tesl::parser::*;

pub type Expression = Vec<Token>;

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Instanciate(Type, String, Expression),
    Evaluate(Expression),
    NoOp
}

pub struct Compiler {
    lexer: Lexer,
    parser: Parser,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            lexer: Lexer::new(),
            parser: Parser::new(),
        }
    }

    pub fn create_expressions(&mut self, input: &str) -> Vec<Expression> {
        let items = self.lexer.scan(input);
        let mut tokens = self.parser.tokenize(items).into_iter();
        let mut expressions = vec![];
        let mut temp = vec![];
        while let Some(token) = tokens.next() {
            if token == Token::NewLine {
                if !temp.is_empty() {
                    expressions.push(temp.clone());
                    temp = vec![];
                }
            } else { temp.push(token); }
        }
        if !temp.is_empty() { expressions.push(temp.clone()); }
        expressions
    }

    pub fn create_instructions(&mut self, input: &str) -> Vec<Instruction> {
        let expressions = self.create_expressions(input);
        let mut result = vec![];
        let expression_len = expressions.len();
        for i in 0 .. expression_len {
            let expr = &expressions[i];
            let instruction = match expr[0] {
                Token::Type(_) => self.parse_declaration(expr),
                _ => Instruction::NoOp
            };
            result.push(instruction);
        }
        result
    }

    fn parse_declaration(&mut self, expression: &Expression) -> Instruction {
        let var_type = if let Token::Type(var_type) = &expression[0] { var_type } else { panic!("Unable to get type") };
        let var_name = if let Token::Identifier(var_name) = &expression[1] { var_name } else { panic!("Unable to get identifier") };
        let equal_sign = expression.get(2).expect("Unable to get equal sign");
        let value_expr = expression[3..].to_vec();
        assert_eq!(equal_sign, &Token::Operator(Operator::Equal));
        Instruction::Instanciate(var_type.clone(), var_name.clone(), value_expr)
    }
}