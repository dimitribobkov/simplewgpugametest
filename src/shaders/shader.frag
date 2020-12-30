#version 450
#extension GL_ARB_separate_shader_objects : enable
layout(location=0) in vec2 v_tex_coords;
layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;
layout(set=2, binding=0)
uniform Material{
    float shininess;
    float metallic;
};

void main() {
    float gamma = 2.2;
    vec4 texture = texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords);
    f_color = vec4(pow(vec3(texture), vec3(gamma)), texture.a) - metallic;
}