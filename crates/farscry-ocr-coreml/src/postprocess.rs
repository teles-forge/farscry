use farscry_core::TextRegion;
use ndarray::Array4;
use oar_ocr_core::processors::db_postprocess::{DBPostProcess, DBPostProcessConfig};
use oar_ocr_core::processors::types::{BoxType, ScoreMode};
use oar_ocr_core::processors::CTCLabelDecode;
use oar_ocr_core::processors::ImageScaleInfo;

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub points: Vec<(f32, f32)>,
    pub score: f32,
}

impl BoundingBox {
    pub fn new(points: Vec<(f32, f32)>, score: f32) -> Self {
        Self { points, score }
    }
}

pub fn db_postprocess(
    predictions: &Array4<f32>,
    img_shapes: Vec<ImageScaleInfo>,
    score_threshold: f32,
    box_threshold: f32,
    unclip_ratio: f32,
) -> Vec<Vec<BoundingBox>> {
    let db_postprocess = DBPostProcess::new(
        Some(score_threshold),
        Some(box_threshold),
        None,
        Some(unclip_ratio),
        None,
        Some(ScoreMode::Fast),
        Some(BoxType::Quad),
    );

    let config = DBPostProcessConfig::new(score_threshold, box_threshold, unclip_ratio);
    let (oar_boxes, _oar_scores) = db_postprocess.apply(predictions, img_shapes, Some(&config));

    oar_boxes
        .into_iter()
        .map(|batch_boxes| {
            batch_boxes
                .into_iter()
                .map(|oar_box| {
                    let points: Vec<(f32, f32)> =
                        oar_box.points.iter().map(|p| (p.x, p.y)).collect();

                    BoundingBox::new(points, 1.0)
                })
                .collect()
        })
        .collect()
}

pub fn ctc_decode(
    predictions: &ndarray::Array3<f32>,
    character_dict: &[String],
) -> (Vec<String>, Vec<f32>) {
    let decoder = CTCLabelDecode::from_string_list(Some(character_dict), true, false);
    let (texts, scores) = decoder.apply(predictions);
    (texts, scores)
}

pub fn boxes_to_regions(
    boxes: &[BoundingBox],
    texts: &[String],
    scores: &[f32],
) -> Vec<TextRegion> {
    boxes
        .iter()
        .zip(texts.iter())
        .zip(scores.iter())
        .map(|((box_, text), _score)| {
            let (min_x, max_x) = box_
                .points
                .iter()
                .fold((f32::MAX, f32::MIN), |(min, max), (x, _)| {
                    (min.min(*x), max.max(*x))
                });
            let (min_y, max_y) = box_
                .points
                .iter()
                .fold((f32::MAX, f32::MIN), |(min, max), (_, y)| {
                    (min.min(*y), max.max(*y))
                });

            TextRegion {
                cx: min_x,
                cy: min_y,
                w: (max_x - min_x),
                h: (max_y - min_y),
                text: text.clone(),
            }
        })
        .collect()
}
