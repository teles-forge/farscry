use image::RgbImage;
use ndarray::Array4;
use oar_ocr_core::processors::ImageScaleInfo;

pub fn resize_for_detection(
    image: &RgbImage,
    limit_side_len: u32,
    limit_type: LimitType,
) -> (RgbImage, ImageScaleInfo) {
    let (orig_w, orig_h) = (image.width() as f32, image.height() as f32);

    let (new_w, new_h) = match limit_type {
        LimitType::Min => {
            let r = limit_side_len as f32 / orig_h.min(orig_w);
            ((orig_w * r).round() as u32, (orig_h * r).round() as u32)
        }
        LimitType::Max => {
            let r = limit_side_len as f32 / orig_h.max(orig_w);
            ((orig_w * r).round() as u32, (orig_h * r).round() as u32)
        }
    };

    let resized =
        image::imageops::resize(image, new_w, new_h, image::imageops::FilterType::Triangle);

    let (padded_w, padded_h) = (new_w.max(limit_side_len), new_h.max(limit_side_len));
    let mut padded = RgbImage::from_pixel(padded_w, padded_h, image::Rgb([128u8, 128u8, 128u8]));
    image::imageops::replace(&mut padded, &resized, 0, 0);

    let scale_info =
        ImageScaleInfo::new(orig_h, orig_w, new_h as f32 / orig_h, new_w as f32 / orig_w);
    (padded, scale_info)
}

#[derive(Debug, Clone, Copy)]
pub enum LimitType {
    Min,
    Max,
}

pub fn normalize_to_tensor(image: &RgbImage) -> Array4<f32> {
    let (h, w) = (image.height() as usize, image.width() as usize);
    let mut tensor = Array4::zeros((1, 3, h, w));

    for y in 0..h {
        for x in 0..w {
            let pixel = image.get_pixel(x as u32, y as u32);

            let b = (pixel[2] as f32 / 255.0 - 0.485) / 0.229;
            let g = (pixel[1] as f32 / 255.0 - 0.456) / 0.224;
            let r = (pixel[0] as f32 / 255.0 - 0.406) / 0.225;

            tensor[[0, 0, y, x]] = b;
            tensor[[0, 1, y, x]] = g;
            tensor[[0, 2, y, x]] = r;
        }
    }

    tensor
}

pub fn resize_for_recognition(
    image: &RgbImage,
    target_height: u32,
    max_width: usize,
) -> (RgbImage, usize) {
    let (orig_w, orig_h) = (image.width() as f32, image.height() as f32);
    let ratio = orig_w / orig_h;
    let new_w = ((target_height as f32 * ratio).ceil() as usize).min(max_width);

    let resized = image::imageops::resize(
        image,
        new_w as u32,
        target_height,
        image::imageops::FilterType::Triangle,
    );

    (resized, new_w)
}

pub fn normalize_recognition_to_tensor(image: &RgbImage, tensor_width: usize) -> Array4<f32> {
    let (h, w) = (image.height() as usize, image.width() as usize);
    let mut tensor = Array4::zeros((1, 3, h, tensor_width));

    for y in 0..h {
        for x in 0..w {
            let pixel = image.get_pixel(x as u32, y as u32);

            let b = (pixel[2] as f32 / 255.0 - 0.5) / 0.5;
            let g = (pixel[1] as f32 / 255.0 - 0.5) / 0.5;
            let r = (pixel[0] as f32 / 255.0 - 0.5) / 0.5;

            tensor[[0, 0, y, x]] = b;
            tensor[[0, 1, y, x]] = g;
            tensor[[0, 2, y, x]] = r;
        }
    }

    tensor
}
