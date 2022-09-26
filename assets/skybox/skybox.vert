#version 330
//author https://github.com/autergame

layout (location = 0) in vec3 Positions;

out vec3 UV;

uniform mat4 MVP;

void main()
{
	vec4 pos = MVP * vec4(Positions, 1.0);
	gl_Position = pos.xyww;
	UV = Positions;
}