/*!

Write GLSL Code directly in a marco!

- Compile GLSL to Spriv binary for vulkan
- Not inside a string with shit linting
- Compile-time evaluation to binary slice
- No nightly needed
- Errors with correct lines

Finally, it's possible to write GLSL directly in Rust.

```Rust 
let bin_shader_code: &[u8] = glsl!{type = Compute, code = {
    #version 450 core
    
    layout(binding = 0, rgba8) uniform writeonly image2D img;

    void main () {
        uvec2 pos = gl_GlobalInvocationID.xy;
        vec4 color = vec4(pos, 0.0, 1.0);
        
        imageStore(img, ivec2(pos), color);
    }
}};
```
### will evaluated to 
```Rust 
let bin_shader_code: &[u8] = &[3, 2, 35, 7, 0, 0, 1, 0, 11, 0, 13, 0, 36, ...];
```

## Shader Types
Mark shader type with `type = <shader type>` in marco.

Possible types
- Compute
- Vertex Fragment, Geometry, Mesh
- RayGeneration, AnyHit, ClosestHit, Miss

## Proper Errors 
```Rust 
glsl!{type = Compute, code = {
    #version 450 core

    void main () {
        uvec2 pos = gl_GlobalInvocationID.xy;
        vec4 color = vec4(pos, 0.0, 1.0);
        
        imageStore(img, ivec2(pos), colo);
    }
}};
```
### will error with: 
```shell
error:  undeclared identifier
   |
13 |             imageStore(img, ivec2(pos), colo);
   |                        ^^^

error:  undeclared identifier
   |
13 |             imageStore(img, ivec2(pos), colo);
   |                                         ^^^^

error:  no matching overloaded function found
   |
13 |             imageStore(img, ivec2(pos), colo);
   |             ^^^^^^^^^^
```


## Just compiling a glsl file at compile time
```rust
let bin: &[u8] = glsl!{type = Compute, file = "shaders/test.glsl"};
```

## Including Code from other glsl file

Example Glsl File Name: "shaders/included.glsl"
```rust
let bin: &[u8] = glsl!{type = Compute, code = {
    #version 450 core
    
    #include "shaders/included.glsl"

    layout(binding = 0, rgba8) uniform writeonly image2D img;
    void main () {
        uvec2 pos = gl_GlobalInvocationID.xy;
        imageStore(img, ivec2(pos), COLOR);
    }
}};
```

## Including Code from other Macro

Example Rust File Name: "src/main.rs"
```rust 
fn shader() {
    let bin: &[u8] = glsl!{type = Compute, code = {
        #version 450 core
        
        #include "src/main.rs-included.glsl"
    
        layout(binding = 0, rgba8) uniform writeonly image2D img;
        void main () {
            uvec2 pos = gl_GlobalInvocationID.xy;
            imageStore(img, ivec2(pos), COLOR);
        }
    }};

    println!("{:?}", bin)
}

#[allow(dead_code)]
fn included() {
    glsl!{type = Include, name = "included.glsl", code = {
        #define COLOR vec4(pos, 0.0, 1.0)
    }};
}
```
*/


extern crate proc_macro;

use std::{fs, str};
use std::path::Path;
use proc_macro2::{Span, TokenTree};
use proc_macro_error::{abort_call_site, emit_call_site_error, emit_error, proc_macro_error};
use std::str::FromStr;
use std::string::ToString;
use shaderc::{IncludeCallbackResult, IncludeType, ResolvedInclude};

enum Token {
    None,
    Type(bool),
    Code(bool),
    Name,
    File(bool),
}

const MARCO_FILE_PATH: &str = "in_marco";

/**
## Example
```Rust 
let bin_shader_code: &[u8] = glsl!{type = Compute, code = {
    #version 450 core
    
    layout(binding = 0, rgba8) uniform writeonly image2D img;

    void main () {
        uvec2 pos = gl_GlobalInvocationID.xy;
        vec4 color = vec4(pos, 0.0, 1.0);
        
        imageStore(img, ivec2(pos), color);
    }
}};
```
### will evaluated to 
```Rust 
let bin_shader_code: &[u8] = &[3, 2, 35, 7, 0, 0, 1, 0, 11, 0, 13, 0, 36, ...];
```

## Shader Types
Mark shader type with `type = <shader type>` in marco.

Possible types
- Compute
- Vertex Fragment, Geometry, Mesh
- RayGeneration, AnyHit, ClosestHit, Miss
*/
#[proc_macro_error(proc_macro_hack)]
#[proc_macro]
pub fn glsl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let mut current_token = Token::None;
    let mut type_token = Token::None;
    let mut code_token = Token::None;

    let mut type_text = None;
    let mut code_text = None;
    let mut file_text = None;
    let mut code_token_tree = None;

    for token in input.into_iter(){
        let text = token.span().source_text().unwrap();

        if text == "," || text == ";" {
            continue
        }

        if text == "type"{
            current_token = Token::Type(false);
            type_token = Token::Type(false);
        } else if text == "code" {
            current_token = Token::Code(false);
            code_token = Token::Code(false);
        } else if text == "name" {
            current_token = Token::Name;
        } else if text == "file" {
            current_token = Token::File(false);
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
                Token::File(false) => {
                    current_token = Token::File(true);
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
                Token::File(true) => {
                    file_text = Some(text);
                }
                _ => {}
            }
        }
    }
    
    // Check type Key
    let type_write_help = "Write: type = <shader type>";
    let type_possible_value_help = "Possible shader types: Compute, Vertex, Fragment, Geometry, Mesh, RayGeneration, AnyHit, ClosestHit, Miss, Include";
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
            } else if type_text == Some("Include".to_string()) {
                return proc_macro::TokenStream::from_str("()").unwrap()
            } else {
                abort_call_site!("Invalid type Value: {}", type_text.unwrap(); help=type_possible_value_help;)
            }
        }
        _ => {unreachable!()}
    };

    
    let (source, file_path) = if file_text.is_some(){
        if code_token_tree.is_some() {
            abort_call_site!("Cannot use file = \"<glsl file path>\" and code = <glsl code> in one marco");
        }

        let file_path = file_text.unwrap();
        let file_path = file_path.strip_prefix('"');
        if file_path.is_none() {
            abort_call_site!("Write file = \"<glsl file path>\"")
        }
        let file_path = file_path.unwrap().strip_suffix('"');
        if file_path.is_none() {
            abort_call_site!("Write file = \"<glsl file path>\"")
        }
        let file_path = file_path.unwrap();

        if !Path::new(file_path).exists() {
            abort_call_site!("The File {} could not be found.", file_path)
        }

        let content = fs::read_to_string(file_path);
        if content.is_err() {
            abort_call_site!("The File {} could not be read.", file_path)
        }
        (content.unwrap(), file_path.to_string())
    } else {
        let code_write_help = "Write: code = {<glsl>}";
        (match code_token {
            Token::None => {abort_call_site!("Key missing: code"; help=code_write_help)}
            Token::Code(false) => {abort_call_site!("Invalid Key: code"; help=code_write_help)}
            Token::Code(true) => {
                if code_text.is_none() {
                    abort_call_site!("Missing Value for: code ="; help=code_write_help)
                }
                code_text.unwrap()
            }
            _ => {unreachable!()}
        }, MARCO_FILE_PATH.to_string())
    };

    
    println!("Shader input {source}");
    let compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_include_callback(handle_include);

    let binary_result = compiler.compile_into_spirv(
        &source,
        glsl_type,
        &file_path,
        "main", Some(&options));
    
    if binary_result.is_err() {
        let err = binary_result.err().unwrap().to_string();
        let err_lines: Vec<_> = err.split(&format!("{file_path}:")).collect();
        
        if file_path != MARCO_FILE_PATH || err_lines.len() == 1 {
            emit_call_site_error!("{}", err);
        } else {
            let code_token_tree = code_token_tree.unwrap();
            for err_line in err_lines.iter().skip(1) {
                let parts: Vec<_> = err_line.split(":").collect();

                // println!("Err Message Parts {parts:?}");
                let line = parts[0].parse::<usize>();
                if line.is_err() {
                    emit_call_site_error!("Error: {}", err_line);
                    continue;
                }
                let line = line.unwrap();

                let key = parts[2].strip_prefix(" '").unwrap().strip_suffix("' ");
                if key.is_none() {
                    emit_call_site_error!("Error: {}", err_line);
                    continue;
                }
                let key = key.unwrap();

                let (span, _, _) = find_best_line(&source, code_token_tree.clone(), key,0, line - 1);
                if span.is_some() {
                    emit_error!(span.unwrap(), "{}", parts[3])
                } else {
                    emit_call_site_error!("{}", parts[3])
                }
            }
        }
        
        proc_macro::TokenStream::from_str(&format!("panic!(\"{err}\")")).unwrap()
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

fn handle_include(path: &str, _: IncludeType, file_path: &str, _: usize) -> IncludeCallbackResult {
    
    let parts: Vec<&str> = path.split('-').collect();
    if parts.is_empty() || parts.len() == 1 {
        return handle_glsl_include(path, file_path)
    }

    if parts.len() > 2 {
        return Err(format!("Include Error The Path {path} has more than one \"-\""))
    }
    
    handle_rust_include(parts[0], parts[1])
}

fn handle_glsl_include(file_path: &str, origen_path: &str) -> IncludeCallbackResult {
    let path = if origen_path != MARCO_FILE_PATH {
        let path = Path::new(origen_path);
        if let Some(parent_path) = path.parent() {
            format!("{}/{}", parent_path.to_str().unwrap(), file_path)
        } else {
            file_path.to_string()
        }
    } else {
        file_path.to_string()
    };
    
    if !Path::new(&path).exists() {
        return Err(format!("Include Error The File {path} could not be found."))
    }

    let content = fs::read_to_string(&path);
    if content.is_err() {
        return Err(format!("Include Error: The File {path} could not be read."))
    }
    let content = content.unwrap();
    
    Ok(ResolvedInclude {
        resolved_name: path.to_string(),
        content: content.to_string(),
    })
}

fn handle_rust_include(file_path: &str, glsl_macro_name: &str) -> IncludeCallbackResult {
    if !Path::new(file_path).exists() {
        return Err(format!("Include Error The File {file_path} could not be found."))
    }

    let content = fs::read_to_string(file_path);
    if content.is_err() {
        return Err(format!("Include Error: The File {file_path} could not be read."))
    }
    let content = content.unwrap();

    let found_indices: Vec<usize> = content.match_indices(&format!("name = \"{glsl_macro_name}\"")).map(|(i, _)|i).collect();
    if found_indices.is_empty() {
        return Err(format!("Include Error No glsl! marco with the name = \"{glsl_macro_name}\" in {file_path}."))
    }

    if found_indices.len() > 1 {
        return Err(format!("Include Error More than one occurrence of name = \"{glsl_macro_name}\" in {file_path}."))
    }
    let name_index = found_indices[0];
    let code_start_index = content[name_index..].find("code = {");
    if code_start_index.is_none() {
        return Err(format!("Include Error No opening Brace found! name = \"{glsl_macro_name}\" must be followed by a code = {{<glsl>}}."))
    }
    let code_start_index = code_start_index.unwrap() + name_index + 8;

    let mut counter = 1;
    let mut code_end_index = None;
    for (offset, val) in content[code_start_index..]
        .match_indices(['{', '}'])
        .into_iter() {

        if val == "{" {
            counter += 1;
        } else if val == "}" {
            counter -= 1;
        }

        if counter <= 0 {
            code_end_index = Some(offset -2);
            break
        }
    }

    if code_end_index.is_none() {
        return Err(format!("Include Error No closing Brace found! name = \"{glsl_macro_name}\" must be followed by a code = {{<glsl>}}. \
         Start index {code_start_index}. \
         Searched in {content:?}"))
    }
    let code_end_index = code_end_index.unwrap() + code_start_index;
    let glsl_content = &content[code_start_index..code_end_index];

    //return Err(format!("glsl_content {glsl_content}"));

    Ok(ResolvedInclude {
        resolved_name: format!("{file_path}_glsl_macro_{glsl_macro_name}"),
        content: glsl_content.to_string(),
    })
}


