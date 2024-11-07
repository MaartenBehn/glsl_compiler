extern crate proc_macro;

use proc_macro2::{Span, TokenTree};
use proc_macro_error::{abort_call_site, emit_call_site_error, emit_error, proc_macro_error};
use std::str::FromStr;

enum Token {
    None,
    Type(bool),
    Code(bool),
}

#[proc_macro_error(proc_macro_hack)]
#[proc_macro]
pub fn glsl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let mut current_token = Token::None;
    let mut type_token = Token::None;
    let mut code_token = Token::None;

    let mut type_text = None;
    let mut code_text = None;
    let mut code_token_tree = None;

    for token in input.into_iter(){
        let text = token.span().source_text().unwrap();

        if text == "," ||text == ";" {
            continue
        }

        if text == "type"{
            current_token = Token::Type(false);
            type_token = Token::Type(false);
        } else if text == "code" {
            current_token = Token::Code(false);
            code_token = Token::Code(false);
        } else if text == "=" {
            match current_token {
                Token::Type(false) => {
                    current_token = Token::Type(true);
                    type_token = Token::Type(true);
                }
                Token::Code(false) => {
                    current_token = Token::Code(true);
                    code_token = Token::Code(true);
                }
                _ => {}
            }
        } else {
            match current_token {
                Token::Type(true) => {
                    type_text = Some(text);
                }
                Token::Code(true) => {
                    let t = text.strip_prefix("{");
                    if t.is_none() { continue }
                    let t = t.unwrap().strip_suffix("}");
                    if t.is_none() { continue }
                    code_text = Some(t.unwrap().to_string());
                    code_token_tree = Some(token);
                }
                _ => {}
            }
        }
    }

    // Check type Key
    let type_write_help = "Write: type = <shader type>";
    let type_possible_value_help = "Possible shader types: Compute, Vertex, Fragment, Geometry, Mesh, RayGeneration, AnyHit, ClosestHit, Miss";
    let glsl_type = match type_token {
        Token::None => {abort_call_site!("Key missing: type"; help=type_write_help; note=type_possible_value_help)}
        Token::Type(false) => {abort_call_site!("Invalid Key: type"; help=type_write_help; note=type_possible_value_help)}
        Token::Type(true) => {
            if type_text.is_none() {
                abort_call_site!("Missing Value for: type ="; help=type_write_help; note=type_possible_value_help)
            }

            if type_text == Some("Compute".to_string()) {
                shaderc::ShaderKind::Compute
            } else if type_text == Some("Vertex".to_string()) {
                shaderc::ShaderKind::Vertex
            }else if type_text == Some("Fragment".to_string()) {
                shaderc::ShaderKind::Fragment
            } else if type_text == Some("Geometry".to_string()) {
                shaderc::ShaderKind::Geometry
            } else if type_text == Some("Mesh".to_string()) {
                shaderc::ShaderKind::Mesh
            } else if type_text == Some("RayGeneration".to_string()) {
                shaderc::ShaderKind::RayGeneration
            } else if type_text == Some("AnyHit".to_string()) {
                shaderc::ShaderKind::AnyHit
            } else if type_text == Some("ClosestHit".to_string()) {
                shaderc::ShaderKind::ClosestHit
            } else if type_text == Some("Miss".to_string()) {
                shaderc::ShaderKind::Miss
            } else {
                abort_call_site!("Invalid type Value: {}", type_text.unwrap(); help=type_possible_value_help;)
            }
        }
        _ => {unreachable!()}
    };

    // Check code Key
    let code_write_help = "Write: code = {<glsl>}";
    let source = match code_token {
        Token::None => {abort_call_site!("Key missing: code"; help=code_write_help)}
        Token::Code(false) => {abort_call_site!("Invalid Key: code"; help=code_write_help)}
        Token::Code(true) => {
            if code_text.is_none() {
                abort_call_site!("Missing Value for: code ="; help=code_write_help)
            }
            code_text.unwrap()
        }
        _ => {unreachable!()}
    };

    //println!("Shader input {source}");
    let compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();
    let binary_result = compiler.compile_into_spirv(
        &source,
        glsl_type,
        "shader.glsl",
        "main", Some(&options));

    if binary_result.is_err() {
        let err = binary_result.err().unwrap().to_string();
        let err_lines: Vec<_> = err.split("shader.glsl:").collect();

        let code_token_tree = code_token_tree.unwrap();
        
        for err_line in err_lines.iter().skip(1) {
            let parts: Vec<_> = err_line.split(":").collect();
            //println!("Err Message Parts {parts:?}");
            
            let line = parts[0].parse::<usize>().unwrap();
            let key = parts[2].strip_prefix(" '").unwrap().strip_suffix("' ").unwrap();
            //println!("Error line {line}, Error key {key}");
            
            let (span, _, _) = find_best_line(&source, code_token_tree.clone(), key,0, line - 1);
            if span.is_some() {
                emit_error!(span.unwrap(), "{}", parts[3])
            } else {
                emit_call_site_error!("{}", parts[3])
            }
        }
        
        proc_macro::TokenStream::from_str(&"()").unwrap()
    } else {
        let mut res = "&[".to_string();
        for byte in binary_result.unwrap().as_binary_u8() {
            res = format!("{res}{byte},");
        }
        res = format!("{res}]");

        proc_macro::TokenStream::from_str(&res).unwrap()
    }
}

fn find_best_line<'a>(mut source: &'a str, t: TokenTree, key: &'a str, mut current_line: usize, line: usize) -> (Option<Span>, &'a str, usize) {
    
    let mut check = |span: Span| {
        let text = span.source_text().unwrap();
        let position = source.find(&text).unwrap();
        let lines = source[..position].chars().filter(|c| *c == '\n').count();

        current_line += lines;
        source = &source[position..];
        
        if (key == "" || text.contains(key)) && current_line >= line {
            //println!("Found Key {text} at: {current_line}");
            (Some(span), source, current_line)
        } else {
            (None, source, current_line)
        }
    };
    
    match t {
        TokenTree::Group(g) => {
            for t in g.stream() {
                let (res, new_source, new_current_line) = find_best_line(source, t, key, current_line, line);
                source = new_source;
                current_line = new_current_line;
                
                if res.is_some() {
                    return (res, source, current_line)
                }
            }
        }
        TokenTree::Ident(n) => {return check(n.span())}
        TokenTree::Punct(n) => {return check(n.span())}
        TokenTree::Literal(n) => {return check(n.span())}
    }

    (None, source, current_line)
}