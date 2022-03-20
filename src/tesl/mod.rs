//  Text Editor Scripting Language

mod parser;
mod lexer;

#[cfg(test)]
mod tests {
    use crate::tesl::lexer::{Lexer, LexerItem};
    use crate::tesl::parser::{Parser, Token};
    use super::*;

    #[test]
    fn lex_variable() {
        let input = r#"
        int a = 1
        str b="test"
    "#;
        assert_eq!(
            Lexer::new().lex(input),
            &vec![
                LexerItem::Literal("int".into()),
                LexerItem::Literal("a".into()),
                LexerItem::Operator("=".into()),
                LexerItem::Literal("1".into()),
                LexerItem::Literal("str".into()),
                LexerItem::Literal("b".into()),
                LexerItem::Operator("=".into()),
                LexerItem::Separator("\"".into()),
                LexerItem::Literal("test".into()),
                LexerItem::Separator("\"".into()),
            ]
        )
    }

    #[test]
    fn lex_operation() {
        let input = r#"int c = a + 2"#;
        assert_eq!(
            Lexer::new().lex(input),
            &vec![
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
    fn lex_for_loop() {
        let input = r#"
        for i in 1..10 {
            print(i)
        }
        "#;
        assert_eq!(
            Lexer::new().lex(input),
            &vec![
                LexerItem::Literal("for".into()),
                LexerItem::Literal("i".into()),
                LexerItem::Literal("in".into()),
                LexerItem::Literal("1".into()),
                LexerItem::Operator(".".into()),
                LexerItem::Operator(".".into()),
                LexerItem::Literal("10".into()),
                LexerItem::Separator("{".into()),
                LexerItem::Literal("print".into()),
                LexerItem::Separator("(".into()),
                LexerItem::Literal("i".into()),
                LexerItem::Separator(")".into()),
                LexerItem::Separator("}".into()),
            ]
        )
    }

    // fn parse_variable() {
    //     let str = r#"
    //     int a = 1
    // "#;
    //     assert_eq!(
    //         Parser::parse(str),
    //         vec![
    //             Token::Type("int"),
    //             Token::Identifier("a"),
    //             Token::Operator("="),
    //             Token::Value("1"),
    //         ]
    //     );
    // }
}