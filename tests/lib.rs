#[macro_use]
extern crate glsl_compiler;

#[test]
fn void_main_empty() {
    let bin = glsl!{type = Compute, code = {
        #version 450 core
    
        layout(binding = 0, rgba8) uniform writeonly image2D img;
    
        void main () {
            uvec2 pos = gl_GlobalInvocationID.xy;
            vec4 color = vec4(pos, 0.0, 1.0);
            
            imageStore(img, ivec2(pos), color);
        }
    }};
    
    println!("{:?}", bin)
}

#[test]
fn shader() {
    let bin = glsl!{type = Compute, code = {
        #version 450 core
        
        #include "tests/lib.rs-included.glsl"
    
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

#[test]
fn glsl_file_include() {
    let bin = glsl!{type = Compute, code = {
        #version 450 core
        
        #include "shaders/test_include.glsl"
    
        layout(binding = 0, rgba8) uniform writeonly image2D img;
        void main () {
            uvec2 pos = gl_GlobalInvocationID.xy;
            imageStore(img, ivec2(pos), COLOR);
        }
    }};

    println!("{:?}", bin)
}

#[test]
fn glsl_from_file() {
    let bin: &[u8] = glsl!{type = Compute, file = "shaders/test.glsl"};

    println!("{:?}", bin)
}

#[test]
fn glsl_file_include_in_include() {
    let bin = glsl!{type = Compute, code = {
        #version 450 core
        
        #include "shaders/test_include_include2.glsl"
    
        layout(binding = 0, rgba8) uniform writeonly image2D img;
        void main () {
            uvec2 pos = gl_GlobalInvocationID.xy;
            imageStore(img, ivec2(pos), COLOR);
        }
    }};

    println!("{:?}", bin)
}

#[test]
fn glsl_file_include_in_include2() {
    let bin: &[u8] = glsl!{type = Compute, file = "shaders/test_include_include3.glsl"};

    println!("{:?}", bin)
}

