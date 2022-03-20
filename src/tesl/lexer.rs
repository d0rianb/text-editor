pub struct Lexer {
    result: Vec<LexerItem>,
    temp: String,
    temp_is_string: bool,
}

#[derive(Debug, PartialEq)]
pub enum LexerItem {
    Operator(String),
    Separator(String),
    Literal(String),
}

impl Lexer {
    pub fn new() -> Self {
        Self {
           result: vec![],
           temp: String::new(),
            temp_is_string: false,
        }
    }

    fn flush_temp(&mut self) {
        if self.temp.is_empty() { return; }
        self.flush_temp_even_empty();
    }

    fn flush_temp_even_empty(&mut self) {
        self.result.push(LexerItem::Literal(self.temp.clone()));
        self.temp = String::new();
    }

    pub fn lex(&mut self, input: &str) -> &Vec<LexerItem> {
        let mut it = input.chars();
        while let Some(c) = it.next() {
            print!("{:?}", &c);
            match c {
                'A'..='z' | '0'..='9' => self.temp.push(c),
                '(' | ')' | '{' | '}' => { self.flush_temp(); self.result.push(LexerItem::Separator(c.to_string())) },
                '"' | '\'' => {
                    if !self.temp_is_string { self.temp_is_string = true }
                    else { self.flush_temp_even_empty(); self.temp_is_string = false; }
                    self.result.push(LexerItem::Separator(c.to_string()));
                },
                '=' | '+' | '-' | '*' | '/' | '%' | '.' => { self.flush_temp(); self.result.push(LexerItem::Operator(c.to_string())) },
                ' ' => if self.temp_is_string { self.result.push(LexerItem::Literal(c.to_string())) } else { self.flush_temp() }
                '\n' => self.flush_temp(),
                _ => panic!("Unexpected character {}", c)
            };
        }
        self.flush_temp();
        &self.result
    }
}