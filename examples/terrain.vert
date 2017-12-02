#version 150 core

uniform mat4 u_mvp;
uniform float u_time;
uniform sampler2D t_color;

in vec2 a_position;
in vec2 a_tex_coords;

out vec4 v_color;

float z() {
    float x = a_position.x;
    float y = a_position.y;

    return 10.0 * sin(u_time + x / 10.0 + y / 20.0);
}

void main() {
    v_color = texture(t_color, a_tex_coords);
    gl_Position = u_mvp * vec4(a_position, z(), 1.0);
}
