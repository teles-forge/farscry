use farscry_core::{ElementType, VaspOutput};
use image::{DynamicImage, Rgba};
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;
use std::collections::HashSet;

/// Draw bounding boxes over every detected UI element.
///
/// Color coding:
///   Button                 = indigo  #6366f1
///   Input / Select         = cyan    #22d3ee
///   Error                  = red     #f87171
///   Heading                = amber   #fbbf24
///   Label / Badge          = gray    #a1a1aa
///   Unknown                = gray    #a1a1aa
///
/// Elements that are also affordances get a thicker border (3px vs 2px).
pub fn annotate_image(img: DynamicImage, output: &VaspOutput) -> DynamicImage {
    let mut rgba = img.to_rgba8();
    let img_w = rgba.width();
    let img_h = rgba.height();

    let affordance_keys: HashSet<(i32, i32)> = output
        .affordances
        .iter()
        .map(|a| (a.cx.round() as i32, a.cy.round() as i32))
        .collect();

    for element in &output.ui_tree {
        let color = element_color(&element.element_type);
        let is_affordance =
            affordance_keys.contains(&(element.cx.round() as i32, element.cy.round() as i32));

        let x = (element.cx - element.w / 2.0).floor().max(0.0) as i32;
        let y = (element.cy - element.h / 2.0).floor().max(0.0) as i32;
        let w = (element.w.ceil() as u32)
            .min(img_w.saturating_sub(x as u32))
            .max(1);
        let h = (element.h.ceil() as u32)
            .min(img_h.saturating_sub(y as u32))
            .max(1);

        let thickness: i32 = if is_affordance { 3 } else { 2 };

        for t in 0..thickness {
            let rx = (x - t).max(0);
            let ry = (y - t).max(0);
            let rw = ((w as i32 + t * 2) as u32).min(img_w.saturating_sub(rx as u32));
            let rh = ((h as i32 + t * 2) as u32).min(img_h.saturating_sub(ry as u32));
            if rw > 0 && rh > 0 {
                draw_hollow_rect_mut(&mut rgba, Rect::at(rx, ry).of_size(rw, rh), color);
            }
        }
    }

    DynamicImage::ImageRgba8(rgba)
}

fn element_color(et: &ElementType) -> Rgba<u8> {
    match et {
        ElementType::Button => Rgba([99, 102, 241, 230]),
        ElementType::Input | ElementType::Select => Rgba([34, 211, 238, 230]),
        ElementType::Error => Rgba([248, 113, 113, 230]),
        ElementType::Heading => Rgba([251, 191, 36, 230]),
        ElementType::Label | ElementType::Badge => Rgba([161, 161, 170, 180]),
        ElementType::Unknown => Rgba([161, 161, 170, 180]),
    }
}
