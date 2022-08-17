extern crate imgui_opengl_renderer;

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

struct GlfwClipboardBackend(*mut c_void);

impl imgui::ClipboardBackend for GlfwClipboardBackend {
    fn get(&mut self) -> Option<String> {
        unsafe {
            let char_ptr = glfw::ffi::glfwGetClipboardString(self.0 as *mut glfw::ffi::GLFWwindow);
            Some(CStr::from_ptr(char_ptr).to_str().unwrap().to_string())
        }
    }

    fn set(&mut self, value: &str) {
        unsafe {
            glfw::ffi::glfwSetClipboardString(
                self.0 as *mut glfw::ffi::GLFWwindow,
                value.as_ptr() as *const c_char,
            );
        };
    }
}

pub struct ImguiGLFW {
    mouse_press: [bool; 5],
    renderer: imgui_opengl_renderer::Renderer,
}

impl ImguiGLFW {
    pub fn new(imgui: &mut imgui::Context, window: &mut glfw::Window) -> ImguiGLFW {
        let window_ptr = unsafe { glfw::ffi::glfwGetCurrentContext() as *mut c_void };
        imgui.set_clipboard_backend(GlfwClipboardBackend(window_ptr));

        let mut io_mut = imgui.io_mut();
        io_mut.key_map[imgui::Key::Tab as usize] = glfw::Key::Tab as u32;
        io_mut.key_map[imgui::Key::LeftArrow as usize] = glfw::Key::Left as u32;
        io_mut.key_map[imgui::Key::RightArrow as usize] = glfw::Key::Right as u32;
        io_mut.key_map[imgui::Key::UpArrow as usize] = glfw::Key::Up as u32;
        io_mut.key_map[imgui::Key::DownArrow as usize] = glfw::Key::Down as u32;
        io_mut.key_map[imgui::Key::PageUp as usize] = glfw::Key::PageUp as u32;
        io_mut.key_map[imgui::Key::PageDown as usize] = glfw::Key::PageDown as u32;
        io_mut.key_map[imgui::Key::Home as usize] = glfw::Key::Home as u32;
        io_mut.key_map[imgui::Key::End as usize] = glfw::Key::End as u32;
        io_mut.key_map[imgui::Key::Insert as usize] = glfw::Key::Insert as u32;
        io_mut.key_map[imgui::Key::Delete as usize] = glfw::Key::Delete as u32;
        io_mut.key_map[imgui::Key::Backspace as usize] = glfw::Key::Backspace as u32;
        io_mut.key_map[imgui::Key::Space as usize] = glfw::Key::Space as u32;
        io_mut.key_map[imgui::Key::Enter as usize] = glfw::Key::Enter as u32;
        io_mut.key_map[imgui::Key::Escape as usize] = glfw::Key::Escape as u32;
        io_mut.key_map[imgui::Key::A as usize] = glfw::Key::A as u32;
        io_mut.key_map[imgui::Key::C as usize] = glfw::Key::C as u32;
        io_mut.key_map[imgui::Key::V as usize] = glfw::Key::V as u32;
        io_mut.key_map[imgui::Key::X as usize] = glfw::Key::X as u32;
        io_mut.key_map[imgui::Key::Y as usize] = glfw::Key::Y as u32;
        io_mut.key_map[imgui::Key::Z as usize] = glfw::Key::Z as u32;

        let renderer =
            imgui_opengl_renderer::Renderer::new(imgui, |s| window.get_proc_address(s) as _);

        ImguiGLFW {
            mouse_press: [false; 5],
            renderer,
        }
    }

    pub fn handle_event(&mut self, imgui: &mut imgui::Context, event: &glfw::WindowEvent) {
        match *event {
            glfw::WindowEvent::MouseButton(button, action, _) => {
                let index = match button {
                    glfw::MouseButton::Button1 => 0,
                    glfw::MouseButton::Button2 => 1,
                    glfw::MouseButton::Button3 => 2,
                    glfw::MouseButton::Button4 => 3,
                    glfw::MouseButton::Button5 => 4,
                    _ => 0,
                };
                self.mouse_press[index] = action != glfw::Action::Release;
                imgui.io_mut().mouse_down = self.mouse_press;
            }
            glfw::WindowEvent::CursorPos(xpos, ypos) => {
                imgui.io_mut().mouse_pos = [xpos as f32, ypos as f32];
            }
            glfw::WindowEvent::Scroll(_, yoffset) => {
                imgui.io_mut().mouse_wheel = yoffset as f32;
            }
            glfw::WindowEvent::Char(character) => {
                imgui.io_mut().add_input_character(character);
            }
            glfw::WindowEvent::Key(key, _, action, modifier) => {
                imgui.io_mut().key_alt = modifier.intersects(glfw::Modifiers::Alt);
                imgui.io_mut().key_ctrl = modifier.intersects(glfw::Modifiers::Control);
                imgui.io_mut().key_shift = modifier.intersects(glfw::Modifiers::Shift);
                imgui.io_mut().key_super = modifier.intersects(glfw::Modifiers::Super);

                imgui.io_mut().keys_down[key as usize] = action != glfw::Action::Release;
            }
            _ => {}
        }
    }

    pub fn frame<'a>(
        &mut self,
        delta_time: f32,
        window: &glfw::Window,
        imgui: &'a mut imgui::Context,
    ) -> imgui::Ui<'a> {
        let io = imgui.io_mut();

        io.delta_time = delta_time;

        let (width, height) = window.get_size();
        let (display_width, display_height) = window.get_framebuffer_size();

        let (width, height) = (width as f32, height as f32);
        let (display_width, display_height) = (display_width as f32, display_height as f32);

        io.display_size = [width, height];
        if width > 0.0f32 && height > 0.0f32 && display_width > 0.0f32 && display_height > 0.0f32 {
            io.display_framebuffer_scale = [display_width / width, display_height / height];
        } else {
            io.display_framebuffer_scale = [1.0f32, 1.0f32];
        }

        imgui.frame()
    }

    pub fn draw<'ui>(&mut self, ui: imgui::Ui<'ui>, window: &mut glfw::Window) {
        let io = ui.io();
        if !io
            .config_flags
            .contains(imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE)
        {
            match ui.mouse_cursor() {
                Some(mouse_cursor) if !io.mouse_draw_cursor => {
                    window.set_cursor_mode(glfw::CursorMode::Normal);
                    let cursor = match mouse_cursor {
                        imgui::MouseCursor::TextInput => glfw::StandardCursor::IBeam,
                        imgui::MouseCursor::ResizeNS => glfw::StandardCursor::VResize,
                        imgui::MouseCursor::ResizeEW => glfw::StandardCursor::HResize,
                        imgui::MouseCursor::Hand => glfw::StandardCursor::Hand,
                        _ => glfw::StandardCursor::Arrow,
                    };
                    window.set_cursor(Some(glfw::Cursor::standard(cursor)));
                }
                _ => {
                    window.set_cursor_mode(glfw::CursorMode::Hidden);
                }
            }
        }
        unsafe {
            gl::Disable(gl::MULTISAMPLE);
            self.renderer.render(ui);
            gl::Enable(gl::MULTISAMPLE);
        }
    }
}
