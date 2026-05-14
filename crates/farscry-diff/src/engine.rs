use crate::classify::{classify_delta_entries, compute_tokens_saved};
use crate::similarity::compute_context_similarity;
use crate::spatial::{compute_scroll_offset, full_bipartite_match};
use crate::text_match::rough_text_match;
use farscry_core::{DeltaEntry, DiffEngine, UiElement, VaspDelta, VaspOutput};

const CONTEXT_SIMILARITY_THRESHOLD: f32 = 0.20;
const ROUGH_MATCH_THRESHOLD: f32 = 0.70;

pub struct DiffEngineImpl;

impl DiffEngine for DiffEngineImpl {
    fn diff(
        &self,
        before: &VaspOutput,
        after: &VaspOutput,
        before_dims: Option<(u32, u32)>,
        after_dims: Option<(u32, u32)>,
    ) -> VaspDelta {
        let tokens_saved = compute_tokens_saved(before_dims, after_dims);
        let context_similarity = compute_context_similarity(before, after);
        let context_changed = context_similarity < CONTEXT_SIMILARITY_THRESHOLD;

        if context_changed {
            return VaspDelta {
                vasp_version: "1.0".to_string(),
                diff_from: before.state_id,
                diff_to: after.state_id,
                context_similarity,
                context_changed: true,
                agent_context: after.agent_context.clone(),
                entries: Vec::new(),
                tokens_saved,
            };
        }

        let entries = build_delta_entries(&before.ui_tree, &after.ui_tree);

        VaspDelta {
            vasp_version: "1.0".to_string(),
            diff_from: before.state_id,
            diff_to: after.state_id,
            context_similarity,
            context_changed: false,
            agent_context: after.agent_context.clone(),
            entries,
            tokens_saved,
        }
    }
}

fn build_delta_entries(before: &[UiElement], after: &[UiElement]) -> Vec<DeltaEntry> {
    let rough_matches = rough_text_match(before, after, ROUGH_MATCH_THRESHOLD);
    let scroll_offset = compute_scroll_offset(&rough_matches, before, after);
    let matches = full_bipartite_match(before, after, &scroll_offset);
    classify_delta_entries(before, after, &matches)
}
