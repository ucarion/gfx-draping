#version 150 core

uniform mat4 u_mvp;
uniform sampler2D t_color;

in vec3 a_position;
in vec2 a_tex_coords;

out vec4 v_color;

void main() {
    v_color = texture(t_color, a_tex_coords);
    gl_Position = u_mvp * vec4(a_position, 1.0);
}
