use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{IntType, VoidType};
use inkwell::values::{FunctionValue, PointerValue};
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser.pest"]
pub struct SysYParser;

// 编译器结构，包含LLVM上下文和构建器
pub struct LlvmIRGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    scope_stack: Vec<HashMap<String, PointerValue<'ctx>>>,
}

enum FuncType<'ctx> {
    Void(VoidType<'ctx>),
    Int(IntType<'ctx>),
}

impl<'ctx> LlvmIRGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
            scope_stack: Vec::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn find_variable(&self, name: &str) -> Option<PointerValue<'ctx>> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(*val);
            }
        }
        None
    }

    fn add_variable(&mut self, name: &str, val: PointerValue<'ctx>) {
        self.scope_stack.last_mut().unwrap().insert(name.to_string(), val);
    }

    pub fn parse_func_def(&mut self, func_def: Pair<Rule>) {
        let mut inner = func_def.into_inner();

        // Get function's return type
        let func_type = inner.next().unwrap();
        let ret_ty = match func_type.into_inner().next().unwrap().as_rule() {
            Rule::Void => FuncType::Void(self.context.void_type()),
            Rule::Int => FuncType::Int(self.context.i32_type()),
            _ => unreachable!(),
        };

        let func_name = inner.next().unwrap().as_str();

        // LParen
        let _ = inner.next();

        let next = inner.next().unwrap();
        let mut has_params = false;
        let param_types = if next.as_rule() == Rule::FuncFParams {
            has_params = true;
            let mut types = vec![];
            for param in next.into_inner() {
                match param.as_rule() {
                    Rule::FuncFParam => {
                        types.push(self.context.i32_type().into());
                    }
                    Rule::Comma => {}
                    _ => unreachable!(),
                }
            }
            types
        } else {
            vec![]
        };

        // 定义LLVM函数类型
        let fn_type = match ret_ty {
            FuncType::Void(ty) => ty.fn_type(&param_types, false),
            FuncType::Int(ty) => ty.fn_type(&param_types, false),
        };

        let function = self.module.add_function(func_name, fn_type, None);

        let entry_block = self.context.append_basic_block(function, "mainEntry");
        self.builder.position_at_end(entry_block);

        if has_params {
            // RParen
            let _ = inner.next();
        }
        let mut has_return = false;
        self.parse_block(inner.next().unwrap(), function, &mut has_return);

        // 如果没有return语句且返回类型不是void，则添加默认返回值
        // if !has_return && ret_ty != Type::Void {
        //     let zero = self.context.i32_type().const_int(0, false);
        //     self.builder.build_return(Some(&zero));
        // }
    }

    pub fn parse_decl(&mut self, decl: Pair<Rule>, is_global: bool) {
        let mut inner = decl.into_inner();

        // Get decl type
        let decl_type = inner.next().unwrap();

        match decl_type.as_rule() {
            Rule::ConstDecl => {
                // 处理常量声明: const BType ConstDef (, ConstDef)* ;
                // 跳过 const 关键字
                let mut inner = decl_type.into_inner();
                let _btype = inner.next().unwrap(); // BType (目前只有 int)

                // 处理常量定义列表
                for const_def in inner {
                    if const_def.as_rule() == Rule::ConstDef {
                        self.parse_const_def(const_def);
                    } else if const_def.as_rule() == Rule::Semicolon {
                        break;
                    }
                }
            }
            Rule::VarDecl => {
                // 处理变量声明: BType VarDef (, VarDef)* ;
                let mut inner = decl_type.into_inner();
                let _btype = inner.next().unwrap(); // BType (目前只有 int)

                // 处理变量定义列表
                for var_def in inner {
                    if var_def.as_rule() == Rule::VarDef {
                        self.parse_var_def(var_def, is_global);
                    } else if var_def.as_rule() == Rule::Semicolon {
                        break;
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    // 解析常量定义
    fn parse_const_def(&self, const_def: Pair<Rule>) {}

    // 解析变量定义
    fn parse_var_def(&mut self, var_def: Pair<Rule>, is_global: bool) {
        let mut inner = var_def.into_inner();

        // 获取变量名
        let ident = inner.next().unwrap().as_str();

        // skip (LBracket ~ ConstExp ~ RBracket)*
        while let Some(item) = inner.next() {
            if item.as_rule() == Rule::LBracket {
                // Skip ConstExp
                let _ = inner.next();
                // Skip RBracket
                let _ = inner.next();
            } else if item.as_rule() == Rule::Assign {
                let init_val = inner.next().unwrap();
                let value = self.parse_init_val(init_val);

                if is_global {
                    // 在LLVM模块中创建全局变量
                    let global = self.module.add_global(
                        self.context.i32_type(),
                        Some(AddressSpace::default()),
                        ident,
                    );
                    global.set_initializer(&value);
                    break;
                } else {
                    let alloca = self
                        .builder
                        .build_alloca(self.context.i32_type(), ident)
                        .unwrap();
                    let _ = self.builder.build_store(alloca, value);
                    self.add_variable(ident, alloca);
                    break;
                }
            } else if item.as_rule() == Rule::Semicolon {
                // 没有初始化值，创建全局变量并初始化为0
                let zero = self.context.i32_type().const_int(0, false);
                let global = self.module.add_global(
                    self.context.i32_type(),
                    Some(inkwell::AddressSpace::default()),
                    ident,
                );
                global.set_initializer(&zero);
                break;
            }
        }
    }

    // 解析常量初始化值
    fn parse_const_init_val(&self, init_val: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = init_val.into_inner();

        match inner.next().unwrap().as_rule() {
            Rule::ConstExp => {
                // 解析常量表达式
                self.parse_exp(inner.next().unwrap())
            }
            Rule::LBrace => {
                // 处理数组初始化 { ... }
                // 简化处理，返回0
                self.context.i32_type().const_int(0, false)
            }
            _ => self.context.i32_type().const_int(0, false),
        }
    }

    // 解析初始化值
    fn parse_init_val(&self, init_val: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = init_val.into_inner();

        let next = inner.next().unwrap();
        match next.as_rule() {
            Rule::Exp => {
                // 解析表达式
                self.parse_exp(next)
            }
            Rule::LBrace => {
                // 处理数组初始化 { ... }
                // 简化处理，返回0
                self.context.i32_type().const_int(0, false)
            }
            _ => self.context.i32_type().const_int(0, false),
        }
    }

    // 解析代码块
    pub fn parse_block(
        &mut self,
        block: Pair<Rule>,
        function: FunctionValue<'ctx>,
        has_return: &mut bool,
    ) {
        self.enter_scope();
        let mut inner = block.into_inner();

        // 跳过左大括号
        let _ = inner.next();

        // 解析语句
        for item in inner {
            match item.as_rule() {
                Rule::BlockItem => {
                    let block_item = item.into_inner().next().unwrap();
                    match block_item.as_rule() {
                        Rule::Stmt => {
                            self.parse_stmt(block_item, function, has_return);
                        }
                        Rule::Decl => {
                            self.parse_decl(block_item, false);
                        }
                        _ => unreachable!(),
                    }
                }
                Rule::RBrace => break,
                _ => continue,
            }
        }
        self.exit_scope();
    }

    // 解析语句
    pub fn parse_stmt(
        &self,
        stmt: Pair<Rule>,
        _function: FunctionValue<'ctx>,
        has_return: &mut bool,
    ) {
        let mut inner = stmt.into_inner();

        match inner.next().unwrap().as_rule() {
            Rule::Return => {
                // 解析return后面的表达式
                if let Some(expr) = inner.next() {
                    if expr.as_rule() == Rule::Exp {
                        let value = self.parse_exp(expr);
                        let _ret = self.builder.build_return(Some(&value)).unwrap();
                        *has_return = true;
                    }
                }
                // 跳过分号
                let _ = inner.next();
            }
            _ => {
                // 其他语句类型的处理
                // 这里简化处理，只关注return语句
            }
        }
    }

    // 解析表达式
    pub fn parse_exp(&self, exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = exp.into_inner();

        return self.parse_add_exp(inner.next().unwrap());
    }

    // 解析加法表达式
    pub fn parse_add_exp(&self, add_exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = add_exp.into_inner();
        let mut result = self.parse_mul_exp(inner.next().unwrap());

        // 处理连续的加法和减法
        while let Some(item) = inner.next() {
            match item.as_rule() {
                Rule::Plus => {
                    let next_mul = self.parse_mul_exp(inner.next().unwrap());
                    result = self
                        .builder
                        .build_int_add(result, next_mul, "tmp_")
                        .unwrap();
                }
                Rule::Minus => {
                    let next_mul = self.parse_mul_exp(inner.next().unwrap());
                    result = self
                        .builder
                        .build_int_sub(result, next_mul, "tmp_")
                        .unwrap();
                }
                _ => {}
            }
        }

        return result;
    }

    // 解析乘法表达式
    pub fn parse_mul_exp(&self, mul_exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = mul_exp.into_inner();
        let mut result = self.parse_unary_exp(inner.next().unwrap());

        // 处理连续的乘法、除法和取模
        while let Some(item) = inner.next() {
            match item.as_rule() {
                Rule::Mul => {
                    let next_unary = self.parse_unary_exp(inner.next().unwrap());
                    result = self
                        .builder
                        .build_int_mul(result, next_unary, "multmp")
                        .unwrap();
                }
                Rule::Div => {
                    let next_unary = self.parse_unary_exp(inner.next().unwrap());
                    result = self
                        .builder
                        .build_int_signed_div(result, next_unary, "divtmp")
                        .unwrap();
                }
                Rule::Mod => {
                    let next_unary = self.parse_unary_exp(inner.next().unwrap());
                    result = self
                        .builder
                        .build_int_signed_rem(result, next_unary, "modtmp")
                        .unwrap();
                }
                _ => {}
            }
        }

        return result;
    }

    // 解析一元表达式
    pub fn parse_unary_exp(&self, unary_exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = unary_exp.into_inner();

        // 获取第一个元素
        if let Some(item) = inner.next() {
            match item.as_rule() {
                Rule::PrimaryExp => {
                    return self.parse_primary_exp(item);
                }
                Rule::UnaryOp => {
                    let op = item.as_str();
                    // 获取一元操作符后面的表达式
                    if let Some(next_item) = inner.next() {
                        let value = self.parse_unary_exp(next_item);
                        match op {
                            "-" => return value.const_neg(),
                            "!" => return value.const_not(),
                            _ => return value,
                        }
                    }
                }
                _ => {
                    // 处理其他情况
                }
            }
        }

        // 默认返回0
        return self.context.i32_type().const_int(0, false);
    }

    // 解析基本表达式
    pub fn parse_primary_exp(&self, primary_exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = primary_exp.into_inner();

        // 获取第一个元素
        if let Some(item) = inner.next() {
            match item.as_rule() {
                Rule::Number => {
                    let num_str = item.as_str();
                    // 处理不同进制的数字
                    let value = if num_str.starts_with("0x") || num_str.starts_with("0X") {
                        u64::from_str_radix(&num_str[2..], 16).unwrap_or(0)
                    } else if num_str.starts_with("0") && num_str.len() > 1 {
                        u64::from_str_radix(&num_str[1..], 8).unwrap_or(0)
                    } else {
                        num_str.parse().unwrap_or(0)
                    };

                    let result = self.context.i32_type().const_int(value, false);
                    return result;
                }
                Rule::LVal => {
                    let mut lval_inner = item.into_inner();
                    let ident = lval_inner.next().unwrap().as_str();

                    // 首先检查是否是局部变量
                    if let Some(var_ptr) = self.find_variable(ident) {
                        // 加载局部变量的值
                        return self
                            .builder
                            .build_load(var_ptr, ident)
                            .unwrap()
                            .into_int_value();
                    } else {
                        // 尝试作为全局变量处理
                        if let Some(global_var) = self.module.get_global(ident) {
                            // 加载全局变量的值
                            return self
                                .builder
                                .build_load(global_var.as_pointer_value(), ident)
                                .unwrap()
                                .into_int_value();
                        } else {
                            // 变量未定义，返回0
                            return self.context.i32_type().const_int(0, false);
                        }
                    }
                }
                _ => {
                    // 处理其他基本表达式类型
                }
            }
        }

        // 默认返回0
        return self.context.i32_type().const_int(0, false);
    }

    // 输出LLVM IR到文件
    pub fn write_to_file(&self, filename: &str) -> Result<(), String> {
        self.module
            .print_to_file(filename)
            .map_err(|e| e.to_string())
    }
}

pub fn parse(input: &str, output_filename: &str) -> Result<(), String> {
    match SysYParser::parse(Rule::program, &input) {
        Ok(mut pairs) => {
            // 创建LLVM上下文
            let context = Context::create();

            // 创建编译器实例
            let mut generator = LlvmIRGen::new(&context, "module");

            // 获取程序根节点
            let program = pairs.next().unwrap();

            // 遍历程序中的声明和函数定义
            for item in program.into_inner().next().unwrap().into_inner() {
                match item.as_rule() {
                    Rule::FuncDef => {
                        generator.parse_func_def(item);
                    }
                    Rule::Decl => {
                        generator.parse_decl(item, true);
                    }
                    _ => continue,
                }
            }

            // 输出LLVM IR到文件
            generator.write_to_file(output_filename)?;

            Ok(())
        }
        Err(e) => Err(format!("Parse error: {:?}", e)),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::parser::parse;

    #[test]
    fn test_parse_part1() {
        let input = fs::read_to_string("tests/test1.sysy").expect("Failed to read file");
        parse(&input, "tests/test1.ll").expect("Failed to parse");
        let output = fs::read_to_string("tests/test1.ll").expect("Failed to read file");
        let expect_output = fs::read_to_string("tests/output1.ll").expect("Failed to read file");
        assert_eq!(output, expect_output);

        let input = fs::read_to_string("tests/test2.sysy").expect("Failed to read file");
        parse(&input, "tests/test2.ll").expect("Failed to parse");
        let output = fs::read_to_string("tests/test2.ll").expect("Failed to read file");
        let expect_output = fs::read_to_string("tests/output2.ll").expect("Failed to read file");
        assert_eq!(output, expect_output);
    }

    #[test]
    fn test_parse_part2() {
        let input = fs::read_to_string("tests/test3.sysy").expect("Failed to read file");
        parse(&input, "tests/test3.ll").expect("Failed to parse");
        let output = fs::read_to_string("tests/test3.ll").expect("Failed to read file");
        let expect_output = fs::read_to_string("tests/output3.ll").expect("Failed to read file");
        assert_eq!(output, expect_output);
        let input = fs::read_to_string("tests/test4.sysy").expect("Failed to read file");
        parse(&input, "tests/test4.ll").expect("Failed to parse");
        let output = fs::read_to_string("tests/test4.ll").expect("Failed to read file");
        let expect_output = fs::read_to_string("tests/output4.ll").expect("Failed to read file");
        assert_eq!(output, expect_output);
    }
}
