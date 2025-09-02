use std::fmt;

use pest::{
    Parser,
    error::{InputLocation, LineColLocation},
    iterators::Pair,
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "lexer.pest"]
pub struct SysYLexer;

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub value: Value,
    pub lineno: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} at Line {}.", self.kind, &self.value, self.lineno)
    }
}

impl Token {
    pub fn new(kind: TokenKind, value: Value, lineno: usize) -> Self {
        Token {
            kind,
            value,
            lineno,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    Const,
    Int,
    Void,
    If,
    Else,
    While,
    Break,
    Continue,
    Return,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    Plus,
    Minus,
    Mul,
    Div,
    Mod,
    Assign,
    Not,
    And,
    Or,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Ident,
    IntegerConst,
    Eoi,
}

impl From<TokenKind> for String {
    fn from(value: TokenKind) -> Self {
        match value {
            TokenKind::Const => "const".to_string(),
            TokenKind::Int => "int".to_string(),
            TokenKind::Void => "void".to_string(),
            TokenKind::If => "if".to_string(),
            TokenKind::Else => "else".to_string(),
            TokenKind::While => "while".to_string(),
            TokenKind::Break => "break".to_string(),
            TokenKind::Continue => "continue".to_string(),
            TokenKind::Return => "return".to_string(),
            TokenKind::Eq => "==".to_string(),
            TokenKind::Neq => "!=".to_string(),
            TokenKind::Lt => "<".to_string(),
            TokenKind::Gt => ">".to_string(),
            TokenKind::Le => "<=".to_string(),
            TokenKind::Ge => ">=".to_string(),
            TokenKind::Plus => "+".to_string(),
            TokenKind::Minus => "-".to_string(),
            TokenKind::Mul => "*".to_string(),
            TokenKind::Div => "/".to_string(),
            TokenKind::Mod => "%".to_string(),
            TokenKind::Assign => "=".to_string(),
            TokenKind::Not => "!".to_string(),
            TokenKind::And => "&&".to_string(),
            TokenKind::Or => "||".to_string(),
            TokenKind::LParen => "(".to_string(),
            TokenKind::RParen => ")".to_string(),
            TokenKind::LBrace => "{".to_string(),
            TokenKind::RBrace => "}".to_string(),
            TokenKind::LBracket => "[".to_string(),
            TokenKind::RBracket => "]".to_string(),
            TokenKind::Comma => ",".to_string(),
            TokenKind::Semicolon => ";".to_string(),
            TokenKind::Eoi => "eoi".to_string(),
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Const => write!(f, "CONST"),
            TokenKind::Int => write!(f, "INT"),
            TokenKind::Void => write!(f, "VOID"),
            TokenKind::If => write!(f, "IF"),
            TokenKind::Else => write!(f, "ELSE"),
            TokenKind::While => write!(f, "WHILE"),
            TokenKind::Break => write!(f, "BREAK"),
            TokenKind::Continue => write!(f, "CONTINUE"),
            TokenKind::Return => write!(f, "RETURN"),
            TokenKind::Eq => write!(f, "EQ"),
            TokenKind::Neq => write!(f, "NEQ"),
            TokenKind::Lt => write!(f, "LT"),
            TokenKind::Gt => write!(f, "GT"),
            TokenKind::Le => write!(f, "LE"),
            TokenKind::Ge => write!(f, "GE"),
            TokenKind::Plus => write!(f, "PLUS"),
            TokenKind::Minus => write!(f, "MINUS"),
            TokenKind::Mul => write!(f, "MUL"),
            TokenKind::Div => write!(f, "DIV"),
            TokenKind::Mod => write!(f, "MOD"),
            TokenKind::Assign => write!(f, "ASSIGN"),
            TokenKind::Not => write!(f, "NOT"),
            TokenKind::And => write!(f, "AND"),
            TokenKind::Or => write!(f, "OR"),
            TokenKind::LParen => write!(f, "L_PAREN"),
            TokenKind::RParen => write!(f, "R_PAREN"),
            TokenKind::LBrace => write!(f, "L_BRACE"),
            TokenKind::RBrace => write!(f, "R_BRACE"),
            TokenKind::LBracket => write!(f, "L_BRACKT"),
            TokenKind::RBracket => write!(f, "R_BRACKT"),
            TokenKind::Comma => write!(f, "COMMA"),
            TokenKind::Semicolon => write!(f, "SEMICOLON"),
            TokenKind::Ident => write!(f, "IDENT"),
            TokenKind::IntegerConst => write!(f, "INTEGER_CONST"),
            TokenKind::Eoi => write!(f, "EOI"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Int(i64),
    String(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "{}", s),
        }
    }
}

fn parse_string_to_i64(s: &str) -> Result<i64, String> {
    // 去除前后空白字符
    let s = s.trim();

    if s.is_empty() {
        return Err("Empty string".to_string());
    }

    // 检查十六进制格式
    if s.starts_with("0x") || s.starts_with("0X") {
        if s.len() <= 2 {
            return Err("Invalid hex format: no digits after prefix".to_string());
        }

        // 解析十六进
        let hex_str = &s[2..];
        match i64::from_str_radix(hex_str, 16) {
            Ok(value) => Ok(value),
            Err(e) => Err(format!("Invalid hex format: {}", e)),
        }
    }
    // 检查八进制格式（以0开头但不是十六进制）
    else if s.starts_with('0') && s.len() > 1 {
        // 解析八进制
        let oct_str = &s[1..];
        match i64::from_str_radix(oct_str, 8) {
            Ok(value) => Ok(value),
            Err(e) => Err(format!("Invalid octal format: {}", e)),
        }
    }
    // 处理十进制格式
    else {
        match s.parse::<i64>() {
            Ok(value) => Ok(value),
            Err(e) => Err(format!("Invalid decimal format: {}", e)),
        }
    }
}

fn get_token(p: &Pair<'_, Rule>) -> Token {
    // println!("{:?}", p.as_rule());
    match p.as_rule() {
        Rule::CONST => Token::new(
            TokenKind::Const,
            Value::String(TokenKind::Const.into()),
            p.line_col().0,
        ),
        Rule::INT => Token::new(
            TokenKind::Int,
            Value::String(TokenKind::Int.into()),
            p.line_col().0,
        ),
        Rule::VOID => Token::new(
            TokenKind::Void,
            Value::String(TokenKind::Void.into()),
            p.line_col().0,
        ),
        Rule::IF => Token::new(
            TokenKind::If,
            Value::String(TokenKind::If.into()),
            p.line_col().0,
        ),
        Rule::ELSE => Token::new(
            TokenKind::Else,
            Value::String(TokenKind::Else.into()),
            p.line_col().0,
        ),
        Rule::WHILE => Token::new(
            TokenKind::While,
            Value::String(TokenKind::While.into()),
            p.line_col().0,
        ),
        Rule::BREAK => Token::new(
            TokenKind::Break,
            Value::String(TokenKind::Break.into()),
            p.line_col().0,
        ),
        Rule::CONTINUE => Token::new(
            TokenKind::Continue,
            Value::String(TokenKind::Continue.into()),
            p.line_col().0,
        ),
        Rule::RETURN => Token::new(
            TokenKind::Return,
            Value::String(TokenKind::Return.into()),
            p.line_col().0,
        ),
        Rule::EQ => Token::new(
            TokenKind::Eq,
            Value::String(TokenKind::Eq.into()),
            p.line_col().0,
        ),
        Rule::NEQ => Token::new(
            TokenKind::Neq,
            Value::String(TokenKind::Neq.into()),
            p.line_col().0,
        ),
        Rule::LT => Token::new(
            TokenKind::Lt,
            Value::String(TokenKind::Lt.into()),
            p.line_col().0,
        ),
        Rule::GT => Token::new(
            TokenKind::Gt,
            Value::String(TokenKind::Gt.into()),
            p.line_col().0,
        ),
        Rule::LE => Token::new(
            TokenKind::Le,
            Value::String(TokenKind::Le.into()),
            p.line_col().0,
        ),
        Rule::GE => Token::new(
            TokenKind::Ge,
            Value::String(TokenKind::Ge.into()),
            p.line_col().0,
        ),
        Rule::PLUS => Token::new(
            TokenKind::Plus,
            Value::String(TokenKind::Plus.into()),
            p.line_col().0,
        ),
        Rule::MINUS => Token::new(
            TokenKind::Minus,
            Value::String(TokenKind::Minus.into()),
            p.line_col().0,
        ),
        Rule::MUL => Token::new(
            TokenKind::Mul,
            Value::String(TokenKind::Mul.into()),
            p.line_col().0,
        ),
        Rule::DIV => Token::new(
            TokenKind::Div,
            Value::String(TokenKind::Div.into()),
            p.line_col().0,
        ),
        Rule::MOD => Token::new(
            TokenKind::Mod,
            Value::String(TokenKind::Mod.into()),
            p.line_col().0,
        ),
        Rule::ASSIGN => Token::new(
            TokenKind::Assign,
            Value::String(TokenKind::Assign.into()),
            p.line_col().0,
        ),
        Rule::NOT => Token::new(
            TokenKind::Not,
            Value::String(TokenKind::Not.into()),
            p.line_col().0,
        ),
        Rule::AND => Token::new(
            TokenKind::And,
            Value::String(TokenKind::And.into()),
            p.line_col().0,
        ),
        Rule::OR => Token::new(
            TokenKind::Or,
            Value::String(TokenKind::Or.into()),
            p.line_col().0,
        ),
        Rule::L_PAREN => Token::new(
            TokenKind::LParen,
            Value::String(TokenKind::LParen.into()),
            p.line_col().0,
        ),
        Rule::R_PAREN => Token::new(
            TokenKind::RParen,
            Value::String(TokenKind::RParen.into()),
            p.line_col().0,
        ),
        Rule::L_BRACE => Token::new(
            TokenKind::LBrace,
            Value::String(TokenKind::LBrace.into()),
            p.line_col().0,
        ),
        Rule::R_BRACE => Token::new(
            TokenKind::RBrace,
            Value::String(TokenKind::RBrace.into()),
            p.line_col().0,
        ),
        Rule::L_BRACKT => Token::new(
            TokenKind::LBracket,
            Value::String(TokenKind::LBracket.into()),
            p.line_col().0,
        ),
        Rule::R_BRACKT => Token::new(
            TokenKind::RBracket,
            Value::String(TokenKind::RBracket.into()),
            p.line_col().0,
        ),
        Rule::COMMA => Token::new(
            TokenKind::Comma,
            Value::String(TokenKind::Comma.into()),
            p.line_col().0,
        ),
        Rule::SEMICOLON => Token::new(
            TokenKind::Semicolon,
            Value::String(TokenKind::Semicolon.into()),
            p.line_col().0,
        ),
        Rule::IDENT => Token::new(
            TokenKind::Ident,
            Value::String(p.as_str().into()),
            p.line_col().0,
        ),
        Rule::INTEGER_CONST => Token::new(
            TokenKind::IntegerConst,
            Value::Int(parse_string_to_i64(p.as_str()).unwrap()),
            p.line_col().0,
        ),
        Rule::EOI => Token::new(
            TokenKind::Eoi,
            Value::String(TokenKind::Eoi.into()),
            p.line_col().0,
        ),
        _ => unreachable!(),
    }
}

fn detect_all_error(input: &str, pos: usize, base_line: usize) {
    let new_input = &input[pos + 1..];
    // let state: Box<pest::ParserState<&str>> = ParserState::new(input);
    // let result = state.skip(pos).unwrap();
    match SysYLexer::parse(Rule::program, new_input) {
        Ok(_) => return,
        Err(e) => {
            let line_col = &e.line_col;
            let line = match line_col {
                LineColLocation::Pos((line, _)) => line,
                LineColLocation::Span((start_line, _), (_, _)) => start_line,
            };
            let location = e.location;
            let pos = match location {
                InputLocation::Pos(pos) => pos,
                InputLocation::Span((pos_start, _pos_end)) => pos_start,
            };
            eprintln!(
                "Error type A at Line {}: Mysterious character {}.",
                base_line - 1 + line,
                new_input.chars().nth(pos).unwrap_or(' ')
            );
            detect_all_error(new_input, pos, base_line - 1 + line);
        }
    }
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let pair = match SysYLexer::parse(Rule::program, input) {
        Ok(mut p) => p.next().unwrap(),
        Err(e) => {
            let line_col = &e.line_col;
            let line = match line_col {
                LineColLocation::Pos((line, _)) => line,
                LineColLocation::Span((start_line, _), (_, _)) => start_line,
            };
            let location = e.location;
            let pos = match location {
                InputLocation::Pos(pos) => pos,
                InputLocation::Span((pos_start, _pos_end)) => pos_start,
            };
            eprintln!(
                "Error type A at Line {}: Mysterious character {}.",
                line,
                input.chars().nth(pos).unwrap_or(' ')
            );
            detect_all_error(input, pos, *line);
            return vec![];
        }
    };
    pair.into_inner()
        .map(|p| {
            if p.as_rule() != Rule::EOI {
                get_token(&p.into_inner().next().unwrap())
            } else {
                Token::new(
                    TokenKind::Eoi,
                    Value::String(TokenKind::Eoi.into()),
                    p.line_col().0,
                )
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::lexer::tokenize;

    #[test]
    fn test_tokenize_normal0() {
        let input = "int main()
        {
           // line comment
           /*
             block comment
           */
           int i = 0x1;
        }";
        let mut tokens = tokenize(input);
        tokens.remove(tokens.len() - 1);
        let expected = "INT int at Line 1.
            IDENT main at Line 1.
            L_PAREN ( at Line 1.
            R_PAREN ) at Line 1.
            L_BRACE { at Line 2.
            INT int at Line 7.
            IDENT i at Line 7.
            ASSIGN = at Line 7.
            INTEGER_CONST 1 at Line 7.
            SEMICOLON ; at Line 7.
            R_BRACE } at Line 8.
            ";
        assert_eq!(
            expected
                .trim()
                .split("\n")
                .map(|line| line.replace("\t", "").trim().to_string())
                .collect::<Vec<String>>(),
            tokens
                .iter()
                .map(|token| token.to_string())
                .collect::<Vec<String>>()[..]
        );
    }

    #[test]
    fn test_tokenize_error() {
        let input = "int main() {
           int i = 1;
           int j = ~i;
        }";
        let _tokens = tokenize(input);
    }

    #[test]
    fn test_tokenize_normal1() {
        let input = "int func(int arg) {
            int l;
            l = - - - arg;
            return l;
        }

        int main() {
            int x, y;
            x = 02;
            y = 0x1;
            x = x - 1 + y;
            if (+-!!!x) {
                x = - - -2;
            }
            else {
                x = 1 + + y;
            }
            func(x);
            return 0;
        }";
        let mut tokens = tokenize(input);
        tokens.remove(tokens.len() - 1);
        let expected = "INT int at Line 1.
        IDENT func at Line 1.
        L_PAREN ( at Line 1.
        INT int at Line 1.
        IDENT arg at Line 1.
        R_PAREN ) at Line 1.
        L_BRACE { at Line 1.
        INT int at Line 2.
        IDENT l at Line 2.
        SEMICOLON ; at Line 2.
        IDENT l at Line 3.
        ASSIGN = at Line 3.
        MINUS - at Line 3.
        MINUS - at Line 3.
        MINUS - at Line 3.
        IDENT arg at Line 3.
        SEMICOLON ; at Line 3.
        RETURN return at Line 4.
        IDENT l at Line 4.
        SEMICOLON ; at Line 4.
        R_BRACE } at Line 5.
        INT int at Line 7.
        IDENT main at Line 7.
        L_PAREN ( at Line 7.
        R_PAREN ) at Line 7.
        L_BRACE { at Line 7.
        INT int at Line 8.
        IDENT x at Line 8.
        COMMA , at Line 8.
        IDENT y at Line 8.
        SEMICOLON ; at Line 8.
        IDENT x at Line 9.
        ASSIGN = at Line 9.
        INTEGER_CONST 2 at Line 9.
        SEMICOLON ; at Line 9.
        IDENT y at Line 10.
        ASSIGN = at Line 10.
        INTEGER_CONST 1 at Line 10.
        SEMICOLON ; at Line 10.
        IDENT x at Line 11.
        ASSIGN = at Line 11.
        IDENT x at Line 11.
        MINUS - at Line 11.
        INTEGER_CONST 1 at Line 11.
        PLUS + at Line 11.
        IDENT y at Line 11.
        SEMICOLON ; at Line 11.
        IF if at Line 12.
        L_PAREN ( at Line 12.
        PLUS + at Line 12.
        MINUS - at Line 12.
        NOT ! at Line 12.
        NOT ! at Line 12.
        NOT ! at Line 12.
        IDENT x at Line 12.
        R_PAREN ) at Line 12.
        L_BRACE { at Line 12.
        IDENT x at Line 13.
        ASSIGN = at Line 13.
        MINUS - at Line 13.
        MINUS - at Line 13.
        MINUS - at Line 13.
        INTEGER_CONST 2 at Line 13.
        SEMICOLON ; at Line 13.
        R_BRACE } at Line 14.
        ELSE else at Line 15.
        L_BRACE { at Line 15.
        IDENT x at Line 16.
        ASSIGN = at Line 16.
        INTEGER_CONST 1 at Line 16.
        PLUS + at Line 16.
        PLUS + at Line 16.
        IDENT y at Line 16.
        SEMICOLON ; at Line 16.
        R_BRACE } at Line 17.
        IDENT func at Line 18.
        L_PAREN ( at Line 18.
        IDENT x at Line 18.
        R_PAREN ) at Line 18.
        SEMICOLON ; at Line 18.
        RETURN return at Line 19.
        INTEGER_CONST 0 at Line 19.
        SEMICOLON ; at Line 19.
        R_BRACE } at Line 20.";
        assert_eq!(
            expected
                .trim()
                .split("\n")
                .map(|line| line.replace("\t", "").trim().to_string())
                .collect::<Vec<String>>(),
            tokens
                .iter()
                .map(|token| token.to_string())
                .collect::<Vec<String>>()[..]
        );
    }

    #[test]
    fn test_tokenize_normal2() {
        let input = "int array()
        {
            int arr[10] = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9};

            int a1 = 0, a2 = 3, a3 = 5, a4 = 7, a5 = 9, a6 = 1, a7 = 2, a8 = 4,
                a9 = 6;

            return arr[a1] + arr[a2] + arr[a3] + arr[a4] + arr[a7] + arr[a8];
        }

        int main()
        {
            int q = 1, r = 2, s = 04, t = 0x7, u = 0xA, v = 0xb, w = 0xcD, x = 077;

            int sum1 = q + r + s + t + u + v + w + x;

            int sum2 = array();

            int sum3 = sum1 + sum2;

            return 0;
        }";
        let mut tokens = tokenize(input);
        tokens.remove(tokens.len() - 1);
        let expected = "INT int at Line 1.
        IDENT array at Line 1.
        L_PAREN ( at Line 1.
        R_PAREN ) at Line 1.
        L_BRACE { at Line 2.
        INT int at Line 3.
        IDENT arr at Line 3.
        L_BRACKT [ at Line 3.
        INTEGER_CONST 10 at Line 3.
        R_BRACKT ] at Line 3.
        ASSIGN = at Line 3.
        L_BRACE { at Line 3.
        INTEGER_CONST 0 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 1 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 2 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 3 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 4 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 5 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 6 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 7 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 8 at Line 3.
        COMMA , at Line 3.
        INTEGER_CONST 9 at Line 3.
        R_BRACE } at Line 3.
        SEMICOLON ; at Line 3.
        INT int at Line 5.
        IDENT a1 at Line 5.
        ASSIGN = at Line 5.
        INTEGER_CONST 0 at Line 5.
        COMMA , at Line 5.
        IDENT a2 at Line 5.
        ASSIGN = at Line 5.
        INTEGER_CONST 3 at Line 5.
        COMMA , at Line 5.
        IDENT a3 at Line 5.
        ASSIGN = at Line 5.
        INTEGER_CONST 5 at Line 5.
        COMMA , at Line 5.
        IDENT a4 at Line 5.
        ASSIGN = at Line 5.
        INTEGER_CONST 7 at Line 5.
        COMMA , at Line 5.
        IDENT a5 at Line 5.
        ASSIGN = at Line 5.
        INTEGER_CONST 9 at Line 5.
        COMMA , at Line 5.
        IDENT a6 at Line 5.
        ASSIGN = at Line 5.
        INTEGER_CONST 1 at Line 5.
        COMMA , at Line 5.
        IDENT a7 at Line 5.
        ASSIGN = at Line 5.
        INTEGER_CONST 2 at Line 5.
        COMMA , at Line 5.
        IDENT a8 at Line 5.
        ASSIGN = at Line 5.
        INTEGER_CONST 4 at Line 5.
        COMMA , at Line 5.
        IDENT a9 at Line 6.
        ASSIGN = at Line 6.
        INTEGER_CONST 6 at Line 6.
        SEMICOLON ; at Line 6.
        RETURN return at Line 8.
        IDENT arr at Line 8.
        L_BRACKT [ at Line 8.
        IDENT a1 at Line 8.
        R_BRACKT ] at Line 8.
        PLUS + at Line 8.
        IDENT arr at Line 8.
        L_BRACKT [ at Line 8.
        IDENT a2 at Line 8.
        R_BRACKT ] at Line 8.
        PLUS + at Line 8.
        IDENT arr at Line 8.
        L_BRACKT [ at Line 8.
        IDENT a3 at Line 8.
        R_BRACKT ] at Line 8.
        PLUS + at Line 8.
        IDENT arr at Line 8.
        L_BRACKT [ at Line 8.
        IDENT a4 at Line 8.
        R_BRACKT ] at Line 8.
        PLUS + at Line 8.
        IDENT arr at Line 8.
        L_BRACKT [ at Line 8.
        IDENT a7 at Line 8.
        R_BRACKT ] at Line 8.
        PLUS + at Line 8.
        IDENT arr at Line 8.
        L_BRACKT [ at Line 8.
        IDENT a8 at Line 8.
        R_BRACKT ] at Line 8.
        SEMICOLON ; at Line 8.
        R_BRACE } at Line 9.
        INT int at Line 11.
        IDENT main at Line 11.
        L_PAREN ( at Line 11.
        R_PAREN ) at Line 11.
        L_BRACE { at Line 12.
        INT int at Line 13.
        IDENT q at Line 13.
        ASSIGN = at Line 13.
        INTEGER_CONST 1 at Line 13.
        COMMA , at Line 13.
        IDENT r at Line 13.
        ASSIGN = at Line 13.
        INTEGER_CONST 2 at Line 13.
        COMMA , at Line 13.
        IDENT s at Line 13.
        ASSIGN = at Line 13.
        INTEGER_CONST 4 at Line 13.
        COMMA , at Line 13.
        IDENT t at Line 13.
        ASSIGN = at Line 13.
        INTEGER_CONST 7 at Line 13.
        COMMA , at Line 13.
        IDENT u at Line 13.
        ASSIGN = at Line 13.
        INTEGER_CONST 10 at Line 13.
        COMMA , at Line 13.
        IDENT v at Line 13.
        ASSIGN = at Line 13.
        INTEGER_CONST 11 at Line 13.
        COMMA , at Line 13.
        IDENT w at Line 13.
        ASSIGN = at Line 13.
        INTEGER_CONST 205 at Line 13.
        COMMA , at Line 13.
        IDENT x at Line 13.
        ASSIGN = at Line 13.
        INTEGER_CONST 63 at Line 13.
        SEMICOLON ; at Line 13.
        INT int at Line 15.
        IDENT sum1 at Line 15.
        ASSIGN = at Line 15.
        IDENT q at Line 15.
        PLUS + at Line 15.
        IDENT r at Line 15.
        PLUS + at Line 15.
        IDENT s at Line 15.
        PLUS + at Line 15.
        IDENT t at Line 15.
        PLUS + at Line 15.
        IDENT u at Line 15.
        PLUS + at Line 15.
        IDENT v at Line 15.
        PLUS + at Line 15.
        IDENT w at Line 15.
        PLUS + at Line 15.
        IDENT x at Line 15.
        SEMICOLON ; at Line 15.
        INT int at Line 17.
        IDENT sum2 at Line 17.
        ASSIGN = at Line 17.
        IDENT array at Line 17.
        L_PAREN ( at Line 17.
        R_PAREN ) at Line 17.
        SEMICOLON ; at Line 17.
        INT int at Line 19.
        IDENT sum3 at Line 19.
        ASSIGN = at Line 19.
        IDENT sum1 at Line 19.
        PLUS + at Line 19.
        IDENT sum2 at Line 19.
        SEMICOLON ; at Line 19.
        RETURN return at Line 21.
        INTEGER_CONST 0 at Line 21.
        SEMICOLON ; at Line 21.
        R_BRACE } at Line 22.";
        assert_eq!(
            expected
                .trim()
                .split("\n")
                .map(|line| line.replace("\t", "").trim().to_string())
                .collect::<Vec<String>>(),
            tokens
                .iter()
                .map(|token| token.to_string())
                .collect::<Vec<String>>()[..]
        );
    }
    
    #[test]
    fn test() {
        let input = "const const_abc";
        let tokens = tokenize(input);
        for token in tokens {
            println!("{}", token);
        }
    }
}
