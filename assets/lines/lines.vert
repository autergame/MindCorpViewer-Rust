#version 330
//author https://github.com/autergame

layout (location = 0) in vec4 Positions;
layout (location = 1) in vec3 Colors;

out vec3 Color;

uniform mat4 MVP;

void main()
{
	Color = Colors;
	gl_Position = MVP * Positions;
}