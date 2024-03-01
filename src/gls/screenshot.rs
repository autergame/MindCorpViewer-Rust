use std::{os::raw::c_void, path::Path, ptr};

use gl::types::{GLint, GLsizei, GLuint};

pub struct Screenshot {
    pub fbo: Vec<GLuint>,
    pub rbo: Vec<GLuint>,
    pub texture_id: GLuint,

    pub format: usize,
    pub file_name: String,
    pub resolution: [GLsizei; 2],
    pub use_samples: bool,
}

impl Screenshot {
    pub fn new(use_samples: bool, resolution: [GLsizei; 2]) -> Screenshot {
        Screenshot {
            fbo: vec![0; 2],
            rbo: vec![0; 3],
            texture_id: 0,

            format: 0,
            file_name: String::from("screenshot"),
            resolution,
            use_samples,
        }
    }

    pub fn update(&mut self) {
        unsafe {
            gl::GenFramebuffers(2, self.fbo.as_mut_ptr());
            gl::GenRenderbuffers(3, self.rbo.as_mut_ptr());
            gl::GenTextures(1, &mut self.texture_id);

            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                self.resolution[0],
                self.resolution[1],
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                ptr::null(),
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);

            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo[0]);

            gl::BindRenderbuffer(gl::RENDERBUFFER, self.rbo[0]);
            gl::RenderbufferStorage(
                gl::RENDERBUFFER,
                gl::DEPTH_COMPONENT24,
                self.resolution[0],
                self.resolution[1],
            );
            gl::FramebufferRenderbuffer(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::RENDERBUFFER,
                self.rbo[0],
            );
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                self.texture_id,
                0,
            );

            if self.use_samples {
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo[1]);

                gl::BindRenderbuffer(gl::RENDERBUFFER, self.rbo[1]);
                gl::RenderbufferStorageMultisample(
                    gl::RENDERBUFFER,
                    4,
                    gl::DEPTH_COMPONENT24,
                    self.resolution[0],
                    self.resolution[1],
                );
                gl::FramebufferRenderbuffer(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_ATTACHMENT,
                    gl::RENDERBUFFER,
                    self.rbo[1],
                );

                gl::BindRenderbuffer(gl::RENDERBUFFER, self.rbo[2]);
                gl::RenderbufferStorageMultisample(
                    gl::RENDERBUFFER,
                    4,
                    gl::RGBA8,
                    self.resolution[0],
                    self.resolution[1],
                );
                gl::FramebufferRenderbuffer(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::RENDERBUFFER,
                    self.rbo[2],
                );
            }

            gl::BindRenderbuffer(gl::RENDERBUFFER, 0);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    pub fn take(&self, fov: f32) -> glam::Mat4 {
        unsafe {
            if self.use_samples {
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo[1]);
            } else {
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo[0]);
            }

            gl::Viewport(0, 0, self.resolution[0], self.resolution[1]);
            gl::ClearColor(0.0f32, 0.0f32, 0.0f32, 0.0f32);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        glam::Mat4::perspective_infinite_rh(
            fov,
            self.resolution[0] as f32 / self.resolution[1] as f32,
            0.1f32,
        ) * glam::Mat4::from_scale(glam::vec3(1.0f32, -1.0f32, 1.0f32))
    }

    pub fn save(&mut self, window_size: [GLint; 2]) {
        unsafe {
            if self.use_samples {
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.fbo[1]);
                gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.fbo[0]);
                gl::BlitFramebuffer(
                    0,
                    0,
                    self.resolution[0],
                    self.resolution[1],
                    0,
                    0,
                    self.resolution[0],
                    self.resolution[1],
                    gl::COLOR_BUFFER_BIT,
                    gl::NEAREST,
                );
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo[0]);
            }

            let mut buffer = vec![0u8; (self.resolution[0] * self.resolution[1] * 4) as usize];
            gl::ReadPixels(
                0,
                0,
                self.resolution[0],
                self.resolution[1],
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                buffer.as_mut_ptr() as *mut c_void,
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, window_size[0], window_size[1]);
            gl::ClearColor(0.5f32, 0.5f32, 0.5f32, 1.0f32);

            gl::DeleteFramebuffers(2, self.fbo.as_ptr());
            gl::DeleteRenderbuffers(3, self.rbo.as_ptr());
            gl::DeleteTextures(1, &self.texture_id);

            if self.file_name.is_empty() {
                self.file_name = String::from("screenshot");
            }

            let file_name = format!(
                "{}.{}",
                self.file_name,
                FORMATS[self.format].extensions_str()[0]
            );

            image::save_buffer_with_format(
                Path::new(&file_name),
                &buffer,
                self.resolution[0] as u32,
                self.resolution[1] as u32,
                image::ColorType::Rgba8,
                FORMATS[self.format],
            )
            .expect("Could not save screenshot image");
        }
    }
}

const FORMATS: [image::ImageFormat; 4] = [
    image::ImageFormat::Png,
    image::ImageFormat::Jpeg,
    image::ImageFormat::Bmp,
    image::ImageFormat::Tiff,
];
