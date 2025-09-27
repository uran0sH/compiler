use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use pest::{Parser, iterators::Pair};
use pest_derive::Parser;

#[macro_export]
macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl std::convert::TryFrom<usize> for $name {
            type Error = ();

            fn try_from(v: usize) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as usize => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

back_to_enum! {
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorCode {
    UndeclaredVar = 1,
    UndefinedFunc,
    RepeatedDeclaredVar,
    RepeatedDefinedFunc,
    UnmatchedAssign,
    UnmatchedOp,
    UnmatchedReturn,
    NotApplicableParams,
    SubscriptOnNonArray,
    FuncCallOnVar,
    NoArrayOrElemOnLeft,
}
}

impl From<ErrorCode> for usize {
    fn from(value: ErrorCode) -> Self {
        match value {
            ErrorCode::UndeclaredVar => 1,
            ErrorCode::UndefinedFunc => 2,
            ErrorCode::RepeatedDeclaredVar => 3,
            ErrorCode::RepeatedDefinedFunc => 4,
            ErrorCode::UnmatchedAssign => 5,
            ErrorCode::UnmatchedOp => 6,
            ErrorCode::UnmatchedReturn => 7,
            ErrorCode::NotApplicableParams => 8,
            ErrorCode::SubscriptOnNonArray => 9,
            ErrorCode::FuncCallOnVar => 10,
            ErrorCode::NoArrayOrElemOnLeft => 11,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    Int,
    Array(ArrayType),
    Function(FunctionType),
}

impl Type {
    fn array_dimension(&self) -> usize {
        match self {
            Type::Array(arr) => 1 + &arr.contained.array_dimension(),
            _ => 0,
        }
    }
}

/// 函数类型：保存返回值类型和参数类型
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub ret_ty: Box<Type>,      // 返回值类型（实验保证不是数组）
    pub params_type: Vec<Type>, // 参数列表
}

impl FunctionType {
    pub fn new(ret_ty: Type) -> Self {
        Self {
            ret_ty: Box::new(ret_ty),
            params_type: Vec::new(),
        }
    }

    pub fn add_params(&mut self, param_type: Type) {
        self.params_type.push(param_type);
    }

    pub fn get_params(&self) -> &Vec<Type> {
        &self.params_type
    }
}

/// 数组类型：保存元素类型和元素数量
#[derive(Debug, Clone)]
pub struct ArrayType {
    pub contained: Box<Type>,        // 元素类型，可能是 Int 或 Array
    pub num_elements: Option<usize>, // 元素数量
}

impl PartialEq for ArrayType {
    fn eq(&self, other: &Self) -> bool {
        (*self.contained == Type::Int
            && *other.contained == Type::Int
            && (self.num_elements == None || other.num_elements == None))
            || self.contained == other.contained && self.num_elements == other.num_elements
    }
}

impl ArrayType {
    pub fn new(elems: Vec<usize>) -> Self {
        if elems.is_empty() {
            panic!("ArrayType::new called with empty dimensions");
        }

        // 从最内层开始构建：int[5] → int[4][5] → int[3][4][5]
        let mut innermost = Type::Int;

        // 从最后一个维度开始，反向构建
        for &dim in elems.iter().rev() {
            innermost = Type::Array(ArrayType {
                contained: Box::new(innermost),
                num_elements: Some(dim),
            });
        }

        match innermost {
            Type::Array(arr) => arr,
            _ => unreachable!(), // 因为 elems 非空，结果一定是 Array
        }
    }
}

pub struct Analyzer {
    scope_stack: Vec<HashMap<String, Type>>,
    function_table: HashMap<String, FunctionType>,
    current_function_ret_ty: Option<Type>,
    current_function_params: Option<HashMap<String, Type>>,
    is_error: bool,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            scope_stack: vec![HashMap::new()],
            function_table: HashMap::new(),
            current_function_ret_ty: None,
            current_function_params: None,
            is_error: false,
        }
    }

    fn current_scope_mut(&mut self) -> &mut HashMap<String, Type> {
        self.scope_stack.last_mut().unwrap()
    }

    fn enter_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
    }

    fn exit_socpe(&mut self) {
        self.scope_stack.pop();
    }

    fn declare_var(&mut self, name: String, ty: Type) -> bool {
        if self.current_scope_mut().contains_key(&name) {
            return false;
        }
        self.current_scope_mut().insert(name, ty);
        true
    }

    fn lookup_var(&self, name: &str) -> Option<&Type> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    fn declare_function(&mut self, name: String, func_ty: FunctionType) -> Result<(), String> {
        if self.function_table.contains_key(&name) || self.scope_stack[0].contains_key(&name) {
            return Err(format!("Function '{}' already defined", name));
        }
        self.function_table.insert(name, func_ty);
        Ok(())
    }

    fn lookup_function(&self, name: &str) -> Option<&FunctionType> {
        self.function_table.get(name)
    }

    fn lookup_function_mut(&mut self, name: &str) -> Option<&mut FunctionType> {
        self.function_table.get_mut(name)
    }
}

fn report_semantic_error(error_code: usize, line: usize, msg: &str) {
    eprintln!("Error type {} at Line {}: {}", error_code, line, msg);
}

#[derive(Parser)]
#[grammar = "parser.pest"]
pub struct SysYParser;

pub fn parse(input: &str) {
    match SysYParser::parse(Rule::program, &input) {
        Ok(mut pairs) => {
            let mut analyzer = Analyzer::new();
            check_semantic(pairs.next().unwrap(), &mut analyzer);
            if !analyzer.is_error {
                eprintln!("No semantic errors in the program!");
            }
        }
        Err(e) => {
            panic!();
        }
    }
}

fn check_semantic(pair: Pair<Rule>, a: &mut Analyzer) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::CompUnit => {
                visit_comp_unit(pair_inner, a);
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }
}

fn visit_comp_unit(pair: Pair<Rule>, a: &mut Analyzer) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Decl => {
                visit_decl(pair_inner, a);
            }
            Rule::FuncDef => {
                visit_func_def(pair_inner, a);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_decl(pair: Pair<Rule>, a: &mut Analyzer) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::ConstDecl => {
                visit_const_decl(pair_inner, a);
            }
            Rule::VarDecl => {
                visit_var_decl(pair_inner, a);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_const_decl(pair: Pair<Rule>, a: &mut Analyzer) {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    // let btype = inner_pairs[1].clone();
    let line_col = inner_pairs[2].line_col();
    let const_def = inner_pairs[2].clone();
    let (ident, ty) = match visit_const_def(const_def, a) {
        Some((v, t)) => (v, t),
        None => {
            a.is_error = true;
            return;
        }
    };
    if !a.declare_var(ident, ty) {
        a.is_error = true;
        report_semantic_error(
            ErrorCode::RepeatedDeclaredVar.into(),
            line_col.0,
            "repeated declared var",
        );
    }
    for i in 3..l - 1 {
        if inner_pairs[i].as_rule() == Rule::ConstDef {
            let line_col = inner_pairs[i].line_col();
            let (ident, ty) = match visit_const_def(inner_pairs[i].clone(), a) {
                Some((v, t)) => (v, t),
                None => {
                    a.is_error = true;
                    return;
                }
            };
            if !a.declare_var(ident, ty) {
                a.is_error = true;
                report_semantic_error(
                    ErrorCode::RepeatedDeclaredVar.into(),
                    line_col.0,
                    "repeated declared var",
                );
            }
        }
    }
}

fn visit_const_def(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(String, Type)> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let line_col = inner_pairs[0].line_col();
    let l = inner_pairs.len();
    let mut ty = Type::Int;
    let mut dim = vec![];
    for i in 1..l - 2 {
        if inner_pairs[i].as_rule() == Rule::ConstExp {
            match visit_const_exp(inner_pairs[i].clone(), a) {
                Some((n, _)) => dim.push(n as usize),
                None => return None,
            }
        }
    }
    if dim.len() != 0 {
        ty = Type::Array(ArrayType::new(dim));
    }
    let const_ty = visit_const_initval(inner_pairs[l - 1].clone(), a);
    if ty != const_ty {
        a.is_error = true;
        report_semantic_error(
            ErrorCode::UnmatchedAssign.into(),
            line_col.0,
            "const assign unmatched",
        );
    }
    return Some((inner_pairs[0].as_str().to_string(), ty));
}

fn visit_const_initval(pair: Pair<Rule>, a: &mut Analyzer) -> Type {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    if inner_pairs.len() == 1 {
        return Type::Int;
    }
    let mut item = vec![];
    for (_idx, pair_inner) in inner_pairs.into_iter().enumerate() {
        match pair_inner.as_rule() {
            Rule::ConstInitVal => {
                let ty = visit_const_initval(pair_inner, a);
                item.push(ty);
            }
            Rule::LBrace | Rule::Comma | Rule::RBrace => {}
            _ => unreachable!(),
        }
    }
    if item.is_empty() {
        // 空初始化列表 {} → 我们视为 int[0]
        return Type::Array(ArrayType {
            contained: Box::new(Type::Int),
            num_elements: Some(0),
        });
    } else {
        return Type::Array(ArrayType {
            contained: Box::new(item[0].clone()),
            num_elements: Some(item.len()),
        });
    }
}

fn visit_const_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(i64, Type)> {
    let add_exp = pair.into_inner().next().unwrap();
    visit_add_exp(add_exp, a)
}

fn visit_var_decl(pair: Pair<Rule>, a: &mut Analyzer) {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    // let btype = inner_pairs[1].clone();
    let line_col = inner_pairs[1].line_col();
    let const_def = inner_pairs[1].clone();
    let (ident, ty) = match visit_var_def(const_def, a) {
        Some((v, t)) => (v, t),
        None => {
            a.is_error = true;
            return;
        }
    };
    if !a.declare_var(ident, ty) {
        a.is_error = true;
        report_semantic_error(
            ErrorCode::RepeatedDeclaredVar.into(),
            line_col.0,
            "repeated declared var",
        );
    }
    for i in 2..l - 1 {
        if inner_pairs[i].as_rule() == Rule::VarDef {
            let line_col = inner_pairs[i].line_col();
            let (ident, ty) = match visit_var_def(inner_pairs[i].clone(), a) {
                Some((v, t)) => (v, t),
                None => {
                    a.is_error = true;
                    return;
                }
            };
            if !a.declare_var(ident, ty) {
                a.is_error = true;
                report_semantic_error(
                    ErrorCode::RepeatedDeclaredVar.into(),
                    line_col.0,
                    "repeated declared var",
                );
            }
        }
    }
}

fn visit_var_def(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(String, Type)> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let mut ty = Type::Int;
    let mut dim = vec![];
    let mut is_assign = false;
    for i in 1..l {
        if inner_pairs[i].as_rule() == Rule::ConstExp {
            match visit_const_exp(inner_pairs[i].clone(), a) {
                Some((n, _)) => dim.push(n as usize),
                None => return None,
            }
        }
        if inner_pairs[i].as_rule() == Rule::Assign {
            is_assign = true;
        }
    }
    if dim.len() != 0 {
        ty = Type::Array(ArrayType::new(dim));
    }
    if is_assign {
        let line_col = inner_pairs[l - 1].line_col();
        let init_ty = visit_initval(inner_pairs[l - 1].clone(), a);
        if ty != init_ty {
            a.is_error = true;
            report_semantic_error(
                ErrorCode::UnmatchedAssign.into(),
                line_col.0,
                "var assign unmatched",
            );
        }
    }
    return Some((inner_pairs[0].as_str().to_string(), ty));
}

fn visit_initval(pair: Pair<Rule>, a: &mut Analyzer) -> Type {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let mut ty = Type::Int;
    let mut initval_tys = vec![];
    if inner_pairs[0].as_rule() == Rule::Exp {
        visit_exp(inner_pairs[0].clone(), a);
    } else {
        let l = inner_pairs.len();
        for i in 0..l {
            if inner_pairs[i].as_rule() == Rule::InitVal {
                initval_tys.push(visit_initval(inner_pairs[i].clone(), a));
            }
        }
    }
    if !initval_tys.is_empty() {
        ty = Type::Array(ArrayType {
            contained: Box::new(initval_tys[0].clone()),
            num_elements: Some(initval_tys.len()),
        })
    }
    ty
}

fn visit_func_def(pair: Pair<Rule>, a: &mut Analyzer) {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let func_type = inner_pairs[0].clone();
    let ident = inner_pairs[1].clone();
    let ret_ty = visit_func_type(func_type);
    let function_ty = FunctionType::new(ret_ty.clone());
    match a.declare_function(ident.as_str().to_string(), function_ty) {
        Ok(()) => {}
        Err(e) => {
            let line_col = &ident.line_col();
            a.is_error = true;
            report_semantic_error(
                ErrorCode::RepeatedDefinedFunc.into(),
                line_col.0,
                e.as_str(),
            );
            return;
        }
    }
    if l == 6 {
        let func_fparams = inner_pairs[3].clone();
        let ident_ty = visit_func_fparams(func_fparams, a, ident.as_str());
        a.current_function_params = Some(ident_ty);
    }
    let block = inner_pairs.last().cloned().unwrap().clone();
    a.current_function_ret_ty = Some(ret_ty);
    visit_block(block, a);
    a.current_function_ret_ty = None;
    a.current_function_params = None;
}

fn visit_func_type(pair: Pair<Rule>) -> Type {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::Void => Type::Void,
        Rule::Int => Type::Int,
        _ => unreachable!(),
    }
}

fn visit_func_fparams(pair: Pair<Rule>, a: &mut Analyzer, fun_name: &str) -> HashMap<String, Type> {
    let mut ident = HashMap::new();
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::FuncFParam => {
                let line_col = &pair_inner.line_col();
                let (name, ty) = match visit_func_fparam(pair_inner, a) {
                    Some((n, t)) => (n, t),
                    None => continue,
                };
                if ident.contains_key(&name) {
                    a.is_error = true;
                    report_semantic_error(
                        ErrorCode::RepeatedDeclaredVar.into(),
                        line_col.0,
                        "Redefined variable",
                    );
                } else {
                    ident.insert(name, ty.clone());
                    match a.lookup_function_mut(fun_name) {
                        Some(v) => {
                            v.add_params(ty);
                        }
                        None => {
                            unreachable!()
                        }
                    }
                }
            }
            Rule::Comma => {}
            _ => unreachable!(),
        }
    }
    return ident;
}

fn visit_func_fparam(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(String, Type)> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let _btype = inner_pairs[0].clone();
    let ident = inner_pairs[1].clone();
    let ty = if inner_pairs.len() == 2 {
        Type::Int
    } else {
        Type::Array(ArrayType {
            contained: Box::new(Type::Int),
            num_elements: None,
        })
    };
    Some((ident.as_str().to_string(), ty))
}

fn visit_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(i64, Type)> {
    let add_exp = pair.into_inner().next().unwrap();
    visit_add_exp(add_exp, a)
}

fn visit_add_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(i64, Type)> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let line_col = inner_pairs[0].line_col();
    let (left_v, left_t) = match visit_mul_exp(inner_pairs[0].clone(), a) {
        Some((v, t)) => (v, t),
        None => return None,
    };
    let mut result = left_v;
    for i in (1..l).step_by(2) {
        if left_t != Type::Int {
            a.is_error = true;
            report_semantic_error(
                ErrorCode::UnmatchedOp.into(),
                line_col.0,
                "left's type is unmatched right's type",
            );
            return None;
        }
        let op = inner_pairs[i].clone();
        let right = inner_pairs[i + 1].clone();
        let (right_v, right_t) = match visit_mul_exp(right, a) {
            Some((v, t)) => (v, t),
            None => return None,
        };
        if right_t != Type::Int {
            a.is_error = true;
            report_semantic_error(
                ErrorCode::UnmatchedOp.into(),
                line_col.0,
                "left's type is unmatched right's type",
            );
            return None;
        }
        match op.as_rule() {
            Rule::Plus => {
                result += right_v;
            }
            Rule::Minus => {
                result -= right_v;
            }
            _ => unreachable!(),
        }
    }
    return Some((result, left_t));
}

fn visit_mul_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(i64, Type)> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let line_col = inner_pairs[0].line_col();
    let (left_v, left_t) = match visit_unary_exp(inner_pairs[0].clone(), a) {
        Some((v, t)) => (v, t),
        None => return None,
    };
    let mut result = left_v;
    for i in 1..l {
        let op = inner_pairs[i].clone();
        let right = inner_pairs[i + 1].clone();
        let (right_v, right_t) = match visit_unary_exp(right, a) {
            Some((v, t)) => (v, t),
            None => return None,
        };
        if left_t != right_t {
            a.is_error = true;
            report_semantic_error(
                ErrorCode::UnmatchedOp.into(),
                line_col.0,
                "left's type is unmatched right's type",
            );
            return None;
        }
        match op.as_rule() {
            Rule::Mul => {
                result *= right_v;
            }
            Rule::Div => {
                result /= right_v;
            }
            Rule::Mod => {
                result %= right_v;
            }
            _ => unreachable!(),
        }
    }
    return Some((result, left_t));
}

fn visit_unary_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(i64, Type)> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let line_col = inner_pairs[0].line_col();
    if inner_pairs[0].as_rule() == Rule::Ident {
        // 函数类型，需要检测形参，不匹配报函数参数不适用
        match a.lookup_var(inner_pairs[0].as_str()) {
            Some(_) => {
                a.is_error = true;
                report_semantic_error(
                    ErrorCode::FuncCallOnVar.into(),
                    line_col.0,
                    "func call on var",
                );
            }
            None => {
                // a.is_error = true;
                // report_semantic_error(
                //     ErrorCode::UndefinedFunc.into(),
                //     line_col.0,
                //     "Can't find func",
                // );
            }
        }
        let ft = match a.lookup_function(inner_pairs[0].as_str()) {
            Some(v) => v.clone(),
            None => {
                return None;
            }
        };
        let pparams = ft.get_params();
        let rparams = if inner_pairs[2].as_rule() != Rule::RParen {
            visit_rparams(inner_pairs[2].clone(), a)
        } else {
            vec![]
        };
        if pparams != &rparams {
            a.is_error = true;
            report_semantic_error(
                ErrorCode::NotApplicableParams.into(),
                line_col.0,
                "pparams != rparams",
            );
            return None;
        }
        return Some((0, *(ft.ret_ty.clone())));
    }
    if inner_pairs[0].as_rule() == Rule::UnaryOp {
        return visit_unary_exp(inner_pairs[1].clone(), a);
    }
    return visit_primary_exp(inner_pairs[0].clone(), a);
}

fn visit_rparams(pair: Pair<Rule>, a: &mut Analyzer) -> Vec<Type> {
    let mut tys = vec![];
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Exp => {
                match visit_exp(pair_inner, a) {
                    Some((_v, t)) => tys.push(t),
                    None => {}
                };
            }
            Rule::Comma => {}
            _ => unreachable!(),
        }
    }
    return tys;
}

fn visit_primary_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(i64, Type)> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    if inner_pairs[0].as_rule() == Rule::Number {
        let num_str = inner_pairs[0].as_str();
        let num = parse_string_to_i64(num_str).unwrap();
        return Some((num, Type::Int));
    }
    if inner_pairs[0].as_rule() == Rule::LVal {
        // 查明是否已经声明，如果没声明报变量未声明（ErrorCode 1)
        // 变量类型是否匹配，是否对非数组类型使用下标（ErrorCode 9)
        return visit_lval(inner_pairs[0].clone(), a);
    }
    if inner_pairs[0].as_rule() == Rule::LParen {
        return visit_exp(inner_pairs[1].clone(), a);
    }
    None
}

fn visit_lval(pair: Pair<Rule>, a: &mut Analyzer) -> Option<(i64, Type)> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let ident = inner_pairs[0].clone();
    let line_col = inner_pairs[0].line_col();
    let l = inner_pairs.len();
    let decl_typ: Type = match a.lookup_var(ident.as_str()) {
        Some(v) => v.clone(),
        None => {
            let t = match a.lookup_function(ident.as_str()) {
                Some(v) => v.clone(),
                None => {
                    a.is_error = true;
                    report_semantic_error(
                        ErrorCode::UndeclaredVar.into(),
                        line_col.0,
                        "undeclared var",
                    );
                    return None;
                }
            };
            Type::Function(t)
        }
    };
    match decl_typ {
        Type::Function(_) | Type::Int => {
            for i in 1..l {
                if inner_pairs[i].as_rule() == Rule::LBracket {
                    a.is_error = true;
                    report_semantic_error(
                        ErrorCode::SubscriptOnNonArray.into(),
                        line_col.0,
                        "Int and Array",
                    );
                    return None;
                }
            }
            return Some((0, decl_typ));
        }
        Type::Array(t) => {
            let mut ty = Type::Array(t.clone());
            for i in 1..l {
                if inner_pairs[i].as_rule() == Rule::LBracket {
                    if let Type::Array(ty_inner) = ty {
                        ty = *ty_inner.contained.clone();
                    }
                }
            }
            return Some((0, ty));
        }
        _ => {
            unreachable!()
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

fn visit_block(pair: Pair<Rule>, a: &mut Analyzer) {
    a.enter_scope();
    for pair_inner in pair.into_inner() {
        if pair_inner.as_rule() == Rule::BlockItem {
            visit_block_item(pair_inner, a);
        }
    }
    a.exit_socpe();
}

fn visit_block_item(pair: Pair<Rule>, a: &mut Analyzer) {
    let pair_inner = pair.into_inner().next().unwrap();
    if pair_inner.as_rule() == Rule::Decl {
        return visit_decl(pair_inner, a);
    }
    if pair_inner.as_rule() == Rule::Stmt {
        return visit_stmt(pair_inner, a);
    }
}

fn visit_stmt(pair: Pair<Rule>, a: &mut Analyzer) {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let line_col = inner_pairs[0].line_col();
    match inner_pairs[0].as_rule() {
        Rule::LVal => {
            let ty = match visit_lval(inner_pairs[0].clone(), a) {
                Some((_v, t)) => t,
                None => {
                    return;
                }
            };
            match ty {
                Type::Void | Type::Int | Type::Array(_) => {}
                Type::Function(_function_type) => {
                    a.is_error = true;
                    report_semantic_error(
                        ErrorCode::NoArrayOrElemOnLeft.into(),
                        line_col.0,
                        "func can't be lval",
                    );
                    return;
                }
            }
            let right_ty = match visit_exp(inner_pairs[2].clone(), a) {
                Some((_v, t)) => t,
                None => {
                    return;
                }
            };
            if ty.array_dimension() != right_ty.array_dimension() && ty != right_ty {
                a.is_error = true;
                report_semantic_error(
                    ErrorCode::UnmatchedAssign.into(),
                    line_col.0,
                    "stmt left is unmatch to right",
                );
            }
        }
        Rule::Exp => {
            visit_exp(inner_pairs[0].clone(), a);
        }
        Rule::Block => {
            visit_block(inner_pairs[0].clone(), a);
        }
        Rule::If => {
            visit_cond(inner_pairs[2].clone(), a);
            visit_stmt(inner_pairs[4].clone(), a);
            if l > 5 {
                visit_stmt(inner_pairs[l - 1].clone(), a);
            }
        }
        Rule::While => {
            visit_cond(inner_pairs[2].clone(), a);
            visit_stmt(inner_pairs[4].clone(), a);
        }
        Rule::Break | Rule::Continue => {}
        Rule::Return => {
            if inner_pairs[1].as_rule() == Rule::Exp {
                let ty = match visit_exp(inner_pairs[1].clone(), a) {
                    Some((_v, t)) => t,
                    None => {
                        return;
                    }
                };
                match a.current_function_ret_ty.clone() {
                    Some(t) => {
                        if t != ty {
                            a.is_error = true;
                            report_semantic_error(
                                ErrorCode::UnmatchedReturn.into(),
                                line_col.0,
                                "return type is unmatched",
                            );
                        }
                    }
                    None => {
                        return;
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

fn visit_cond(pair: Pair<Rule>, a: &mut Analyzer) {
    let lorexp = pair.into_inner().next().unwrap();
    visit_l_or_exp(lorexp, a);
}

fn visit_l_or_exp(pair: Pair<Rule>, a: &mut Analyzer) {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let line_col = inner_pairs[0].line_col();
    let ty = visit_l_and_exp(inner_pairs[0].clone(), a);
    for i in 1..l {
        if inner_pairs[i].as_rule() == Rule::LAndExp {
            match visit_l_and_exp(inner_pairs[i].clone(), a) {
                Some(t) => {
                    if !ty.is_none() && t != ty.clone().unwrap() {
                        report_semantic_error(
                            ErrorCode::UnmatchedOp.into(),
                            line_col.0,
                            "error rel exp",
                        );
                    }
                }
                None => {}
            };
        }
    }
}

fn visit_l_and_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<Type> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let line_col = inner_pairs[0].line_col();
    let ty = visit_eq_exp(inner_pairs[0].clone(), a);
    for i in 1..l {
        if inner_pairs[i].as_rule() == Rule::EqExp {
            match visit_eq_exp(inner_pairs[i].clone(), a) {
                Some(t) => {
                    if !ty.is_none() && t != ty.clone().unwrap() {
                        report_semantic_error(
                            ErrorCode::UnmatchedOp.into(),
                            line_col.0,
                            "error rel exp",
                        );
                    }
                }
                None => {}
            };
        }
    }
    ty
}

fn visit_eq_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<Type> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let line_col = inner_pairs[0].line_col();
    let ty = visit_rel_exp(inner_pairs[0].clone(), a);
    for i in 1..l {
        if inner_pairs[i].as_rule() == Rule::RelExp {
            match visit_rel_exp(inner_pairs[i].clone(), a) {
                Some(t) => {
                    if !ty.is_none() && t != ty.clone().unwrap() {
                        report_semantic_error(
                            ErrorCode::UnmatchedOp.into(),
                            line_col.0,
                            "error rel exp",
                        );
                    }
                }
                None => {}
            };
        }
    }
    ty
}

fn visit_rel_exp(pair: Pair<Rule>, a: &mut Analyzer) -> Option<Type> {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    let l = inner_pairs.len();
    let line_col = inner_pairs[0].line_col();
    let ty = visit_add_exp(inner_pairs[0].clone(), a);
    for i in 1..l {
        if inner_pairs[i].as_rule() == Rule::AddExp {
            match visit_add_exp(inner_pairs[i].clone(), a) {
                Some((_v, t)) => {
                    if !ty.is_none() && t != ty.clone().unwrap().1 {
                        report_semantic_error(
                            ErrorCode::UnmatchedOp.into(),
                            line_col.0,
                            "error rel exp",
                        );
                    }
                }
                None => {}
            };
        }
    }
    match ty {
        Some((_, t)) => Some(t),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::parser::parse;

    fn read_file(filename: &str) -> String {
        let input = fs::read_to_string(filename).expect("Failed to read file");
        return input;
    }

    #[test]
    fn test_normal1() {
        let input = &read_file("tests/input1.txt");
        parse(input);
    }

    #[test]
    fn test_normal2() {
        let input = &read_file("tests/input2.txt");
        parse(input);
    }

    #[test]
    fn test_normal3() {
        let input = &read_file("tests/input3.txt");
        parse(input);
    }
}
