#[macro_use]
extern crate cfg_if;
extern crate web_sys;
extern crate js_sys;
extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

cfg_if! {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        use console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        fn set_panic_hook() {}
    }
}

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
}

type GlContext = web_sys::WebGl2RenderingContext;
type GlShader = web_sys::WebGlShader;
type GlProgram = web_sys::WebGlProgram;

fn create_shader(gl: &GlContext, shader_type: u32, source: &str) -> GlShader {
    let shader = gl.create_shader(shader_type).unwrap();
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    console::log_1(&JsValue::from_str(&gl.get_shader_info_log(&shader).unwrap()));
    shader
}

fn create_program(gl: &GlContext, vertex_shader: &GlShader, fragment_shader: &GlShader) -> GlProgram {
    let program = gl.create_program().unwrap();
    gl.attach_shader(&program, vertex_shader);
    gl.attach_shader(&program, fragment_shader);
    gl.link_program(&program);
    program
}

fn resize_canvas(canvas: &web_sys::HtmlCanvasElement) {
    let display_width = canvas.client_width() as u32;
    let display_height = canvas.client_height() as u32;
    let width = canvas.width();
    let height = canvas.height();

    if width != display_width || height != display_height {
        canvas.set_width(display_width);
        canvas.set_height(display_height);
    }
}

fn ortho_matrix(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [f32; 16]
{
    let result: [f32; 16] = [
        2.0 / (right - left), 0.0, 0.0, 0.0,
        0.0, 2.0 / (top - bottom), 0.0, 0.0,
        0.0, 0.0, 2.0 / (near - far), 0.0,
        (left + right) / (left - right),
        (top + bottom) / (bottom - top),
        (far + near) / (near - far),
        1.0
    ];

    result
}

fn window() -> web_sys::Window {
    web_sys::window().unwrap()
}

use js_sys::*;
use web_sys::console;

// Called by our JS entry point to run the example.
#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    set_panic_hook();

    let window = window();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("webgl_canvas").unwrap();
    let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

    resize_canvas(&canvas);

    let gl = canvas.get_context("webgl2").unwrap().unwrap()
        .dyn_into::<GlContext>().unwrap();

    let vertex_shader_code = r#"#version 300 es
in vec4 a_position;
in vec4 a_color;

uniform mat4 world;

out vec4 color;

void main() {
    gl_Position = world * a_position;
    color = a_color;
}"#;
    let fragment_shader_code = r#"#version 300 es
precision mediump float;

in vec4 color;
out vec4 outColor;

void main() {
    outColor = color;
}"#;

    let vertex_shader = create_shader(&gl, GlContext::VERTEX_SHADER, vertex_shader_code);
    let fragment_shader = create_shader(&gl, GlContext::FRAGMENT_SHADER, fragment_shader_code);

    let program = create_program(&gl, &vertex_shader, &fragment_shader);
    gl.use_program(Some(&program));

    let vertex_buffer = gl.create_buffer().unwrap();
    gl.bind_buffer(GlContext::ARRAY_BUFFER, Some(&vertex_buffer));

    let memory = wasm_bindgen::memory().dyn_into::<WebAssembly::Memory>()?.buffer();
    let vertices: [f32; 18] = [
        0.0, 0.5 - 0.125, 1.0, 1.0, 0.0, 1.0,
        -0.75 / 1.732, -0.25 - 0.125, 0.0, 1.0, 1.0, 1.0,
        0.75 / 1.732, -0.25 - 0.125, 1.0, 0.0, 1.0, 1.0,
    ];
    let vert_loc = vertices.as_ptr() as u32 / 4;
    let vertex_data = Float32Array::new(&memory).subarray(vert_loc, vert_loc + vertices.len() as u32);

    gl.buffer_data_with_array_buffer_view(GlContext::ARRAY_BUFFER, &vertex_data, GlContext::STATIC_DRAW);

    let vao = gl.create_vertex_array();
    gl.bind_vertex_array(vao.as_ref());

    let pos_location = gl.get_attrib_location(&program, "a_position") as u32;
    gl.enable_vertex_attrib_array(pos_location);
    gl.vertex_attrib_pointer_with_i32(pos_location, 2, GlContext::FLOAT, false, 24, 0);

    let color_loc = gl.get_attrib_location(&program, "a_color") as u32;
    gl.enable_vertex_attrib_array(color_loc);
    gl.vertex_attrib_pointer_with_i32(color_loc, 4, GlContext::FLOAT, false, 24, 8);

    let world_loc = gl.get_uniform_location(&program, "world");
    gl.uniform_matrix4fv_with_f32_array(world_loc.as_ref(), false, &mut ortho_matrix(-4.0 / 3.0, 4.0 / 3.0, -1.0, 1.0, -1.0, 1.0));

    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(GlContext::COLOR_BUFFER_BIT);

    gl.draw_arrays(GlContext::TRIANGLES, 0, 3);

    Ok(())
}
