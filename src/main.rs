use syn::{Expr, ExprBinary, ExprReturn, File, Pat};

use std::{collections::HashMap, fs};

pub struct Recorder {
    block_num: usize,
    regs: HashMap<String, usize>,
    new_return_blk: bool,
    mir: String,
}

impl Recorder {
    fn new() -> Self {
        Self {
            block_num: 0,
            regs: HashMap::new(),
            new_return_blk: false,
            mir: String::new(),
        }
    }
}

fn parse_pat(p: Pat, record: &mut Recorder) {
    match p {
        syn::Pat::Ident(pat_ident) => {
            println!("para_ident = {}", pat_ident.ident);
        }
        _ => todo!(),
    }
}

fn parse_return_exp(expr: ExprReturn, record: &mut Recorder) {

}

fn parse_exp(e: Expr, record: &mut Recorder) {
    match e {
        Expr::Binary(expr_binary) => {
            parse_binary_exp(expr_binary, record);
        },
        Expr::Return(expr_return) => {
            parse_return_exp(expr_return, record);
        }
        _ => todo!(),
    }
}

fn parse_binary_exp(expr_binary: ExprBinary, record: &mut Recorder) {
    let op = expr_binary.op;
    let left = expr_binary.left;
    let right = expr_binary.right;

}

fn main() {
    let code = fs::read_to_string("tests/test.rs").unwrap();

    let syntax: File = syn::parse_file(&code).unwrap();

    // println!("{:#?}", syntax);

    // for item in syntax.items {
    //     println!("item = {:?}\n", item);
    // }
    let mut record = Recorder::new();

    for item in syntax.items {
        match item {
            syn::Item::Fn(item_fn) => {
                for attr in item_fn.attrs {
                    println!("attrs = {:?}", attr);
                }
                println!("ident = {}", item_fn.sig.ident);
                for input in item_fn.sig.inputs {
                    // println!("input = {:?}", input);
                    let arg = match input {
                        syn::FnArg::Receiver(receiver) => {}
                        syn::FnArg::Typed(pat_type) => {
                            let ident = match *pat_type.pat {
                                syn::Pat::Ident(pat_ident) => {
                                    println!("para_ident = {}", pat_ident.ident);
                                    pat_ident.ident
                                }
                                _ => todo!(),
                            };
                            let ty = match *pat_type.ty {
                                syn::Type::Path(type_path) => {
                                    println!(
                                        "type = {}",
                                        type_path.path.get_ident().unwrap().to_string()
                                    );
                                    type_path.path.get_ident()
                                }
                                _ => todo!(),
                            }
                        }
                    }
                }
                match item_fn.sig.output {
                    syn::ReturnType::Default => todo!(),
                    syn::ReturnType::Type(rarrow, ty) => {
                        println!("{:?}", ty);
                    }
                }
                for stmt in item_fn.block.stmts {
                    match stmt {
                        syn::Stmt::Local(local) => {
                            println!("local = {:?}", local);
                            parse_pat(local.pat, &mut record);
                            match local.init {
                                Some(l) => {
                                    println!("local init = {:?}", l);
                                    parse_exp(*l.expr, &mut record);
                                },
                                None => todo!(),
                            }
                        }
                        syn::Stmt::Item(item) => {
                            println!("item = {:?}", item);
                        }
                        syn::Stmt::Expr(expr, _semi) => {
                            println!("expr = {:?}", expr);
                            parse_exp(expr, &mut record);
                        }
                        syn::Stmt::Macro(stmt_macro) => {
                            println!("stmt_macro = {:?}", stmt_macro);
                        }
                    }
                }
            }
            _ => todo!(),
        }
    }
}
