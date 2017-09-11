#version 150 core

in vec2 a_coords;

out vec2 v_tex_coords;

void main() {
    v_tex_coords = (vec2(1.0, 1.0) + a_coords) / 2.0;
    gl_Position = vec4(a_coords, 0.0, 1.0);
}
