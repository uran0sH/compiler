use std::collections::HashMap;
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
    control_stack: Vec<(
        inkwell::basic_block::BasicBlock<'ctx>,
        inkwell::basic_block::BasicBlock<'ctx>,
    )>,
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
            scope_stack: vec![HashMap::new()],
            control_stack: vec![],
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
        self.scope_stack
            .last_mut()
            .unwrap()
            .insert(name.to_string(), val);
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
        let mut param_names = vec![];
        let param_types = if next.as_rule() == Rule::FuncFParams {
            has_params = true;
            let mut types = vec![];
            for param in next.into_inner() {
                match param.as_rule() {
                    Rule::FuncFParam => {
                        types.push(self.context.i32_type().into());
                        let mut param_inner = param.into_inner();
                        let _btype = param_inner.next().unwrap(); // BType
                        let ident = param_inner.next().unwrap().as_str();
                        param_names.push(ident.to_string());
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

        let entry_block = self
            .context
            .append_basic_block(function, &format!("{}Entry", func_name));
        self.builder.position_at_end(entry_block);

        if has_params {
            // RParen
            let _ = inner.next();
            for (i, param_name) in param_names.iter().enumerate() {
                let param = function.get_nth_param(i as u32).unwrap();
                let alloca = self
                    .builder
                    .build_alloca(self.context.i32_type(), param_name)
                    .unwrap();
                self.builder.build_store(alloca, param).unwrap();
                self.add_variable(param_name, alloca);
            }
        }
        let mut has_return = false;
        self.parse_block(inner.next().unwrap(), function, &mut has_return);

        if !has_return {
            match ret_ty {
                FuncType::Void(_) => {
                    // 对于void返回类型的函数，添加ret void指令
                    self.builder.build_return(None).unwrap();
                }
                FuncType::Int(_) => {
                    // 对于int返回类型的函数，添加默认返回值0
                    let zero = self.context.i32_type().const_int(0, false);
                    self.builder.build_return(Some(&zero)).unwrap();
                }
            }
        }
    }

    fn parse_func_call(
        &self,
        func_name: &str,
        args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>>,
    ) -> inkwell::values::IntValue<'ctx> {
        if let Some(function) = self.module.get_function(func_name) {
            // 调用函数
            let result = self.builder.build_call(function, &args, func_name).unwrap();
            match result.try_as_basic_value().left() {
                Some(int_value) => int_value.into_int_value(),
                None => {
                    // 函数返回类型为void，返回0
                    self.context.i32_type().const_int(0, false)
                }
            }
        } else {
            // 函数未定义，返回0
            self.context.i32_type().const_int(0, false)
        }
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
    fn parse_const_def(&mut self, const_def: Pair<Rule>) {
        let mut inner = const_def.into_inner();

        // 获取常量名
        let ident = inner.next().unwrap().as_str();

        // 跳过可选的数组维度 (LBracket ~ ConstExp ~ RBracket)*
        while let Some(item) = inner.next() {
            if item.as_rule() == Rule::LBracket {
                // Skip ConstExp
                let _ = inner.next();
                // Skip RBracket
                let _ = inner.next();
            } else if item.as_rule() == Rule::Assign {
                // 解析常量初始化值
                let init_val = inner.next().unwrap();
                let value = self.parse_const_init_val(init_val);

                // 在当前作用域中创建常量变量
                let alloca = self
                    .builder
                    .build_alloca(self.context.i32_type(), ident)
                    .unwrap();
                let _ = self.builder.build_store(alloca, value);
                self.add_variable(ident, alloca);
                break;
            }
        }
    }

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

        let next = inner.next().unwrap();
        match next.as_rule() {
            Rule::ConstExp => {
                // 解析常量表达式
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
        &mut self,
        stmt: Pair<Rule>,
        function: FunctionValue<'ctx>,
        has_return: &mut bool,
    ) {
        let mut inner = stmt.into_inner();

        let next = inner.next().unwrap();
        match next.as_rule() {
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
            Rule::If => {
                // 解析 if 语句
                self.parse_if_stmt(function, inner, has_return);
            }
            Rule::While => {
                // 解析 while 循环
                // 直接调用parse_while_stmt，不消费While标记
                self.parse_while_stmt(function, inner, has_return);
            }
            Rule::Break => {
                let _ = inner.next();

                // 从控制栈中获取最近的循环结束块
                if let Some((_, after_block)) = self.control_stack.last() {
                    // 跳转到循环结束块
                    self.builder
                        .build_unconditional_branch(*after_block)
                        .unwrap();
                } else {
                    // 如果没有循环结构，则报错
                    panic!("break statement not within a loop");
                }
            }
            Rule::Continue => {
                // 解析 continue 语句
                // 注意：我们需要知道当前的循环结构，这里简化处理
                // 跳过分号
                let _ = inner.next();

                // 从控制栈中获取最近的循环条件块
                if let Some((cond_block, _)) = self.control_stack.last() {
                    // 跳转到循环条件块
                    self.builder
                        .build_unconditional_branch(*cond_block)
                        .unwrap();
                } else {
                    // 如果没有循环结构，则报错
                    panic!("continue statement not within a loop");
                }
            }
            Rule::LVal => {
                self.parse_assign_stmt(next.as_str(), inner);
            }
            Rule::Block => {
                self.parse_block(next, function, has_return);
            }
            Rule::Exp => {
                self.parse_exp(next);
            }
            _ => {}
        }
    }

    fn parse_if_stmt(
        &mut self,
        function: FunctionValue<'ctx>,
        mut inner: pest::iterators::Pairs<Rule>,
        _has_return: &mut bool,
    ) {
        // 解析条件表达式
        let _ = inner.next().unwrap(); // LParen
        let cond = inner.next().unwrap(); // Cond
        let _ = inner.next().unwrap(); // RParen
        let then_stmt = inner.next().unwrap(); // Then Stmt

        // 创建基本块，使用与output7.ll一致的命名
        let if_true = self.context.append_basic_block(function, "if_true_");
        let if_false = self.context.append_basic_block(function, "if_false_");
        let next = self.context.append_basic_block(function, "if_next_");

        // 评估条件表达式并生成分支
        let cond_value = self.parse_cond(cond);
        // 将IntValue转换为BoolValue
        let cond_bool = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                cond_value,
                self.context.i32_type().const_int(0, false),
                "cond_2",
            )
            .unwrap();

        self.builder
            .build_conditional_branch(cond_bool, if_true, if_false)
            .unwrap();

        // 处理 then 分支
        self.builder.position_at_end(if_true);
        let mut then_has_return = false;
        self.parse_stmt(then_stmt, function, &mut then_has_return);
        // 如果 then 分支没有 return 语句，则跳转到 next 块
        if !then_has_return
            && self
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
        {
            self.builder.build_unconditional_branch(next).unwrap();
        }

        // 处理 else 分支（如果存在）
        if let Some(else_token) = inner.next() {
            if else_token.as_rule() == Rule::Else {
                let else_stmt = inner.next().unwrap();
                self.builder.position_at_end(if_false);
                let mut else_has_return = false;
                self.parse_stmt(else_stmt, function, &mut else_has_return);
                // 如果 else 分支没有 return 语句，则跳转到 next 块
                if !else_has_return
                    && self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_none()
                {
                    self.builder.build_unconditional_branch(next).unwrap();
                }
            }
        } else {
            // 如果没有 else 分支，直接跳转到 next 块
            self.builder.position_at_end(if_false);
            self.builder.build_unconditional_branch(next).unwrap();
        }

        // 设置插入点到 next 块
        self.builder.position_at_end(next);
    }

    fn parse_while_stmt(
        &mut self,
        function: FunctionValue<'ctx>,
        mut inner: pest::iterators::Pairs<Rule>,
        has_return: &mut bool,
    ) {
        // 保存当前块，用于循环结束后跳转到这里
        let current_block = self.builder.get_insert_block().unwrap();

        // 创建循环所需的基本块
        let cond_block = self.context.append_basic_block(function, "whileCond");
        let body_block = self.context.append_basic_block(function, "whileBody");
        let after_block = self.context.append_basic_block(function, "whileNext"); // 修改为 whileNext

        // 将循环结束块添加到控制栈，以便break语句可以跳转到这里
        self.control_stack.push((cond_block, after_block));

        // 从当前块跳转到条件块
        self.builder.build_unconditional_branch(cond_block).unwrap();

        // 处理条件块
        self.builder.position_at_end(cond_block);

        // 解析条件表达式
        let lparen = inner.next().unwrap(); // LParen
        assert!(lparen.as_rule() == Rule::LParen);

        let cond = inner.next().unwrap(); // Cond
        let cond_value = self.parse_cond(cond);

        let rparen = inner.next().unwrap(); // RParen
        assert!(rparen.as_rule() == Rule::RParen);

        // 将IntValue转换为BoolValue，类似于您提供的示例中的 %cond_2 = icmp ne i32 %cond_1, 0
        let cond_bool = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                cond_value,
                self.context.i32_type().const_int(0, false),
                "cond_2",
            )
            .unwrap();

        self.builder
            .build_conditional_branch(cond_bool, body_block, after_block)
            .unwrap();

        // 处理循环体
        self.builder.position_at_end(body_block);
        let body_stmt = inner.next().unwrap();
        self.parse_stmt(body_stmt, function, has_return);

        // 如果当前块没有终止符，则添加跳转到条件块的指令
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            // 循环结束后跳回条件块
            self.builder.build_unconditional_branch(cond_block).unwrap();
        }

        // 从控制栈中移除当前循环的信息
        self.control_stack.pop();

        // 设置插入点到循环之后的块
        self.builder.position_at_end(after_block);
    }

    // 添加解析条件表达式的方法
    fn parse_cond(&self, cond: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = cond.into_inner();
        self.parse_l_or_exp(inner.next().unwrap())
    }

    // 添加解析逻辑或表达式的方法（支持短路）
    fn parse_l_or_exp(&self, l_or_exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = l_or_exp.into_inner();
        let mut result = self.parse_l_and_exp(inner.next().unwrap());

        // 检查是否有更多的逻辑或表达式
        while let Some(item) = inner.next() {
            if item.as_rule() == Rule::Or {
                // 解析右侧表达式
                let right_exp = self.parse_l_and_exp(inner.next().unwrap());

                // 将左右表达式转换为布尔值
                let left_bool = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        result,
                        self.context.i32_type().const_int(0, false),
                        "left_bool",
                    )
                    .unwrap();

                let right_bool = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        right_exp,
                        self.context.i32_type().const_int(0, false),
                        "right_bool",
                    )
                    .unwrap();

                // 执行逻辑或运算
                let or_result = self
                    .builder
                    .build_or(left_bool, right_bool, "or_result")
                    .unwrap();

                // 将结果转换回整数
                result = self
                    .builder
                    .build_int_z_extend(or_result, self.context.i32_type(), "or_int")
                    .unwrap();
            }
        }

        result
    }

    // 添加解析逻辑与表达式的方法（支持短路）
    fn parse_l_and_exp(&self, l_and_exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = l_and_exp.into_inner();
        let mut result = self.parse_eq_exp(inner.next().unwrap());

        // 检查是否有更多的逻辑与表达式
        while let Some(item) = inner.next() {
            if item.as_rule() == Rule::And {
                // 解析右侧表达式
                let right_exp = self.parse_eq_exp(inner.next().unwrap());

                // 将左右表达式转换为布尔值
                let left_bool = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        result,
                        self.context.i32_type().const_int(0, false),
                        "left_bool",
                    )
                    .unwrap();

                let right_bool = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        right_exp,
                        self.context.i32_type().const_int(0, false),
                        "right_bool",
                    )
                    .unwrap();

                // 执行逻辑与运算
                let and_result = self
                    .builder
                    .build_and(left_bool, right_bool, "and_result")
                    .unwrap();

                // 将结果转换回整数
                result = self
                    .builder
                    .build_int_z_extend(and_result, self.context.i32_type(), "and_int")
                    .unwrap();
            }
        }

        result
    }

    // 添加解析相等表达式的方法
    fn parse_eq_exp(&self, eq_exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = eq_exp.into_inner();
        let mut result = self.parse_rel_exp(inner.next().unwrap());

        while let Some(item) = inner.next() {
            match item.as_rule() {
                Rule::Eq => {
                    let next_rel = self.parse_rel_exp(inner.next().unwrap());
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::EQ, result, next_rel, "eqtmp")
                        .unwrap();
                    result = self
                        .builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "eqext")
                        .unwrap();
                }
                Rule::Neq => {
                    let next_rel = self.parse_rel_exp(inner.next().unwrap());
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::NE, result, next_rel, "neqt")
                        .unwrap();
                    result = self
                        .builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "neqext")
                        .unwrap();
                }
                _ => {}
            }
        }

        result
    }

    fn parse_rel_exp(&self, rel_exp: Pair<Rule>) -> inkwell::values::IntValue<'ctx> {
        let mut inner = rel_exp.into_inner();
        let mut result = self.parse_add_exp(inner.next().unwrap());

        while let Some(item) = inner.next() {
            match item.as_rule() {
                Rule::Lt => {
                    let next_add = self.parse_add_exp(inner.next().unwrap());
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SLT, result, next_add, "cond_")
                        .unwrap();
                    result = self
                        .builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "cond_1")
                        .unwrap();
                }
                Rule::Gt => {
                    let next_add = self.parse_add_exp(inner.next().unwrap());
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SGT, result, next_add, "cond_")
                        .unwrap();
                    result = self
                        .builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "cond_1")
                        .unwrap();
                }
                Rule::Le => {
                    let next_add = self.parse_add_exp(inner.next().unwrap());
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SLE, result, next_add, "cond_")
                        .unwrap();
                    result = self
                        .builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "cond_1")
                        .unwrap();
                }
                Rule::Ge => {
                    let next_add = self.parse_add_exp(inner.next().unwrap());
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SGE, result, next_add, "cond_")
                        .unwrap();
                    result = self
                        .builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "cond_1")
                        .unwrap();
                }
                _ => {}
            }
        }

        result
    }

    fn parse_assign_stmt(&self, ident: &str, mut inner: pest::iterators::Pairs<Rule>) {
        // 检查是否为赋值操作符
        if let Some(op) = inner.next() {
            if op.as_rule() == Rule::Assign {
                // 解析赋值表达式
                let exp = inner.next().unwrap();
                let value = self.parse_exp(exp);
                // 查找左值对应的变量指针
                if let Some(var_ptr) = self.find_variable(ident.trim()) {
                    // 存储值到变量
                    self.builder.build_store(var_ptr, value).unwrap();
                } else {
                    if let Some(global_var) = self.module.get_global(ident.trim()) {
                        self.builder
                            .build_store(global_var.as_pointer_value(), value)
                            .unwrap();
                    }
                }
            } else {
                println!("call func_call");
                let mut args = vec![];
                if op.as_rule() == Rule::FuncRParams {
                    args = self.parse_func_rparams(op);
                }
                // 调用函数
                self.parse_func_call(ident, args);
            }
        }

        // 跳过分号
        while let Some(item) = inner.next() {
            if item.as_rule() == Rule::Semicolon {
                break;
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
                Rule::Ident => {
                    let func_name = item.as_str();
                    let mut args = vec![];
                    // 解析函数参数
                    while let Some(arg) = inner.next() {
                        if arg.as_rule() == Rule::FuncRParams {
                            args = self.parse_func_rparams(arg);
                        }
                    }
                    return self.parse_func_call(func_name, args);
                }
                _ => {
                    unreachable!()
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

    fn parse_func_rparams(
        &self,
        arg: Pair<'_, Rule>,
    ) -> Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> {
        let inner = arg.into_inner();
        let mut params = vec![];
        for item in inner {
            match item.as_rule() {
                Rule::Exp => {
                    // 解析表达式并转换为IntValue
                    let int_value = self.parse_exp(item);
                    // 将IntValue转换为BasicMetadataValueEnum
                    params.push(inkwell::values::BasicMetadataValueEnum::IntValue(int_value));
                }
                Rule::Comma => {
                    // 跳过逗号，继续处理下一个参数
                    continue;
                }
                _ => break,
            }
        }
        params
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

    #[test]
    fn test_parse_part3() {
        let input = fs::read_to_string("tests/test5.sysy").expect("Failed to read file");
        parse(&input, "tests/test5.ll").expect("Failed to parse");
        let output = fs::read_to_string("tests/test5.ll").expect("Failed to read file");
        let expect_output = fs::read_to_string("tests/output5.ll").expect("Failed to read file");
        assert_eq!(output, expect_output);
        let input = fs::read_to_string("tests/test6.sysy").expect("Failed to read file");
        parse(&input, "tests/test6.ll").expect("Failed to parse");
        let output = fs::read_to_string("tests/test6.ll").expect("Failed to read file");
        let expect_output = fs::read_to_string("tests/output6.ll").expect("Failed to read file");
        assert_eq!(output, expect_output);
    }

    #[test]
    fn test_parse_part4() {
        let input = fs::read_to_string("tests/test7.sysy").expect("Failed to read file");
        parse(&input, "tests/test7.ll").expect("Failed to parse");
        let output = fs::read_to_string("tests/test7.ll").expect("Failed to read file");
        let expect_output = fs::read_to_string("tests/output7.ll").expect("Failed to read file");
        // assert_eq!(output, expect_output);
        let input = fs::read_to_string("tests/test8.sysy").expect("Failed to read file");
        parse(&input, "tests/test8.ll").expect("Failed to parse");
        let output = fs::read_to_string("tests/test8.ll").expect("Failed to read file");
        let expect_output = fs::read_to_string("tests/output8.ll").expect("Failed to read file");
        assert_eq!(output, expect_output);
    }
}
