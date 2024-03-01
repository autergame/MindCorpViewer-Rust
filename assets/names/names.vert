#version 330
//author https://github.com/autergame

layout (location = 0) in vec2 Positions; 
layout (location = 1) in int TextureId; 

out vec2 TextureCoord;

uniform mat4 MVP;

uniform vec2 TextSize;
uniform vec2 TextOffset;
uniform vec2 TextOffsetSize;
uniform vec3 TextPosition;
uniform float TextScale;

uniform vec3 CameraUp;
uniform vec3 CameraRight;

void main()
{
	switch (TextureId) {
		case 0: {
			TextureCoord = TextOffset;
			break;
		}
		case 1: {
			TextureCoord = TextOffset + vec2(TextOffsetSize.x, 0.0);
			break;
		}
		case 2: {
			TextureCoord = TextOffset + vec2(0.0, TextOffsetSize.y);
			break;
		}
		case 3: {
			TextureCoord = TextOffset + TextOffsetSize;
			break;
		}
	}

	vec3 positions = 
		TextPosition
		+ CameraRight * Positions.x * TextSize.x * TextScale
		+ CameraUp * Positions.y * TextSize.y * TextScale;

	gl_Position = MVP * vec4(positions, 1.0);
}