#[macro_use]
extern crate gfx;

extern crate camera_controllers;
extern crate piston_window;
extern crate sdl2_window;
extern crate vecmath;

use camera_controllers::{CameraPerspective, OrbitZoomCamera, OrbitZoomCameraSettings};
use gfx::Factory;
use gfx::traits::FactoryExt;
use piston_window::{OpenGL, PistonWindow, RenderEvent, ResizeEvent, Window, WindowSettings};
use sdl2_window::Sdl2Window;

gfx_vertex_struct!(Vertex {
    position: [f32; 3] = "a_position",
    tex_coords: [f32; 2] = "a_tex_coords",
});

gfx_pipeline!(terrain_pipeline {
    out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
    out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    color_texture: gfx::TextureSampler<[f32; 4]> = "t_color",
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
});

fn get_projection(window: &PistonWindow<Sdl2Window>) -> [[f32; 4]; 4] {
    let draw_size = window.window.draw_size();

    CameraPerspective {
        fov: 45.0,
        near_clip: 0.1,
        far_clip: 10000.0,
        aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32),
    }.projection()
}

fn get_elevation(x: f32, y: f32) -> f32 {
    ((x / 10.0).sin() + (y / 5.0).sin()) * 10.0
}

const TERRAIN_SIDE_LENGTH: u16 = 100;

fn main() {
    let mut window: PistonWindow<Sdl2Window> =
        WindowSettings::new("Shadow Volume Draping Demo", [800, 600])
            .exit_on_esc(true)
            .opengl(OpenGL::V3_2)
            .build()
            .unwrap();

    let mut factory = window.factory.clone();

    let mut terrain_vertices = Vec::new();
    let mut terrain_indices = Vec::new();
    let mut terrain_texture_data = Vec::new();

    for y in 0..TERRAIN_SIDE_LENGTH {
        for x in 0..TERRAIN_SIDE_LENGTH {
            let max_value = TERRAIN_SIDE_LENGTH - 1;
            if y != max_value && x != max_value {
                let a = (x + 0) + (y + 0) * TERRAIN_SIDE_LENGTH;
                let b = (x + 1) + (y + 0) * TERRAIN_SIDE_LENGTH;
                let c = (x + 0) + (y + 1) * TERRAIN_SIDE_LENGTH;
                let d = (x + 1) + (y + 1) * TERRAIN_SIDE_LENGTH;

                terrain_indices.extend_from_slice(&[a, c, b, b, c, d]);
            }

            let (x, y) = (x as f32, y as f32);
            let (u, v) = (x / max_value as f32, y / max_value as f32);
            terrain_vertices.push(Vertex {
                position: [x, y, get_elevation(x, y)],
                tex_coords: [u, v],
            });

            terrain_texture_data.push([(255.0 * u) as u8, (255.0 * v) as u8, 0, 255]);
        }
    }

    let (terrain_vertex_buffer, terrain_slice) =
        factory.create_vertex_buffer_with_slice(&terrain_vertices, terrain_indices.as_slice());
    let (_, terrain_texture_view) = factory
        .create_texture_immutable::<gfx::format::Srgba8>(
            gfx::texture::Kind::D2(
                TERRAIN_SIDE_LENGTH,
                TERRAIN_SIDE_LENGTH,
                gfx::texture::AaMode::Single,
            ),
            &[terrain_texture_data.as_slice()],
        )
        .unwrap();

    let terrain_sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(
        gfx::texture::FilterMethod::Bilinear,
        gfx::texture::WrapMode::Clamp,
    ));

    let terrain_shader_set = factory
        .create_shader_set(
            include_bytes!("shaders/terrain.vert"),
            include_bytes!("shaders/terrain.frag"),
        )
        .unwrap();

    let terrain_pso = factory
        .create_pipeline_state(
            &terrain_shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            terrain_pipeline::new(),
        )
        .unwrap();

    let mut terrain_data = terrain_pipeline::Data {
        color_texture: (terrain_texture_view, terrain_sampler),
        mvp: [[0.0; 4]; 4],
        out_color: window.output_color.clone(),
        out_depth: window.output_stencil.clone(),
        vertex_buffer: terrain_vertex_buffer,
    };

    let mut camera_controller =
        OrbitZoomCamera::new([0.0, 0.0, 0.0], OrbitZoomCameraSettings::default());

    while let Some(event) = window.next() {
        camera_controller.event(&event);

        window.draw_3d(&event, |window| {
            let render_args = event.render_args().unwrap();

            window.encoder.clear(
                &window.output_color,
                [0.3, 0.3, 0.3, 1.0],
            );
            window.encoder.clear_depth(&window.output_stencil, 1.0);

            terrain_data.mvp = camera_controllers::model_view_projection(
                vecmath::mat4_id(),
                camera_controller.camera(render_args.ext_dt).orthogonal(),
                get_projection(window),
            );

            window.encoder.draw(
                &terrain_slice,
                &terrain_pso,
                &terrain_data,
            );
        });

        event.resize(|_, _| {
            terrain_data.out_color = window.output_color.clone();
            terrain_data.out_depth = window.output_stencil.clone();
        });
    }
}
