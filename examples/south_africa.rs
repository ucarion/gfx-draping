#[macro_use]
extern crate gfx;

extern crate camera_controllers;
extern crate cgmath;
extern crate fps_counter;
extern crate geo;
extern crate geojson;
extern crate gfx_draping;
extern crate gfx_text;
extern crate piston_window;
extern crate vecmath;

use camera_controllers::{CameraPerspective, OrbitZoomCamera, OrbitZoomCameraSettings};
use cgmath::Matrix4;
use fps_counter::FPSCounter;
use geo::boundingbox::BoundingBox;
use geo::map_coords::MapCoords;
use geo::simplify::Simplify;
use geo::MultiPolygon;
use geojson::GeoJson;
use geojson::conversion::TryInto;
use gfx::Factory;
use gfx::traits::FactoryExt;
use gfx_draping::{DrapingRenderer, PolygonBuffer, PolygonBufferIndices};
use piston_window::{OpenGL, PistonWindow, RenderEvent, ResizeEvent, Window, WindowSettings};

gfx_vertex_struct!(Vertex {
    position: [f32; 3] = "a_position",
    tex_coords: [f32; 2] = "a_tex_coords",
});

gfx_pipeline!(terrain_pipeline {
    out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
    color_texture: gfx::TextureSampler<[f32; 4]> = "t_color",
    time: gfx::Global<f32> = "u_time",
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
    out_depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::preset::depth::LESS_EQUAL_WRITE,
        gfx::state::Stencil::new(
            gfx::state::Comparison::Always,
            255,
            (
                gfx::state::StencilOp::Keep, // never happens if Comparison::Always
                gfx::state::StencilOp::Keep, // when depth test fails
                gfx::state::StencilOp::Keep, // when depth test passes
            ),
        ),
    ),
});

fn get_projection(window: &PistonWindow) -> [[f32; 4]; 4] {
    let draw_size = window.window.draw_size();

    CameraPerspective {
        fov: 45.0,
        near_clip: 0.1,
        far_clip: 1000.0,
        aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32),
    }.projection()
}

fn get_elevation(x: f32, y: f32) -> f32 {
    ((x / 3.0).sin() + (y / 2.0).sin()) * 5.0
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new("South Africa Draping Demo", [800, 600])
        .exit_on_esc(true)
        .opengl(OpenGL::V3_2)
        .build()
        .unwrap();

    let mut factory = window.factory.clone();

    let geojson: GeoJson = include_str!("south_africa.geojson").parse().unwrap();
    let mut feature_collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => panic!("Unexpected geojson object type!"),
    };

    let feature = feature_collection.features.remove(0);
    let geometry = feature.geometry.unwrap();

    let multi_polygon: MultiPolygon<f32> = geometry.value.try_into().unwrap();
    let multi_polygon = multi_polygon.simplify(&0.01);
    let bbox = multi_polygon.bbox().unwrap();

    let multi_polygon = multi_polygon.map_coords(&|point| {
        let x = point.0 - bbox.xmin;
        let y = point.1 - bbox.ymax;

        (x, y)
    });

    let max_x_value = (bbox.xmax - bbox.xmin).ceil() as u16;
    let max_y_value = (bbox.ymax - bbox.ymin).ceil() as u16;

    let mut terrain_vertices = Vec::new();
    let mut terrain_indices = Vec::new();
    let mut terrain_texture_data = Vec::new();
    for y in 0..max_y_value + 1 {
        for x in 0..max_x_value + 1 {
            if y != max_y_value && x != max_x_value {
                let a = (x + 0) + (y + 0) * (max_x_value + 1);
                let b = (x + 1) + (y + 0) * (max_x_value + 1);
                let c = (x + 0) + (y + 1) * (max_x_value + 1);
                let d = (x + 1) + (y + 1) * (max_x_value + 1);

                terrain_indices.extend_from_slice(&[a, c, b, b, c, d]);
            }

            let (x, y) = (x as f32, y as f32);
            let (u, v) = (x / max_x_value as f32, y / max_y_value as f32);
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
                max_x_value + 1,
                max_y_value + 1,
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
        time: 0.0,
        mvp: [[0.0; 4]; 4],
        out_color: window.output_color.clone(),
        out_depth_stencil: (window.output_stencil.clone(), (255, 255)),
        vertex_buffer: terrain_vertex_buffer,
    };

    let mut terrain_bundle = gfx::Bundle {
        slice: terrain_slice,
        pso: terrain_pso,
        data: terrain_data,
    };

    let renderer = DrapingRenderer::new(&mut factory);
    let mut buffer = PolygonBuffer::new();
    let mut indices = PolygonBufferIndices::new();
    for polygon in multi_polygon {
        indices.extend(&buffer.add(&polygon.into()));
    }

    let renderable_buffer = buffer.as_renderable(&mut factory);
    let renderable_indices = indices.as_renderable(&mut factory);

    let mut camera_controller =
        OrbitZoomCamera::new([0.0, 0.0, 0.0], OrbitZoomCameraSettings::default());

    let mut fps_counter = FPSCounter::new();
    let mut text_renderer = gfx_text::new(factory).build().unwrap();

    let max_z = 10.0;
    let min_z = -10.0;
    let polygon_model = Matrix4::from_translation([0.0, 0.0, min_z].into()) *
        Matrix4::from_nonuniform_scale(1.0, 1.0, max_z - min_z);

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

            terrain_bundle.data.time += 0.01;
            terrain_bundle.data.mvp = mvp;
            terrain_bundle.encode(&mut window.encoder);

            let cgmath_mvp: Matrix4<f32> = mvp.into();

            renderer.render(
                &mut window.encoder,
                window.output_color.clone(),
                window.output_stencil.clone(),
                (cgmath_mvp * polygon_model).into(),
                [0.0, 0.0, 1.0, 0.5],
                &renderable_buffer,
                &renderable_indices,
            );

            let fps_message = format!("Frames per second: {}", fps_counter.tick());
            text_renderer.add(&fps_message, [10, 10], [0.0, 0.0, 0.0, 1.0]);
            text_renderer
                .draw(&mut window.encoder, &window.output_color)
                .unwrap();
        });

        event.resize(|_, _| {
            terrain_bundle.data.out_color = window.output_color.clone();
            terrain_bundle.data.out_depth_stencil.0 = window.output_stencil.clone();
        });
    }
}
