#version 330
//author https://github.com/autergame

layout (location = 0) in vec3 Positions;
layout (location = 1) in vec2 UVs;

out vec2 UV;

uniform mat4 MVP;

void main()
{
    UV = UVs;
    gl_Position = MVP * vec4(Positions, 1.0);
}