//! A small software renderer for the viewer sample: procedural meshes, an orbit camera, Blinn-Phong
//! shading, optional wireframe, supersampling, and a rasterizer that works on all cores.
//!
//! It knows nothing about the network or about the sample's protocol; the server maps a client's
//! request onto [`Settings`] (after clamping it: the request is untrusted).

pub mod jpeg;
pub mod math;
pub mod mesh;
pub mod raster;

pub use jpeg::encode_jpeg;
pub use mesh::Mesh;
pub use raster::{Image, Material, Settings, render};
