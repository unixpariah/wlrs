use fast_image_resize::{FilterType, PixelType, Resizer};
use image::RgbImage;
use std::{error::Error, num::NonZeroU32};

pub fn resize_image(image: RgbImage, width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let (img_w, img_h) = image.dimensions();
    let ratio = width as f32 / height as f32;
    let img_r = img_w as f32 / img_h as f32;

    let (trg_w, trg_h) = if ratio > img_r {
        let scale = height as f32 / img_h as f32;
        ((img_w as f32 * scale) as u32, height as u32)
    } else {
        let scale = width as f32 / img_w as f32;
        (width as u32, (img_h as f32 * scale) as u32)
    };

    let trg_w = trg_w.min(width as u32);
    let trg_h = trg_h.min(height as u32);

    let src = fast_image_resize::Image::from_vec_u8(
        NonZeroU32::new(img_w).unwrap(),
        NonZeroU32::new(img_h).unwrap(),
        image.into_raw(),
        PixelType::U8x3,
    )?;

    let new_w = NonZeroU32::new(trg_w).unwrap();
    let new_h = NonZeroU32::new(trg_h).unwrap();

    let mut dst = fast_image_resize::Image::new(new_w, new_h, PixelType::U8x3);
    let mut dst_view = dst.view_mut();

    let mut resizer = Resizer::new(fast_image_resize::ResizeAlg::Convolution(
        FilterType::Lanczos3,
    ));

    resizer.resize(&src.view(), &mut dst_view)?;

    let dst = dst.into_vec();

    let mut rgba_dst = Vec::with_capacity(dst.len() / 3 * 4);
    dst.chunks(3).for_each(|rgb_pixels| {
        rgba_dst.extend_from_slice(rgb_pixels);
        rgba_dst.push(255);
    });

    Ok(rgba_dst)
}
