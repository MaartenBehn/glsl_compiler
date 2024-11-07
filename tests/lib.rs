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
