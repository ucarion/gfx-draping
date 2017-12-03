use geo;
use geo::algorithm::boundingbox::BoundingBox;
use gfx;

use render::*;
use vertex::Vertex;

/// A collection of polygons that could all be rendered in a single draw call.
#[derive(Clone, Debug)]
pub struct PolygonBuffer {
    pub(crate) polyhedron_vertices: Vec<Vertex>,
    pub(crate) bounding_box_vertices: Vec<Vertex>,
}

impl PolygonBuffer {
    /// Create a new, empty buffer.
    pub fn new() -> PolygonBuffer {
        PolygonBuffer {
            polyhedron_vertices: Vec::new(),
            bounding_box_vertices: Vec::new(),
        }
    }

    /// Add a polygon to this buffer.
    ///
    /// The `PolygonBufferIndices` returned can be used to render the passed polygon in a future
    /// call to `DrapingRenderer::render` using this buffer.
    pub fn add(&mut self, polygon: &Polygon) -> PolygonBufferIndices {
        let polyhedron_offset = self.polyhedron_vertices.len() as u32;
        let bounding_box_offset = self.bounding_box_vertices.len() as u32;

        self.polyhedron_vertices.extend(
            polygon.polyhedron_vertices(),
        );
        self.bounding_box_vertices.extend(
            polygon.bounding_box_vertices(),
        );

        PolygonBufferIndices {
            polyhedron_indices: polygon
                .polyhedron_indices()
                .map(|i| i + polyhedron_offset)
                .collect(),
            bounding_box_indices: polygon
                .bounding_box_indices()
                .map(|i| i + bounding_box_offset)
                .collect(),
        }
    }

    /// Prepare this buffer for rendering.
    pub fn as_renderable<F: gfx::Factory<R>, R: gfx::Resources>(
        &self,
        factory: &mut F,
    ) -> RenderablePolygonBuffer<R> {
        RenderablePolygonBuffer::new(factory, &self)
    }
}

/// A set of indices into a `PolygonBuffer`.
///
/// You can combine these indices using `extend` to render multiple polygons at once.
#[derive(Clone, Debug)]
pub struct PolygonBufferIndices {
    pub(crate) polyhedron_indices: Vec<u32>,
    pub(crate) bounding_box_indices: Vec<u32>,
}

impl PolygonBufferIndices {
    /// Create an empty set of indices.
    ///
    /// Rendering the returned indices would be a no-op unless you call `extend` on it. This is a
    /// convenience method that you can use as the "zero" value to a `reduce`-like operation.
    pub fn new() -> PolygonBufferIndices {
        PolygonBufferIndices {
            polyhedron_indices: Vec::new(),
            bounding_box_indices: Vec::new(),
        }
    }

    /// Add all the polygons in `other` into this set of indices.
    ///
    /// After calling `extend`, rendering `this` will draw all the polygons previously in `this` as
    /// well as all the polygons in `other`. In other words, you can think of this as a
    /// "union"/"add all" operation.
    pub fn extend(&mut self, other: &PolygonBufferIndices) {
        self.polyhedron_indices.extend_from_slice(
            &other.polyhedron_indices,
        );
        self.bounding_box_indices.extend_from_slice(
            &other.bounding_box_indices,
        );
    }

    /// Prepare these indices for rendering.
    pub fn as_renderable<F: gfx::Factory<R>, R: gfx::Resources>(
        &self,
        factory: &mut F,
    ) -> RenderablePolygonIndices<R> {
        RenderablePolygonIndices::new(factory, &self)
    }
}

/// A polygon with a bounding box.
///
/// This struct implements `From<geoo:Polygon>`, so for GIS applications you can instantiate this
/// from any `geo::Polygon`.
#[derive(Clone, Debug)]
pub struct Polygon {
    bounding_ring: [(f32, f32); 5],
    points: Vec<(f32, f32)>,
}

impl Polygon {
    /// Construct a Polygon from a bounding-box and set of points.
    ///
    /// `bounds` should be `[(min_x, max_x), (min_y, max_y)]`.
    ///
    /// `points` should be an exterior ring concatenated with a (possibly empty) set of interior
    /// rings, where a "ring" is a list of points where the first and last point are equal.
    ///
    /// The exterior ring of `points` should be *positively oriented*, i.e. it should go in
    /// counter-clockwise order. The interior rings should be *negatively oriented*.
    pub fn new(bounds: [(f32, f32); 2], points: Vec<(f32, f32)>) -> Polygon {
        let bounding_ring = [
            (bounds[0].0, bounds[1].0),
            (bounds[0].1, bounds[1].0),
            (bounds[0].1, bounds[1].1),
            (bounds[0].0, bounds[1].1),
            (bounds[0].0, bounds[1].0),
        ];

        Polygon {
            bounding_ring: bounding_ring,
            points: points,
        }
    }

    fn bounding_box_vertices<'a>(&'a self) -> Box<'a + Iterator<Item = Vertex>> {
        Box::new(Self::prism_vertices(&self.bounding_ring))
    }

    fn bounding_box_indices(&self) -> Box<Iterator<Item = u32>> {
        Self::prism_indices(5)
    }

    fn polyhedron_vertices<'a>(&'a self) -> Box<'a + Iterator<Item = Vertex>> {
        Self::prism_vertices(&self.points)
    }

    fn polyhedron_indices(&self) -> Box<Iterator<Item = u32>> {
        Self::prism_indices(self.points.len() as u32)
    }

    fn prism_vertices<'a>(points: &'a [(f32, f32)]) -> Box<'a + Iterator<Item = Vertex>> {
        Box::new(points.iter().flat_map(move |&(x, y)| {
            let below = Vertex { position: [x, y, 0.0] };
            let above = Vertex { position: [x, y, 1.0] };
            vec![below, above]
        }))
    }

    fn prism_indices(num_points: u32) -> Box<Iterator<Item = u32>> {
        Box::new((0..num_points).flat_map(move |index| {
            let below_index = 2 * index;
            let above_index = below_index + 1;
            let after_below_index = 2 * ((1 + index) % num_points);
            let after_above_index = after_below_index + 1;

            // When on an exterior ring, whose points are in counter-clockwise orientation,
            // this face should face outward.
            //
            // For interior rings, with clockwise orientation, this face should face inward.
            let mut indices = vec![
                below_index,
                after_below_index,
                above_index,
                after_below_index,
                after_above_index,
                above_index,
            ];

            if index != 0 && index != num_points - 1 {
                // The top faces should face upward; the bottom faces, downward.
                let cap_triangles = vec![
                    0,
                    after_below_index,
                    below_index,
                    1,
                    above_index,
                    after_above_index,
                ];

                indices.extend(cap_triangles);
            }

            indices
        }))
    }
}

impl From<geo::Polygon<f32>> for Polygon {
    fn from(polygon: geo::Polygon<f32>) -> Polygon {
        let bounding_box = polygon.bbox().unwrap();
        let bounds = [
            (bounding_box.xmin, bounding_box.xmax),
            (bounding_box.ymin, bounding_box.ymax),
        ];

        let mut points = Vec::new();
        points.extend(polygon.exterior.into_iter().map(|point| (point.x(), point.y())));

        for interior in polygon.interiors {
            points.extend(interior.into_iter().map(|point| (point.x(), point.y())));
        }

        Polygon::new(bounds, points)
    }
}
