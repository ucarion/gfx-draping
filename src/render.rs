use gfx;
use gfx::traits::FactoryExt;

use Vertex;
use polygon::*;

gfx_pipeline!(z_fail_polyhedron_pipeline {
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
    out_color: gfx::BlendTarget<gfx::format::Srgba8> = (
        "o_color",
        gfx::state::ColorMask::all(),
        gfx::preset::blend::ALPHA,
    ),
    out_depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::preset::depth::LESS_EQUAL_TEST,
        gfx::state::Stencil {
            front: gfx::state::StencilSide {
                fun: gfx::state::Comparison::Always,
                mask_read: 255,
                mask_write: 255,
                op_fail: gfx::state::StencilOp::Keep,
                op_depth_fail: gfx::state::StencilOp::DecrementWrap,
                op_pass: gfx::state::StencilOp::Keep,
            },
            back: gfx::state::StencilSide {
                fun: gfx::state::Comparison::Always,
                mask_read: 255,
                mask_write: 255,
                op_fail: gfx::state::StencilOp::Keep,
                op_depth_fail: gfx::state::StencilOp::IncrementWrap,
                op_pass: gfx::state::StencilOp::Keep,
            },
        },
    ),
});

gfx_pipeline!(z_fail_bounding_box_pipeline {
    out_color: gfx::BlendTarget<gfx::format::Srgba8> = (
        "o_color",
        gfx::state::ColorMask::all(),
        gfx::preset::blend::ALPHA,
    ),
    mvp: gfx::Global<[[f32; 4]; 4]> = "u_mvp",
    color: gfx::Global<[f32; 4]> = "u_color",
    vertex_buffer: gfx::VertexBuffer<Vertex> = (),
    out_depth_stencil: gfx::DepthStencilTarget<gfx::format::DepthStencil> = (
        gfx::preset::depth::PASS_TEST,
        gfx::state::Stencil::new(
            // A fragment is only "inside" the polyhedron, and thus supposed to be drawn, if the
            // stencil buffer is nonzero at that point.
            gfx::state::Comparison::NotEqual,
            255,
            (
                // An important property of the stencil is that it is all zeroes after this
                // pipeline runs, so that the next draw doesn't need to clear the stencil first.
                //
                // If the stencil test fails, the value is zero and thus should be kept.
                gfx::state::StencilOp::Keep,
                // This never happens, because the depth test always passes.
                gfx::state::StencilOp::Keep,
                // The stencil test passed, so the value should be reset to zero.
                gfx::state::StencilOp::Replace,
            ),
        ),
    ),
});

#[derive(Debug)]
pub struct DrapingRenderer<R: gfx::Resources> {
    polyhedron_pso: gfx::pso::PipelineState<R, z_fail_polyhedron_pipeline::Meta>,
    bounding_box_pso: gfx::pso::PipelineState<R, z_fail_bounding_box_pipeline::Meta>,
}

impl<R: gfx::Resources> DrapingRenderer<R> {
    /// Set up the pipeline state objects needed for rendering draped polygons.
    pub fn new<F: gfx::Factory<R>>(factory: &mut F) -> DrapingRenderer<R> {
        DrapingRenderer {
            polyhedron_pso: Self::polyhedron_pso(factory),
            bounding_box_pso: Self::bounding_box_pso(factory),
        }
    }

    /// Render a single `DrapeablePolygon` as `color`.
    ///
    /// *Note:* The depth buffer in `depth_stencil_target` should contain the depth values of your
    /// terrain -- in other words, draw your terrain just before you call this function, and make
    /// sure you don't clear the buffer until after rendering all the polygons you wish to draw.
    ///
    /// *Note:* In addition, the stencil buffer should be cleared to zero before calling this
    /// function. The stencil buffer is guaranteed to remain zero after each call, so there is no
    /// need to clear the stencil buffer between calls to this function.
    pub fn render<C: gfx::CommandBuffer<R>, F: gfx::Factory<R>>(
        &self,
        factory: &mut F,
        encoder: &mut gfx::Encoder<R, C>,
        render_target: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        depth_stencil_target: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
        mvp: [[f32; 4]; 4],
        color: [f32; 4],
        buffer: &PolygonBuffer,
        indices: &PolygonBufferIndices,
    ) {
        let polyhedron_slice = indices.polyhedron_slice(factory);
        let polyhedron_vertex_buffer = buffer.polyhedron_vertex_buffer(factory);

        let bounding_box_slice = indices.bounding_box_slice(factory);
        let bounding_box_vertex_buffer = buffer.bounding_box_vertex_buffer(factory);

        let polyhedron_bundle = gfx::Bundle {
            pso: self.polyhedron_pso.clone(),
            slice: polyhedron_slice,
            data: z_fail_polyhedron_pipeline::Data {
                mvp: mvp,
                out_color: render_target.clone(),
                out_depth_stencil: (depth_stencil_target.clone(), (0, 0)),
                vertex_buffer: polyhedron_vertex_buffer,
            },
        };

        let bounding_box_bundle = gfx::Bundle {
            pso: self.bounding_box_pso.clone(),
            slice: bounding_box_slice,
            data: z_fail_bounding_box_pipeline::Data {
                color: color,
                mvp: mvp,
                out_color: render_target.clone(),
                out_depth_stencil: (depth_stencil_target.clone(), (0, 0)),
                vertex_buffer: bounding_box_vertex_buffer,
            },
        };

        polyhedron_bundle.encode(encoder);
        bounding_box_bundle.encode(encoder);
    }

    fn polyhedron_pso<F: gfx::Factory<R>>(
        factory: &mut F,
    ) -> gfx::pso::PipelineState<R, z_fail_polyhedron_pipeline::Meta> {
        let shaders = factory
            .create_shader_set(
                include_bytes!("shaders/polyhedron.vert"),
                include_bytes!("shaders/polyhedron.frag"),
            )
            .unwrap();

        let rasterizer = gfx::state::Rasterizer::new_fill();

        factory
            .create_pipeline_state(
                &shaders,
                gfx::Primitive::TriangleList,
                rasterizer,
                z_fail_polyhedron_pipeline::new(),
            )
            .unwrap()
    }

    fn bounding_box_pso<F: gfx::Factory<R>>(
        factory: &mut F,
    ) -> gfx::pso::PipelineState<R, z_fail_bounding_box_pipeline::Meta> {
        let shaders = factory
            .create_shader_set(
                include_bytes!("shaders/bounding_box.vert"),
                include_bytes!("shaders/bounding_box.frag"),
            )
            .unwrap();

        let rasterizer = gfx::state::Rasterizer {
            cull_face: gfx::state::CullFace::Front,
            ..gfx::state::Rasterizer::new_fill()
        };

        factory
            .create_pipeline_state(
                &shaders,
                gfx::Primitive::TriangleList,
                rasterizer,
                z_fail_bounding_box_pipeline::new(),
            )
            .unwrap()
    }
}

#[derive(Debug)]
pub struct DrapeablePolygon {
    buffer: PolygonBuffer,
    indices: PolygonBufferIndices,
}

impl DrapeablePolygon {
    /// Prepare vertex and index buffers needed for rendering a individual draped polygon.
    ///
    /// `points` should be the concatenation of the rings in a polygon. The first ring should be
    /// the exterior ring, and then the interior rings should follow.
    ///
    /// A "ring" is a sequence of points, where the first and last point are the same. The exterior
    /// ring should be *positively oriented* -- that is, it should go in counter-clockwise order.
    /// The interior rings should be *negatively oriented*.
    ///
    /// `bounds` should be the `(min, max)` values along each dimension the area enclosed by the
    /// polygon; they define an axis-aligned bounding rectangular prism for the polygon.
    /// `points[0]` should have the min-max values along the x-axis, `points[1]` should have
    /// min-max y-values, and `points[2]` should have min-max z-values/elevations.
    pub fn new_from_points<F: gfx::Factory<R>, R: gfx::Resources>(
        factory: &mut F,
        points: &[(f32, f32)],
        bounds: &[(f32, f32); 3],
    ) -> DrapeablePolygon {
        let polygon = Polygon {
            bounds: bounds.to_owned(),
            points: points.to_owned(),
        };
        let mut buffer = PolygonBuffer::new();
        let indices = buffer.add(&polygon);

        DrapeablePolygon {
            buffer: buffer,
            indices: indices,
        }
    }
}
