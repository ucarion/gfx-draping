//! A library for rendering polygons onto 3d terrain.
//!
//! This crate uses a screen-space algorithm; it uses your GPU's stencil buffer to detect pixels
//! where the polygon lies on terrain unoccluded. In addition, this crate is meant for drawing
//! multiple polygons at once. If you have a collection of polygons which may need to be drawn
//! simultaneously, put them all in a `PolygonBuffer`. You can choose which of those polygons to
//! render by combining their respective `PolygonBufferIndices`.
//!
//! Typically, usage of this crate will look something like:
//!
//! ```rust,compile_fail
//! extern crate geo;
//! extern crate gfx_draping;
//!
//! use gfx_draping::{DrapingRenderer, PolygonBuffer, PolygonBufferIndices};
//!
//! // Let's say you're using `geo` (a Rust GIS crate) to construct polygons.
//! let polygons: Vec<geo::Polygon> = a_vec_of_polygons();
//!
//! // Prepare assets for rendering.
//! let mut buffer = PolygonBuffer::new();
//! let mut indices = PolygonBufferIndices::new();
//! for polygon in polygons {
//!     indices.extend(buffer.add(polygon.into()));
//! }
//!
//! let renderer = DrapingRenderer::new();
//! let renderable_buffer = buffer.as_renderable(&mut window.factory);
//! let renderable_indices = indices.as_renderable(&mut window.factory);
//!
//! while your_event_loop() {
//!     // Render your 3d terrain
//!     render_terrain();
//!
//!     // At this point, your depth stencil should be the result of drawing the terrain. Your
//!     // stencil buffer should be all zeroes.
//!     renderer.render(
//!         window.encoder,
//!         window.output_color,
//!         window.output_stencil,
//!         // See docs for `DrapingRenderer::render` for a caveat about what `mvp` should be.
//!         your_scaled_mvp(),
//!         // R - G - B - A
//!         [1.0, 0.0, 1.0, 0.5],
//!         &renderable_buffer,
//!         &renderable_indices,
//!     );
//!
//!     // Now you can clear / clean-up as you do usually.
//! }
//! ```

extern crate geo;
#[macro_use]
extern crate gfx;

mod polygon;
mod render;
mod vertex;

pub use polygon::{Polygon, PolygonBuffer, PolygonBufferIndices};
pub use render::{DrapingRenderer, RenderablePolygonBuffer, RenderablePolygonIndices};
