use std::{env, fs};

mod parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_filename> <output_filename>", args[0]);
        std::process::exit(1);
    }

    // 获取文件名
    let input_filename = &args[1];
    let output_filename = &args[2];

    // 读取输入文件
    let input = fs::read_to_string(input_filename).expect("Failed to read file");

    // 解析并生成LLVM IR
    if let Err(err) = parser::parse(&input, output_filename) {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }

    println!("Successfully generated LLVM IR to {}", output_filename);
}
