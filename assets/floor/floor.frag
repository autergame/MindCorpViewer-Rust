#version 330
//author https://github.com/autergame

in vec2 UV;

out vec4 FragColor;

uniform sampler2D Diffuse;

void main()
{
	FragColor = texture(Diffuse, UV);
}