#version 450 core

#include "test_include.glsl"

layout(binding = 0, rgba8) uniform writeonly image2D img;
void main () {
    uvec2 pos = gl_GlobalInvocationID.xy;
    imageStore(img, ivec2(pos), COLOR);
}