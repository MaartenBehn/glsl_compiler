use proc_macro_error::abort_call_site;
use crate::find_closing_bracket;



fn get_id(name: &str, profile_scope_names: &mut Vec<String>) -> usize {
    let id = profile_scope_names.iter().position(|t| t == name);
    let id = if id.is_none() {
        profile_scope_names.push(name.to_string());
        profile_scope_names.len() -1
    } else {
        id.unwrap()
    };

    id
}

fn profile_begin_code(name: &str, profile_scope_names: &mut Vec<String>, with_init: bool) -> String {
    let id = get_id(name, profile_scope_names);

    let possible_init = if with_init {
        "PROFILE_INIT();\n"
    } else {""};

    format!("{possible_init}PROFILE_SCOPE_BEING({id});\n")
}

fn profile_end_code(name: &str, profile_scope_names: &mut Vec<String>) -> String {
    let id = get_id(name, profile_scope_names);

    format!("\nPROFILE_SCOPE_END({id});\n")
}

fn profile_inject_code<'a>(num_scopes: usize) -> String {
    format!(r#"
#extension GL_EXT_shader_realtime_clock : require

layout(binding = 10) uniform ProfilerIn {{
    uint active_pixel_x;
    uint active_pixel_y;
}} profiler_in;

layout(binding = 11) buffer ProfilerOut {{
    uint[] data;
}} profiler_out;

void PROFILE_INIT() {{
    if (profiler_in.active_pixel_x != gl_GlobalInvocationID.x || profiler_in.active_pixel_y != gl_GlobalInvocationID.y) {{
        return;
    }}

    for (uint i = 0; i < {num_scopes}; i++) {{
        profiler_out.data[i * 5] = 0;
    }}
}}

void PROFILE_SCOPE_BEING(uint id) {{
    if (profiler_in.active_pixel_x != gl_GlobalInvocationID.x || profiler_in.active_pixel_y != gl_GlobalInvocationID.y) {{
        return;
    }}
    uint index = id * 5;

    uvec2 timing = clockRealtime2x32EXT();
    profiler_out.data[index]++;
    profiler_out.data[index + 1] = timing.x;
    profiler_out.data[index + 2] = timing.y;
}}

void PROFILE_SCOPE_END(uint id) {{
    if (profiler_in.active_pixel_x != gl_GlobalInvocationID.x || profiler_in.active_pixel_y != gl_GlobalInvocationID.y) {{
        return;
    }}
    uint index = id * 5;

    uvec2 timing = clockRealtime2x32EXT();
    profiler_out.data[index + 3] = timing.x;
    profiler_out.data[index + 4] = timing.y;
}}
    "#)
}

pub fn inject_profiler(mut source: String) -> (String, Vec<String>) {
    let mut profile_scope_names = vec![];

    let mut main_scope_placed = false;
    let mut start_offset = 0;

    loop {
        let profile_start = source[start_offset..].find("//PROFILE(\"");
        if profile_start.is_none() {
            break
        }
        let profile_start = profile_start.unwrap() + start_offset;
        start_offset = profile_start;

        let include_line_end = source[profile_start..].find("\");");
        if include_line_end.is_none() {
            abort_call_site!("//PROFILE(\" doesn't end with \");"; note=source[profile_start..]);
        }
        let include_line_end = include_line_end.unwrap() + profile_start + 3;
        
        let code_end_index = find_closing_bracket(&source[profile_start..]);
        if code_end_index.is_none() {
            abort_call_site!("//PROFILE(\"<name>\"); in invalid scope. No closing bracket found!"; note=source[profile_start..]);
        }
        let code_end_index = code_end_index.unwrap() + profile_start + 1;

        let name = &source[(profile_start + 11)..(include_line_end - 3)].to_string();
        let main_scope = name == "main";
        main_scope_placed |= main_scope;

        let return_starts: Vec<usize> = source[profile_start..code_end_index].match_indices("return")
            .map(|(i, _)|i + profile_start)
            .collect();


        // Replace parts
        source.replace_range(code_end_index..code_end_index, &profile_end_code(&name, &mut profile_scope_names));

        for return_start in return_starts.into_iter().rev() {
            source.replace_range(return_start..return_start, &profile_end_code(&name, &mut profile_scope_names));
        }

        source.replace_range(profile_start..include_line_end, &profile_begin_code(&name, &mut profile_scope_names, main_scope));
    }

    if !main_scope_placed {
        abort_call_site!("Please place a //PROFILE(\"main\") at the start of the main function.");
    }
 
    let version_start = source.find("#version");
    if version_start.is_none() {
        abort_call_site!("no #version found!");
    }
    let version_start = version_start.unwrap();

    let version_line_end = source[version_start..].find('\n');
    if version_line_end.is_none() {
        abort_call_site!("#version has no new line after it!");
    }
    let version_line_end = version_line_end.unwrap() + version_start + 1;

    source.replace_range(version_line_end..version_line_end, &profile_inject_code(profile_scope_names.len()));

    (source, profile_scope_names)
}
