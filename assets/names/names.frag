#version 330
//author https://github.com/autergame

in vec2 TextureCoord;

out vec4 FragColor;

uniform sampler2D TextTexture;

void main()
{
	float alpha = texture(TextTexture, TextureCoord).r;
	FragColor = vec4(1.0, 1.0, 1.0, alpha);
}