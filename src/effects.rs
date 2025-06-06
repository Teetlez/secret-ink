pub mod effects {
    use image::{RgbImage, Rgba, RgbaImage};
    use photon_rs::PhotonImage;
    use photon_rs::affine;
    use photon_rs::channels::alter_channel;
    use photon_rs::conv;
    use photon_rs::filters::{self, FilterType};
    use photon_rs::helpers;
    use photon_rs::monochrome;
    use photon_rs::multiple::{self, join_images};
    use photon_rs::native::{open_image, save_image};
    use photon_rs::transform::{SamplingFilter, resize};

    /// Applies a series of aging/scan effects to the given image in-place
    pub fn apply_aging_effects(img: &mut RgbImage) {
        // Convert to PhotonImage
        let mut photon_img = PhotonImage::new(img.clone());

        // 1. Add paper grain / noise (e.g., use a dithering or noise filter)
        photon_rs::filters::filter(&mut photon_img, FilterType::Unsharpen);
        // Unsharpening can add some edge irregularities; adjust as needed

        // 2. Apply slight Gaussian blur to soften edges
        photon_rs::filters::filter(&mut photon_img, FilterType::GaussianBlur);

        // 3. Apply a very subtle vignette or sepia tint (optional)
        photon_rs::filters::filter(&mut photon_img, FilterType::Vintage);

        // 4. Add scratch/grain overlay: for more control, you could load a PNG
        //    texture of paper grain or scratches and overlay it with low opacity.
        //    (Here we skip that for boilerplate.)

        // 5. (Optional) Skew or rotate slightly to simulate scan misalignment
        //    e.g., affine rotate by ±1° and then resize/crop back
        //    let angle = 1.0f32.to_radians();
        //    photon_rs::affine::rotate(&mut photon_img, angle, Rgba([0,0,0,0]));

        // Convert back to RgbImage
        let raw_buffer = photon_img.get_raw_pixels();
        let (width, height) = (photon_img.get_width(), photon_img.get_height());
        let mut out_img = RgbaImage::new(width, height);
        for (i, pixel) in raw_buffer.chunks(4).enumerate() {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            out_img.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], pixel[3]]));
        }
        // Flatten alpha (since our image is opaque)
        for pixel in out_img.pixels_mut() {
            *pixel = Rgba([pixel[0], pixel[1], pixel[2], 255]);
        }
        // Copy back into the original RgbImage
        for (x, y, px) in out_img.enumerate_pixels() {
            img.put_pixel(x, y, image::Rgb([px[0], px[1], px[2]]));
        }
    }
}
