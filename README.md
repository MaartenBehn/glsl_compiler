# GLSL Compiler Marco

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
