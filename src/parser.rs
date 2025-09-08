use pest::{
    Parser,
    error::{Error, InputLocation, LineColLocation},
    iterators::Pair,
};
use pest_derive::Parser;

pub struct Formatter {
    output: String,
    indent_level: usize,
    needs_indent: bool,
    at_line_start: bool,
    at_first_line: bool,
}

impl Formatter {
    fn new() -> Self {
        Formatter {
            output: String::new(),
            indent_level: 0,
            needs_indent: true,
            at_line_start: true,
            at_first_line: true,
        }
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    fn write_str(&mut self, s: &str) {
        if self.needs_indent && self.at_line_start {
            self.output.push_str(&" ".repeat(self.indent_level * 4));
            self.needs_indent = false;
        }
        self.output.push_str(s);
        self.at_line_start = false;
    }

    fn newline(&mut self) {
        if self.output.ends_with('\n') {
            return;
        }
        self.output.push('\n');
        self.needs_indent = true;
        self.at_line_start = true;
    }

    fn space(&mut self) {
        if !self.at_line_start && !self.output.ends_with(' ') {
            self.output.push(' ');
        }
    }
}

#[derive(Parser)]
#[grammar = "parser.pest"]
pub struct SysYParser;

pub fn parse(input: &str) -> Option<String> {
    match SysYParser::parse(Rule::program, &input) {
        Ok(mut pairs) => {
            let mut formatter = Formatter::new();
            format_code(pairs.next().unwrap(), &mut formatter);
            println!("{}", formatter.output);
            return Some(formatter.output);
        }
        Err(e) => {
            report_errors(e, input, 0);
            None
        }
    }
}

fn report_errors(e: Error<Rule>, input: &str, base_line: usize) {
    let pos = match &e.location {
        InputLocation::Pos(pos) => pos,
        InputLocation::Span((pos_start, _pos_end)) => pos_start,
    };
    let line = match &e.line_col {
        LineColLocation::Pos((line, _)) => line,
        LineColLocation::Span((start_line, _), (_, _)) => start_line,
    };
    let input_char = input.chars().collect::<Vec<char>>();
    println!(
        "Error type B at Line {}: mismatched input {}",
        base_line + line,
        input_char[*pos]
    );
    let mut start = pos - 1;
    let mut end = pos + 1;
    while start > 0 && input_char[start] != '\n' {
        start -= 1;
    }
    while end < input_char.len() && input_char[end] != '\n' {
        end += 1;
    }
    let mut input_str = String::new();
    input_str.extend(&input_char[..start]);
    input_str.extend(&input_char[end..]);
    match SysYParser::parse(Rule::program, &input_str) {
        Ok(_) => return,
        Err(e) => {
            report_errors(e, &input_str, base_line + 1);
        }
    }
}

fn format_code(pairs: Pair<Rule>, formatter: &mut Formatter) {
    for pair in pairs.into_inner() {
        match pair.as_rule() {
            Rule::CompUnit => {
                visit_comp_unit(pair, formatter);
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }
}

fn visit_comp_unit(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Decl => {
                if !formatter.at_first_line {
                    formatter.newline();
                }
                visit_decl(pair_inner, formatter)
            }
            Rule::FuncDef => {
                if !formatter.at_first_line {
                    formatter.newline();
                    formatter.write_str("\n");
                }
                visit_func_def(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
        formatter.at_first_line = false;
    }
}

fn visit_func_def(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::FuncType => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Ident => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
            }
            Rule::LParen => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::FuncFParams => {
                visit_func_fparams(pair_inner, formatter);
            }
            Rule::RParen => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Block => {
                formatter.space();
                visit_block(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_func_fparams(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::FuncFParam => {
                visit_func_fparam(pair_inner, formatter);
            }
            Rule::Comma => {
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            _ => unreachable!(),
        }
    }
}

fn visit_func_fparam(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::BType => {
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            Rule::Ident => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::LBracket => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::RBracket => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Exp => {
                visit_exp(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_block(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::LBrace => {
                formatter.write_str(pair_inner.as_str());
                formatter.indent();
            }
            Rule::BlockItem => {
                visit_block_item(pair_inner, formatter);
            }
            Rule::RBrace => {
                formatter.newline();
                formatter.dedent();
                formatter.write_str(pair_inner.as_str());
            }
            _ => unreachable!(),
        }
    }
}

fn visit_block_item(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Decl => {
                formatter.newline();
                visit_decl(pair_inner, formatter);
            }
            Rule::Stmt => {
                formatter.newline();
                visit_stmt(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_stmt(pair: Pair<Rule>, formatter: &mut Formatter) {
    let inner_pairs = pair.into_inner().collect::<Vec<Pair<Rule>>>();
    if inner_pairs.is_empty() {
        return;
    }
    let first = inner_pairs[0].clone();

    match first.as_rule() {
        Rule::LVal => {
            // Assignment statement: LVal "=" Exp ";"
            visit_lval(first, formatter);
            for pair_inner in inner_pairs.into_iter().skip(1) {
                match pair_inner.as_rule() {
                    Rule::Assign => {
                        formatter.space();
                        formatter.write_str(pair_inner.as_str());
                        formatter.space();
                    }
                    Rule::Exp => {
                        visit_exp(pair_inner, formatter);
                    }
                    Rule::Semicolon => {
                        formatter.write_str(pair_inner.as_str());
                    }
                    _ => unreachable!(),
                }
            }
        }
        Rule::Exp => {
            // Expression statement: Exp ";"
            visit_exp(first, formatter);
            formatter.write_str(";");
        }
        Rule::Block => {
            visit_block(first, formatter);
        }
        Rule::If | Rule::While => {
            formatter.write_str(first.as_str());
            formatter.space();
            for pair_inner in inner_pairs.into_iter().skip(1) {
                match pair_inner.as_rule() {
                    Rule::LParen => {
                        formatter.write_str(pair_inner.as_str());
                    }
                    Rule::Cond => {
                        visit_cond(pair_inner, formatter);
                    }
                    Rule::RParen => {
                        formatter.write_str(pair_inner.as_str());
                    }
                    Rule::Stmt => {
                        let pos = pair_inner.as_span().start_pos().pos();
                        let input = pair_inner.get_input().chars().collect::<Vec<char>>();
                        if input[pos] == '{' {
                            formatter.space();
                            visit_stmt(pair_inner, formatter);
                        } else {
                            formatter.newline();
                            formatter.indent();
                            visit_stmt(pair_inner, formatter);
                            formatter.dedent();
                        }
                    }
                    Rule::Elif => {
                        formatter.newline();
                        formatter.write_str("else if ");
                    }
                    Rule::Else => {
                        formatter.newline();
                        formatter.write_str(pair_inner.as_str());
                    }
                    _ => {
                        unreachable!()
                    }
                }
            }
        }
        Rule::Break => {
            // Break statement: Break ";"
            formatter.write_str("break");
            formatter.write_str(";");
            formatter.newline();
        }
        Rule::Continue => {
            // Continue statement: Continue ";"
            formatter.write_str("continue");
            formatter.write_str(";");
            formatter.newline();
        }
        Rule::Return => {
            // Return statement: Return Exp? ";"
            formatter.newline();
            formatter.write_str(first.as_str());
            for pair_inner in inner_pairs.into_iter().skip(1) {
                match pair_inner.as_rule() {
                    Rule::Exp => {
                        formatter.space();
                        visit_exp(pair_inner, formatter);
                    }
                    Rule::Semicolon => {
                        formatter.write_str(pair_inner.as_str());
                    }
                    _ => unreachable!(),
                }
            }
        }
        _ => {
            // Empty statement: just ";"
            // formatter.write_str(";");
            // formatter.newline();
            unreachable!()
        }
    }
}

fn visit_cond(pair: Pair<Rule>, formatter: &mut Formatter) {
    visit_l_or_exp(pair.into_inner().next().unwrap(), formatter);
}

fn visit_l_or_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::LAndExp => {
                visit_l_and_exp(pair_inner, formatter);
            }
            Rule::Or => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            _ => unreachable!(),
        }
    }
}

fn visit_l_and_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::EqExp => {
                visit_eq_exp(pair_inner, formatter);
            }
            Rule::And => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            _ => unreachable!(),
        }
    }
}

fn visit_eq_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::RelExp => {
                visit_rel_exp(pair_inner, formatter);
            }
            Rule::Eq | Rule::Neq => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            _ => unreachable!(),
        }
    }
}

fn visit_rel_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::AddExp => {
                visit_add_exp(pair_inner, formatter);
            }
            Rule::Lt | Rule::Gt | Rule::Le | Rule::Ge => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            _ => unreachable!(),
        }
    }
}

fn visit_decl(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::ConstDecl => {
                visit_const_decl(pair_inner, formatter);
            }
            Rule::VarDecl => {
                visit_var_decl(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_const_decl(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Const => {
                formatter.write_str("const");
            }
            Rule::BType => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
            }
            Rule::ConstDef => {
                formatter.space();
                visit_const_def(pair_inner, formatter);
            }
            Rule::Comma => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Semicolon => {
                formatter.write_str(pair_inner.as_str());
            }
            _ => unreachable!(),
        }
    }
}

fn visit_var_decl(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::BType => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::VarDef => {
                formatter.space();
                visit_var_def(pair_inner, formatter);
            }
            Rule::Comma => {
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            Rule::Semicolon => {
                formatter.write_str(pair_inner.as_str());
            }
            _ => unreachable!(),
        }
    }
}

fn visit_var_def(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Ident => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
            }
            Rule::LBracket => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::ConstExp => {
                visit_const_exp(pair_inner, formatter);
            }
            Rule::RBracket => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Assign => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            Rule::InitVal => {
                visit_init_val(pair_inner, formatter);
            }
            _ => unreachable!()
        }
    }
}

fn visit_init_val(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Exp => {
                visit_exp(pair_inner, formatter);
            }
            Rule::LBrace => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Comma => {
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            Rule::RBrace => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::InitVal => {
                visit_init_val(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_add_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::MulExp => {
                visit_mul_exp(pair_inner, formatter);
            }
            Rule::Plus => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            Rule::Minus => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            _ => {
                unreachable!()
            }
        }
    }
}

fn visit_mul_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::UnaryExp => {
                visit_unary_exp(pair_inner, formatter);
            }
            Rule::Mul => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            Rule::Div => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            Rule::Mod => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            _ => unreachable!(),
        }
    }
}

fn visit_unary_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::PrimaryExp => {
                visit_primary_exp(pair_inner, formatter);
            }
            Rule::Ident => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
            }
            Rule::LParen => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::FuncRParams => {
                visit_func_rparams(pair_inner, formatter);
            }
            Rule::RParen => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::UnaryOp => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::UnaryExp => {
                visit_unary_exp(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_primary_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::LParen => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Exp => {
                visit_exp(pair_inner, formatter);
            }
            Rule::RParen => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::LVal => {
                visit_lval(pair_inner, formatter);
            }
            Rule::Number => {
                formatter.write_str(pair_inner.as_str());
            }
            _ => unreachable!(),
        }
    }
}

fn visit_lval(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Ident | Rule::LBracket | Rule::RBracket => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Exp => {
                visit_exp(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_func_rparams(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Exp => {
                visit_exp(pair_inner, formatter);
            }
            Rule::Comma => {
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            _ => {
                unreachable!()
            }
        }
    }
}

fn visit_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    visit_add_exp(pair.into_inner().next().unwrap(), formatter);
}

fn visit_const_def(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::Ident => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::LBracket => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::ConstExp => {
                visit_const_exp(pair_inner, formatter);
            }
            Rule::RBracket => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::Assign => {
                formatter.space();
                formatter.write_str(pair_inner.as_str());
                formatter.space();
            }
            Rule::ConstInitVal => {
                visit_const_init_val(pair_inner, formatter);
            }
            _ => unreachable!(),
        }
    }
}

fn visit_const_init_val(pair: Pair<Rule>, formatter: &mut Formatter) {
    for pair_inner in pair.into_inner() {
        match pair_inner.as_rule() {
            Rule::ConstExp => {
                visit_const_exp(pair_inner, formatter);
            }
            Rule::LBrace => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::ConstInitVal => {
                visit_const_init_val(pair_inner, formatter);
            }
            Rule::Comma => {
                formatter.write_str(pair_inner.as_str());
            }
            Rule::RBrace => {
                formatter.write_str(pair_inner.as_str());
            }
            _ => unreachable!(),
        }
    }
}

fn visit_const_exp(pair: Pair<Rule>, formatter: &mut Formatter) {
    visit_add_exp(pair.into_inner().next().unwrap(), formatter);
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
    fn test_parse_normal1() {
        let input = &read_file("/home/huangwenyu/compiler/tests/lab2/in1.txt");
        let _output = parse(input).unwrap();
    }

    #[test]
    fn test_parse_normal2() {
        let input = &read_file("/home/huangwenyu/compiler/tests/lab2/in2.txt");
        let _output = parse(input).unwrap();
    }

    #[test]
    fn test_parse_normal3() {
        let input = &read_file("/home/huangwenyu/compiler/tests/lab2/in3.txt");
        let _output = parse(input).unwrap();
    }

    #[test]
    fn test_parse_normal4() {
        let input = &read_file("/home/huangwenyu/compiler/tests/lab2/in4.txt");
        let _output = parse(input);
    }

    #[test]
    fn test_parse_normal5() {
        let input = &read_file("/home/huangwenyu/compiler/tests/lab2/in5.txt");
        let _output = parse(input).unwrap();
    }

    #[test]
    fn test_parse_normal6() {
        let input = &read_file("/home/huangwenyu/compiler/tests/lab2/in6.txt");
        let _output = parse(input).unwrap();
    }

    #[test]
    fn test_parse_error() {
        let input = "int main({
            int a[];
            return a][;
        }";
        parse(input);
    }
}
