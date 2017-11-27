use gfx;
use gfx::traits::FactoryExt;

use Vertex;

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

pub struct Polygon {
    pub bounds: [(f32, f32); 3],
    pub points: Vec<(f32, f32)>,
}

impl Polygon {
    fn bounding_box_vertices(&self) -> Vec<Vertex> {
        let bounding_ring = &[
            (self.bounds[0].0, self.bounds[1].0),
            (self.bounds[0].1, self.bounds[1].0),
            (self.bounds[0].1, self.bounds[1].1),
            (self.bounds[0].0, self.bounds[1].1),
            (self.bounds[0].0, self.bounds[1].0),
        ];

        Self::prism_vertices(bounding_ring, self.bounds[2].0, self.bounds[2].1)
    }

    fn bounding_box_indices(&self) -> Vec<u32> {
        Self::prism_indices(5)
    }

    fn polyhedron_vertices(&self) -> Vec<Vertex> {
        Self::prism_vertices(&self.points, self.bounds[2].0, self.bounds[2].1)
    }

    fn polyhedron_indices(&self) -> Vec<u32> {
        Self::prism_indices(self.points.len() as u32)
    }

    fn prism_vertices(
        points: &[(f32, f32)],
        height_lower_bound: f32,
        height_upper_bound: f32,
    ) -> Vec<Vertex> {
        points
            .iter()
            .flat_map(|&(x, y)| {
                let below = Vertex { position: [x, y, height_lower_bound] };
                let above = Vertex { position: [x, y, height_upper_bound] };
                vec![below, above]
            })
            .collect()
    }

    fn prism_indices(num_points: u32) -> Vec<u32> {
        (0..num_points)
            .flat_map(|index| {
                let below_index = 2 * index;
                let above_index = below_index + 1;
                let after_below_index = 2 * ((1 + index) % num_points);
                let after_above_index = after_below_index + 1;

                // When on an exterior ring, whose points are in counter-clockwise orientation,
                // this face should face outward.
                //
                // For interior rings, with clockwise orientation, this face should face inward.
                let mut indices = vec![
                    below_index, after_below_index, above_index,
                    after_below_index, after_above_index, above_index,
                ];

                if index != 0 && index != num_points - 1 {
                    // The top faces should face upward; the bottom faces, downward.
                    let cap_triangles = vec![
                        0, after_below_index, below_index,
                        1, above_index, after_above_index,
                    ];

                    indices.extend(cap_triangles);
                }

                indices
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct PolygonBuffer {
    polyhedron_vertices: Vec<Vertex>,
    bounding_box_vertices: Vec<Vertex>,
}

impl PolygonBuffer {
    pub fn new() -> PolygonBuffer {
        PolygonBuffer {
            polyhedron_vertices: Vec::new(),
            bounding_box_vertices: Vec::new(),
        }
    }

    pub fn add(&mut self, polygon: &Polygon) -> PolygonBufferIndices {
        let polyhedron_offset = self.polyhedron_vertices.len() as u32;
        let bounding_box_offset = self.bounding_box_vertices.len() as u32;

        self.polyhedron_vertices.extend(polygon.polyhedron_vertices());
        self.bounding_box_vertices.extend(polygon.bounding_box_vertices());

        PolygonBufferIndices {
            polyhedron_indices: polygon.polyhedron_indices().iter().map(|i| i + polyhedron_offset).collect(),
            bounding_box_indices: polygon.bounding_box_indices().iter().map(|i| i + bounding_box_offset).collect()
        }
    }

    fn polyhedron_vertex_buffer<F: gfx::Factory<R>, R: gfx::Resources>(&self, factory: &mut F) -> gfx::handle::Buffer<R, Vertex> {
        factory.create_vertex_buffer(&self.polyhedron_vertices)
    }

    fn bounding_box_vertex_buffer<F: gfx::Factory<R>, R: gfx::Resources>(&self, factory: &mut F) -> gfx::handle::Buffer<R, Vertex> {
        factory.create_vertex_buffer(&self.bounding_box_vertices)
    }
}

#[derive(Debug)]
pub struct PolygonBufferIndices {
    polyhedron_indices: Vec<u32>,
    bounding_box_indices: Vec<u32>,
}

impl PolygonBufferIndices {
    fn polyhedron_slice<F: gfx::Factory<R>, R: gfx::Resources>(&self, factory: &mut F) -> gfx::Slice<R> {
        gfx::Slice {
            start: 0,
            end: self.polyhedron_indices.len() as u32,
            base_vertex: 0,
            instances: None,
            buffer: factory.create_index_buffer(&self.polyhedron_indices[..]),
        }
    }

    fn bounding_box_slice<F: gfx::Factory<R>, R: gfx::Resources>(&self, factory: &mut F) -> gfx::Slice<R> {
        gfx::Slice {
            start: 0,
            end: self.bounding_box_indices.len() as u32,
            base_vertex: 0,
            instances: None,
            buffer: factory.create_index_buffer(&self.bounding_box_indices[..]),
        }
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
