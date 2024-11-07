#[macro_use]
extern crate glsl_compiler;

#[test]
fn void_main_empty() {
    let a = glsl_comp! {type = comp, code = {
        #version 450 core
        
        vec4 rgb(uint r, uint g, uint b) {
            return vec4(float(r) / 255.0, float(g) / 255.0, float(b) / 255.0, 1.0);
        }
    
        #define BORDER_SIZE 0.02
        #define SELECTOR_COLOR rgb(255, 255, 255)
    
        layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;
    
        layout(binding = 0, rgba8) uniform writeonly image2D img;
    
        layout(binding = 1) uniform RenderData {
            uint chunk_size;
            uint selector_pos_x;
            uint selector_pos_y;
        } render_data;
    
        #define CHUNK_SIZE render_data.chunk_size
        #define SELECTOR_POS ivec2(render_data.selector_pos_x, render_data.selector_pos_y)
    
        #define PIXELS_PER_NODE 30
    
        layout(binding = 2) buffer ChunkBuffer {
            uint[] data;
        } chunk_buffer;
    
        #define POS_IN_BOUNDS(pos) pos.x < CHUNK_SIZE && pos.y < CHUNK_SIZE
        #define GET_NODE_AT(pos) chunk_buffer.data[pos.x * CHUNK_SIZE + pos.y]
    
        vec4 node_color(uint data) {
            return vec4(0.0);
        }
    
        bool at_boarder(vec2 v, float border_size) {
            return v.x < border_size || (1 - v.x) < border_size * 2 || v.y < border_size || (1 - v.y) < border_size * 2;
        }
    
    
        vec4 debug_overlay(vec4 color, uvec2 node_pos, vec2 in_node_pos) {
            if (ivec2(node_pos) == SELECTOR_POS && at_boarder(in_node_pos, BORDER_SIZE)) {
                return SELECTOR_COLOR;
            }
    
            return color;
        }
    
        void main () {
            uvec2 pos = gl_GlobalInvocationID.xy;
            uvec2 node_pos = pos / PIXELS_PER_NODE;
            vec2 in_node_pos = mod(vec2(pos), PIXELS_PER_NODE) / PIXELS_PER_NODE;
    
            vec4 color = vec4(0.0);
    
            if (POS_IN_BOUNDS(node_pos)) {
                uint data = GET_NODE_AT(node_pos);
    
                color = node_color(data);
                color = debug_overlay(color, node_pos, in_node_pos);
            }
    
            imageStore(img, ivec2(gl_GlobalInvocationID.xy), color);
        }
    }
    };
    
    println!("{:?}", a)
}
