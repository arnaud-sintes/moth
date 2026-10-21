//! JPEG encoding of a rendered [`Image`].
//!
//! This lives here, behind a function that is not generic, on purpose: `jpeg-encoder`'s inner loops are
//! generic over the writer, so they are compiled in the crate that *calls* the encoder, at that crate's
//! optimization level. Called from an unoptimized dev build of the server they ran six times slower
//! (30 ms instead of 5 ms for a 1030x686 frame). Here they are always optimized.

use jpeg_encoder::{ColorType, Encoder, EncodingError, SamplingFactor};

use crate::Image;

/// Encode as a baseline JPEG, 4:2:0, as video does: the eye resolves colour far less finely than
/// brightness, and it makes the encode about 40% faster and the file about 30% smaller. `quality` is
/// 1 to 100.
pub fn encode_jpeg(image: &Image, quality: u8) -> Result<Vec<u8>, EncodingError> {
	let mut jpeg = Vec::with_capacity(image.rgb.len() / 10);
	let mut encoder = Encoder::new(&mut jpeg, quality.clamp(1, 100));
	encoder.set_sampling_factor(SamplingFactor::F_2_2);
	encoder.encode(&image.rgb, image.width as u16, image.height as u16, ColorType::Rgb)?;
	Ok(jpeg)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Mesh, Settings, render};

	/// The size a JPEG declares in its first start-of-frame marker.
	fn declared_size(jpeg: &[u8]) -> Option<(u16, u16)> {
		let mut at = 2;
		while at + 9 < jpeg.len() {
			if jpeg[at] != 0xFF {
				return None;
			}
			let marker = jpeg[at + 1];
			let length = usize::from(u16::from_be_bytes([jpeg[at + 2], jpeg[at + 3]]));
			if matches!(marker, 0xC0..=0xC2) {
				let height = u16::from_be_bytes([jpeg[at + 5], jpeg[at + 6]]);
				let width = u16::from_be_bytes([jpeg[at + 7], jpeg[at + 8]]);
				return Some((width, height));
			}
			at += 2 + length;
		}
		None
	}

	fn frame() -> Image {
		render(&Mesh::torus(48, 24), &Settings { width: 200, height: 120, supersample: 2, ..Settings::default() })
	}

	#[test]
	fn the_output_is_a_jpeg_of_the_size_of_the_image() {
		let jpeg = encode_jpeg(&frame(), 85).unwrap();
		assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "starts with SOI");
		assert_eq!(&jpeg[jpeg.len() - 2..], &[0xFF, 0xD9], "ends with EOI");
		assert_eq!(declared_size(&jpeg), Some((200, 120)));
	}

	#[test]
	fn a_higher_quality_is_a_bigger_file_and_the_quality_is_clamped() {
		let image = frame();
		let low = encode_jpeg(&image, 30).unwrap().len();
		let high = encode_jpeg(&image, 95).unwrap().len();
		assert!(high > low, "{low} < {high}");
		encode_jpeg(&image, 0).unwrap();
		encode_jpeg(&image, 255).unwrap();
	}

	#[test]
	fn encoding_is_deterministic() {
		let image = frame();
		assert_eq!(encode_jpeg(&image, 80).unwrap(), encode_jpeg(&image, 80).unwrap());
	}

	#[test]
	fn a_picture_of_one_pixel_encodes() {
		let image = render(&Mesh::gem(), &Settings { width: 1, height: 1, ..Settings::default() });
		assert_eq!(declared_size(&encode_jpeg(&image, 80).unwrap()), Some((1, 1)));
	}
}
