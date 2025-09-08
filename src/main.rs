use std::{env, fs};

use crate::lexer::{tokenize, TokenKind};

mod lexer;
mod parser;

fn main() {
    //收集命令行参数
    let args: Vec<String> = env::args().collect();

    // 检查是否提供了文件名
    if args.len() < 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        std::process::exit(1);
    }

    // 获取文件名
    let filename = &args[1];

    // 读取输入文件
    let input = fs::read_to_string(filename).expect("Failed to read file");

    // 词法分析
    let tokens = tokenize(&input);
    tokens.iter().for_each(|t| {
        if t.kind != TokenKind::Eoi {
            eprintln!("{}", t);
        }
    });
    parser::parse(&input);
}
