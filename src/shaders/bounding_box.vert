#version 150 core

uniform mat4 u_mvp;

in vec3 a_position;

void main() {
    gl_Position = u_mvp * vec4(a_position, 1.0);
}
