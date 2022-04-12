//  Text Editor Scripting Language

mod lexer;
mod parser;
mod compiler;
mod vm;

#[cfg(test)]
mod tests {
    use crate::tesl::compiler::{Compiler, Instruction};
    use crate::tesl::lexer::{Lexer, LexerItem};
    use crate::tesl::parser::{Keyword, Operator, Parser, Separator, Token, Type, Value};

    #[test]
    fn scan_variable() {
        let input = r#"
        int a = 1
        str b="test un"
    "#;
        assert_eq!(
            Lexer::new().scan(input),
            vec![
                LexerItem::NewLine,
                LexerItem::Literal("int".into()),
                LexerItem::Literal("a".into()),
                LexerItem::Operator("=".into()),
                LexerItem::Literal("1".into()),
                LexerItem::NewLine,
                LexerItem::Literal("str".into()),
                LexerItem::Literal("b".into()),
                LexerItem::Operator("=".into()),
                LexerItem::Separator("\"".into()),
                LexerItem::Literal("test un".into()),
                LexerItem::Separator("\"".into()),
                LexerItem::NewLine,
            ]
        )
    }

    #[test]
    fn scan_operation() {
        let input = r#"int c = a + 2"#;
        assert_eq!(
            Lexer::new().scan(input),
            vec![
                LexerItem::Literal("int".into()),
                LexerItem::Literal("c".into()),
                LexerItem::Operator("=".into()),
                LexerItem::Literal("a".into()),
                LexerItem::Operator("+".into()),
                LexerItem::Literal("2".into()),
            ]
        )
    }

    #[test]
    fn scan_for_loop() {
        let input = r#"
        for i in 1..10 {
            print(i)
        }
        "#;
        assert_eq!(
            Lexer::new().scan(input),
            vec![
                LexerItem::NewLine,
                LexerItem::Literal("for".into()),
                LexerItem::Literal("i".into()),
                LexerItem::Literal("in".into()),
                LexerItem::Literal("1".into()),
                LexerItem::Separator(".".into()),
                LexerItem::Separator(".".into()),
                LexerItem::Literal("10".into()),
                LexerItem::Separator("{".into()),
                LexerItem::NewLine,
                LexerItem::Literal("print".into()),
                LexerItem::Separator("(".into()),
                LexerItem::Literal("i".into()),
                LexerItem::Separator(")".into()),
                LexerItem::NewLine,
                LexerItem::Separator("}".into()),
                LexerItem::NewLine,
            ]
        )
    }

    #[test]
    fn identify_var() {
        let input = r#"int a = 1"#;
        let mut lexer = Lexer::new();
        let mut parser = Parser::new();
        let items = lexer.scan(input);
        let identities: Vec<Token> = items.iter().map(|i| Parser::identify(i)).collect();
        assert_eq!(
            identities,
            vec![
                Token::Type(Type::Int),
                Token::Identifier("a".into()),
                Token::Operator(Operator::Equal),
                Token::Identifier("1".into()),
            ]
        )
    }

    #[test]
    fn identify_loop() {
        let input = r#"
            for i in 1 .. 10 {
                print(i)
            }
        "#;
        let mut lexer = Lexer::new();
        let mut parser = Parser::new();
        let items = lexer.scan(input);
        let identities: Vec<Token> = items.iter().map(|i| Parser::identify(i)).collect();
        assert_eq!(
            identities,
            vec![
                Token::NewLine,
                Token::Keyword(Keyword::For),
                Token::Identifier("i".into()),
                Token::Keyword(Keyword::In),
                Token::Identifier("1".into()),
                Token::Separator(Separator::Dot),
                Token::Separator(Separator::Dot),
                Token::Identifier("10".into()),
                Token::Separator(Separator::LCurlyBracket),
                Token::NewLine,
                Token::Identifier("print".into()),
                Token::Separator(Separator::LParenthesis),
                Token::Identifier("i".into()),
                Token::Separator(Separator::RParenthesis),
                Token::NewLine,
                Token::Separator(Separator::RCurlyBracket),
                Token::NewLine,
            ]
        )
    }

    #[test]
    fn parse_var() {
        let input = r#"int a = 1"#;
        let mut lexer = Lexer::new();
        let mut parser = Parser::new();
        let items = lexer.scan(input);
        assert_eq!(
            parser.tokenize(items),
            vec![
                Token::Type(Type::Int),
                Token::Identifier("a".into()),
                Token::Operator(Operator::Equal),
                Token::Value(Value::Int(1)),
            ]
        )
    }

    #[test]
    fn parse_float() {
        let input = r#"float a = 1.25"#;
        let mut lexer = Lexer::new();
        let mut parser = Parser::new();
        assert_eq!(
            parser.tokenize(lexer.scan(input)),
            vec![
                Token::Type(Type::Float),
                Token::Identifier("a".into()),
                Token::Operator(Operator::Equal),
                Token::Value(Value::Float(1.25)),
            ]
        )
    }

    #[test]
    fn parse_var_types() {
        let input = r#"
        int a = 1
        bool b = true
        str c = "string"
        "#;
        let mut lexer = Lexer::new();
        let mut parser = Parser::new();
        let items = lexer.scan(input);
        assert_eq!(
            parser.tokenize(items),
            vec![
                Token::NewLine,
                Token::Type(Type::Int),
                Token::Identifier("a".into()),
                Token::Operator(Operator::Equal),
                Token::Value(Value::Int(1)),
                Token::NewLine,
                Token::Type(Type::Bool),
                Token::Identifier("b".into()),
                Token::Operator(Operator::Equal),
                Token::Value(Value::Bool(true)),
                Token::NewLine,
                Token::Type(Type::Str),
                Token::Identifier("c".into()),
                Token::Operator(Operator::Equal),
                Token::Separator(Separator::Quote),
                Token::Value(Value::Str("string".into())),
                Token::Separator(Separator::Quote),
                Token::NewLine,
            ]
        )
    }

    #[test]
    fn parse_for_loop() {
        let input = r#"
        for i in 1 .. len {}
        "#;
        let mut lexer = Lexer::new();
        let mut parser = Parser::new();
        assert_eq!(
            parser.tokenize(lexer.scan(input)),
            vec![
                Token::NewLine,
                Token::Keyword(Keyword::For),
                Token::Identifier("i".into()),
                Token::Keyword(Keyword::In),
                Token::Value(Value::Int(1)),
                Token::Operator(Operator::DotDot),
                Token::Identifier("len".into()),
                Token::Separator(Separator::LCurlyBracket),
                Token::Separator(Separator::RCurlyBracket),
                Token::NewLine,
            ]
        )
    }

    #[test]
    fn create_expressions() {
        let input = r#"
        int a = 1
        int b = a + 1
        "#;
        let mut compiler = Compiler::new();
        assert_eq!(
            compiler.create_expressions(input),
            vec![
                vec![Token::Type(Type::Int), Token::Identifier("a".into()), Token::Operator(Operator::Equal), Token::Value(Value::Int(1))],
                vec![Token::Type(Type::Int), Token::Identifier("b".into()), Token::Operator(Operator::Equal), Token::Identifier("a".into()), Token::Operator(Operator::Add), Token::Value(Value::Int(1))]
            ]
        );
    }

    #[test]
    fn add_two_numbers() {
        let input = r#"
        int a = 1
        int b = a + 1
        "#;
        let mut compiler = Compiler::new();
        assert_eq!(
            compiler.create_instructions(input),
            vec![
                Instruction::Instanciate(Type::Int, "a".into(), vec![Token::Value(Value::Int(1))]),
                Instruction::Instanciate(Type::Int, "b".into(), vec![Token::Identifier("a".into()), Token::Operator(Operator::Add), Token::Value(Value::Int(1))]),
            ]
        );
    }
}