#version 150 core

uniform sampler2D t_texture;

in vec2 v_tex_coords;

out vec4 o_color;

void main() {
    float depth = texture(t_texture, v_tex_coords).r;
    o_color = vec4(depth, depth, depth, 1.0);
}
