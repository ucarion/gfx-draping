#[macro_use]
extern crate gfx;

extern crate camera_controllers;
extern crate gfx_draping;
extern crate piston_window;
extern crate vecmath;

use camera_controllers::{CameraPerspective, OrbitZoomCamera, OrbitZoomCameraSettings};
use gfx::Factory;
use gfx::traits::FactoryExt;
use gfx_draping::{DrapingRenderer, Polygon, PolygonBuffer};
use piston_window::{OpenGL, PistonWindow, RenderEvent, ResizeEvent, Window, WindowSettings};

gfx_vertex_struct!(Vertex {
    position: [f32; 3] = "a_position",
    tex_coords: [f32; 2] = "a_tex_coords",
});

gfx_pipeline!(terrain_pipeline {
    color_texture: gfx::TextureSampler<[f32; 4]> = "t_color",
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
    out_depth: gfx::DepthTarget<::gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
});

fn get_projection(window: &PistonWindow) -> [[f32; 4]; 4] {
    let draw_size = window.window.draw_size();

    CameraPerspective {
        fov: 45.0,
        near_clip: 10.0,
        far_clip: 1000.0,
        aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32),
    }.projection()
}

fn get_elevation(x: f32, y: f32) -> f32 {
    ((x / 10.0).sin() + (y / 5.0).sin()) * 10.0
}

const TERRAIN_SIDE_LENGTH: u16 = 100;

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Shadow Volume Draping Demo", [800, 600])
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
                position: [x, -y, get_elevation(x, y)],
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
            include_bytes!("terrain.vert"),
            include_bytes!("terrain.frag"),
        )
        .unwrap();

    let terrain_pso = factory
        .create_pipeline_state(
            &terrain_shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill().with_cull_back(),
            terrain_pipeline::new(),
        )
        .unwrap();

    let terrain_data = terrain_pipeline::Data {
        color_texture: (terrain_texture_view.clone(), terrain_sampler.clone()),
        mvp: [[0.0; 4]; 4],
        out_color: window.output_color.clone(),
        out_depth: window.output_stencil.clone(),
        vertex_buffer: terrain_vertex_buffer,
    };

    let mut terrain_bundle = gfx::Bundle {
        slice: terrain_slice,
        pso: terrain_pso,
        data: terrain_data,
    };

    let polygon1_points = vec![
        // Exterior ring
        (40.0, -60.0),
        (60.0, -60.0),
        (60.0, -40.0),
        (40.0, -40.0),
        (40.0, -60.0),

        // Interior ring #1
        (49.0, -55.0),
        (49.0, -45.0),
        (59.0, -45.0),
        (59.0, -55.0),
        (49.0, -55.0),

        // Interior ring #2
        (41.0, -42.0),
        (41.0, -41.0),
        (42.0, -41.0),
        (42.0, -42.0),
        (41.0, -42.0),
    ];
    let polygon1_bounds = [(40.0, 60.0), (-60.0, -40.0), (-20.0, 20.0)];
    let polygon1 = Polygon {
        bounds: polygon1_bounds,
        points: polygon1_points,
    };

    let polygon2_points = vec![
        (10.0, -20.0),
        (20.0, -50.0),
        (30.0, -20.0),
        (10.0, -20.0),
    ];
    let polygon2_bounds = [(10.0, 30.0), (-50.0, -20.0), (-20.0, 20.0)];
    let polygon2 = Polygon {
        bounds: polygon2_bounds,
        points: polygon2_points,
    };

    let renderer = DrapingRenderer::new(&mut factory);
    let mut buffer = PolygonBuffer::new();
    let indices1 = buffer.add(&polygon1);
    let indices2 = buffer.add(&polygon2);

    let renderable_buffer = buffer.as_renderable(&mut factory);
    let renderable_indices1 = indices1.as_renderable(&mut factory);
    let renderable_indices2 = indices2.as_renderable(&mut factory);

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
            window.encoder.clear_stencil(&window.output_stencil, 0);
            let mvp = camera_controllers::model_view_projection(
                vecmath::mat4_id(),
                camera_controller.camera(render_args.ext_dt).orthogonal(),
                get_projection(window),
            );

            terrain_bundle.data.mvp = mvp;
            terrain_bundle.encode(&mut window.encoder);

            renderer.render(
                &mut factory,
                &mut window.encoder,
                window.output_color.clone(),
                window.output_stencil.clone(),
                mvp,
                [0.0, 0.0, 1.0, 0.5],
                &renderable_buffer,
                &renderable_indices1,
            );

            renderer.render(
                &mut factory,
                &mut window.encoder,
                window.output_color.clone(),
                window.output_stencil.clone(),
                mvp,
                [0.0, 0.0, 0.0, 0.8],
                &renderable_buffer,
                &renderable_indices2,
            );
        });

        event.resize(|_, _| {
            terrain_bundle.data.out_color = window.output_color.clone();
            terrain_bundle.data.out_depth = window.output_stencil.clone();
        });
    }
}
