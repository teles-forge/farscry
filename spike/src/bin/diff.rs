


use oar_ocr::core::config::{OrtGraphOptimizationLevel, OrtSessionConfig};
use oar_ocr::domain::TextDetectionConfig;
use oar_ocr::prelude::*;
use oar_ocr::processors::LimitType;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;


#[derive(Debug, Clone)]
struct Region {
    text: String,
    cx: f32,
    cy: f32,
}


#[derive(Debug, Clone, PartialEq)]
enum ElemType {
    Error,
    Title,
    Label,
    Button,
    Text,
}

fn infer_type(text: &str) -> ElemType {
    let l = text.to_lowercase();
    if l.starts_with("error") || l.starts_with("err:") || l.starts_with("warning") {
        return ElemType::Error;
    }
    let stripped = l.trim();
    if stripped.ends_with(':') {
        return ElemType::Label;
    }
    if matches!(
        stripped,
        "submit" | "cancel" | "ok" | "save" | "delete" | "pay now"
            | "continue" | "next" | "back" | "close"
    ) {
        return ElemType::Button;
    }
    let words = stripped.split_whitespace().count();
    let starts_upper = stripped.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
    if words <= 4 && starts_upper && !stripped.contains(':') {
        return ElemType::Title;
    }
    ElemType::Text
}


fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut row: Vec<usize> = (0..=m).collect();
    for i in 1..=n {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=m {
            let old = row[j];
            row[j] = if a[i - 1] == b[j - 1] {
                prev
            } else {
                1 + prev.min(row[j]).min(row[j - 1])
            };
            prev = old;
        }
    }
    row[m]
}

fn text_similarity(a: &str, b: &str) -> f32 {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    if a == b {
        return 1.0;
    }
    let dist = levenshtein(a.as_bytes(), b.as_bytes());
    let max_len = a.len().max(b.len()).max(1);
    1.0 - (dist as f32 / max_len as f32).min(1.0)
}


fn pos_proximity(ax: f32, ay: f32, bx: f32, by: f32, scroll_dy: f32) -> f32 {
    let dx = bx - ax;
    let dy = (by - scroll_dy) - ay;
    let dist_sq = dx * dx + dy * dy;
    const SIGMA: f32 = 80.0;
    (-dist_sq / (2.0 * SIGMA * SIGMA)).exp()
}


fn match_score(a: &Region, b: &Region, scroll_dy: f32) -> f32 {
    let t = text_similarity(&a.text, &b.text);
    let p = pos_proximity(a.cx, a.cy, b.cx, b.cy, scroll_dy);
    let m: f32 = if infer_type(&a.text) == infer_type(&b.text) {
        1.0
    } else {
        0.5
    };
    0.4 * t + 0.4 * p + 0.2 * m
}


fn estimate_scroll_dy(before: &[Region], after: &[Region]) -> f32 {
    let mut dys: Vec<f32> = before
        .iter()
        .flat_map(|a| {
            after.iter().filter_map(|b| {
                if text_similarity(&a.text, &b.text) >= 0.70 {
                    Some(b.cy - a.cy)
                } else {
                    None
                }
            })
        })
        .collect();

    if dys.is_empty() {
        return 0.0;
    }
    dys.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    let mid = dys.len() / 2;
    if dys.len() % 2 == 0 {
        (dys[mid - 1] + dys[mid]) / 2.0
    } else {
        dys[mid]
    }
}


fn greedy_match(
    before: &[Region],
    after: &[Region],
    scroll_dy: f32,
    threshold: f32,
) -> Vec<Option<usize>> {
    let mut candidates: Vec<(usize, usize, f32)> = before
        .iter()
        .enumerate()
        .flat_map(|(i, a)| {
            after.iter().enumerate().filter_map(move |(j, b)| {
                let s = match_score(a, b, scroll_dy);
                if s >= threshold {
                    Some((i, j, s))
                } else {
                    None
                }
            })
        })
        .collect();


    candidates
        .sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut used_b = vec![false; before.len()];
    let mut used_a = vec![false; after.len()];
    let mut matches: Vec<Option<usize>> = vec![None; before.len()];

    for (i, j, _) in candidates {
        if !used_b[i] && !used_a[j] {
            matches[i] = Some(j);
            used_b[i] = true;
            used_a[j] = true;
        }
    }
    matches
}


#[derive(Debug)]
enum Status {
    Unchanged,
    Changed { from: String, to: String },
    Appeared,
    Removed,
}

#[derive(Debug)]
struct Entry {
    text: String,
    status: Status,
}

struct DiffResult {
    entries: Vec<Entry>,
    scroll_dy: f32,
}

fn run_diff(before: &[Region], after: &[Region]) -> DiffResult {
    const MATCH_THRESHOLD: f32 = 0.60;
    const UNCHANGED_THRESHOLD: f32 = 0.95;

    let scroll_dy = estimate_scroll_dy(before, after);
    let matches = greedy_match(before, after, scroll_dy, MATCH_THRESHOLD);

    let mut entries: Vec<Entry> = Vec::new();
    let mut matched_after: HashSet<usize> = HashSet::new();

    for (i, m) in matches.iter().enumerate() {
        match m {
            Some(j) => {
                matched_after.insert(*j);
                let bt = &before[i].text;
                let at = &after[*j].text;
                let status = if text_similarity(bt, at) >= UNCHANGED_THRESHOLD {
                    Status::Unchanged
                } else {
                    Status::Changed {
                        from: bt.clone(),
                        to: at.clone(),
                    }
                };
                entries.push(Entry {
                    text: bt.clone(),
                    status,
                });
            }
            None => {
                entries.push(Entry {
                    text: before[i].text.clone(),
                    status: Status::Removed,
                });
            }
        }
    }

    for (j, b) in after.iter().enumerate() {
        if !matched_after.contains(&j) {
            entries.push(Entry {
                text: b.text.clone(),
                status: Status::Appeared,
            });
        }
    }

    DiffResult { entries, scroll_dy }
}


fn build_ocr() -> Result<OAROCR, Box<dyn std::error::Error>> {
    let models = Path::new("models");
    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let physical = if cfg!(target_arch = "x86_64") {
        (logical / 2).max(1)
    } else {
        logical
    };

    let ort_cfg = OrtSessionConfig::new()
        .with_intra_threads(physical)
        .with_inter_threads(1)
        .with_optimization_level(OrtGraphOptimizationLevel::Level2);

    let ocr = OAROCRBuilder::new(
        models.join("pp-ocrv5_mobile_det.onnx"),
        models.join("en_pp-ocrv5_mobile_rec.onnx"),
        models.join("ppocrv5_en_dict.txt"),
    )
    .ort_session(ort_cfg)
    .text_detection_config(TextDetectionConfig {
        limit_side_len: Some(640),
        limit_type: Some(LimitType::Max),
        ..TextDetectionConfig::default()
    })
    .region_batch_size(32)
    .build()?;

    Ok(ocr)
}

fn ocr_file(ocr: &OAROCR, path: &str) -> Result<Vec<Region>, Box<dyn std::error::Error>> {
    let img = load_image(Path::new(path))?;
    let results = ocr.predict(vec![img])?;
    let regions = results[0]
        .text_regions
        .iter()
        .filter_map(|r| {
            let text = r.text.as_deref()?.to_string();
            let c = r.bounding_box.center();
            Some(Region {
                text,
                cx: c.x,
                cy: c.y,
            })
        })
        .collect();
    Ok(regions)
}


fn print_delta(result: &DiffResult) -> (usize, usize, usize, usize) {
    let (mut unch, mut chg, mut app, mut rem) = (0usize, 0, 0, 0);
    for e in &result.entries {
        let (tag, detail) = match &e.status {
            Status::Unchanged => {
                unch += 1;
                ("UNCHANGED", format!("\"{}\"", e.text))
            }
            Status::Changed { from, to } => {
                chg += 1;
                ("CHANGED  ", format!("\"{}\"  ->  \"{}\"", from, to))
            }
            Status::Appeared => {
                app += 1;
                ("APPEARED ", format!("\"{}\"", e.text))
            }
            Status::Removed => {
                rem += 1;
                ("REMOVED  ", format!("\"{}\"", e.text))
            }
        };
        println!("  [{tag}] {detail}");
    }
    (unch, chg, app, rem)
}


fn verdict_test1(result: &DiffResult, before: &[Region], after: &[Region]) -> bool {
    println!("\n── Test 1: Scroll Detection ──");
    println!(
        "  Before: {} regions   After: {} regions",
        before.len(),
        after.len()
    );
    println!("  scroll_dy detected: {:.0}px", result.scroll_dy);

    println!("\n  Delta output:");
    let (unch, chg, app, rem) = print_delta(result);
    println!("\n  Counts: {rem} removed / {unch} unchanged / {app} appeared / {chg} changed");

    let scroll_ok = result.scroll_dy.abs() > 100.0;
    let no_false_changed = chg == 0;

    println!(
        "\n  Scroll correction: {}  (|dy|={:.0}px, need >100)",
        if scroll_ok { " ACTIVATED" } else { "No NOT detected" },
        result.scroll_dy.abs()
    );
    println!(
        "  False positives (wrongly CHANGED in matched set): {}",
        chg
    );

    let go = scroll_ok && no_false_changed;
    println!("  Verdict: {}", if go { " GO" } else { "No NO-GO" });
    go
}

fn verdict_test2(result: &DiffResult, before: &[Region], after: &[Region]) -> bool {
    println!("\n── Test 2: Field Filled ──");
    println!(
        "  Before: {} regions   After: {} regions",
        before.len(),
        after.len()
    );
    println!("  scroll_dy detected: {:.0}px", result.scroll_dy);

    println!("\n  Delta output:");
    let (unch, chg, app, rem) = print_delta(result);
    println!("\n  Counts: {rem} removed / {unch} unchanged / {app} appeared / {chg} changed");

    let scroll_ok = result.scroll_dy.abs() < 25.0;
    let changed_ok = chg >= 1;

    println!(
        "\n  Scroll ~0: {}  (dy={:.0}px)",
        if scroll_ok { "" } else { " non-zero" },
        result.scroll_dy
    );
    println!(
        "  Changed fields detected: {}  (need >=1)",
        chg
    );

    let go = scroll_ok && changed_ok;
    println!("  Verdict: {}", if go { " GO" } else { "No NO-GO" });
    go
}

fn verdict_test3(result: &DiffResult, before: &[Region], after: &[Region]) -> bool {
    println!("\n── Test 3: Error Appeared ──");
    println!(
        "  Before: {} regions   After: {} regions",
        before.len(),
        after.len()
    );
    println!("  scroll_dy detected: {:.0}px", result.scroll_dy);

    println!("\n  Delta output:");
    let (unch, chg, app, rem) = print_delta(result);
    println!("\n  Counts: {rem} removed / {unch} unchanged / {app} appeared / {chg} changed");

    let scroll_ok = result.scroll_dy.abs() < 25.0;
    let error_appeared = result.entries.iter().any(|e| {
        matches!(e.status, Status::Appeared)
            && (e.text.to_lowercase().contains("error")
                || e.text.to_lowercase().contains("payment"))
    });
    let no_false_changed = chg == 0;

    println!(
        "\n  Scroll ~0: {}  (dy={:.0}px)",
        if scroll_ok { "" } else { " non-zero" },
        result.scroll_dy
    );
    println!(
        "  Error in appeared: {}",
        if error_appeared { " yes" } else { "No not detected" }
    );
    println!(
        "  False positives (wrongly CHANGED): {}",
        chg
    );

    let go = scroll_ok && error_appeared && no_false_changed;
    println!("  Verdict: {}", if go { " GO" } else { "No NO-GO" });
    go
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  farscry - Diff Engine Spike (bipartite matching)            ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let tests: &[(&str, &str, &str)] = &[
        (
            "TEST 1 - Scroll",
            "diff_test/test1_scroll/before.png",
            "diff_test/test1_scroll/after.png",
        ),
        (
            "TEST 2 - Field Filled",
            "diff_test/test2_field/before.png",
            "diff_test/test2_field/after.png",
        ),
        (
            "TEST 3 - Error Appeared",
            "diff_test/test3_error/before.png",
            "diff_test/test3_error/after.png",
        ),
    ];


    for (name, bp, ap) in tests {
        for p in [bp, ap] {
            if !Path::new(p).exists() {
                eprintln!(
                    "ERROR: {name} missing image {p}\n  Run: uv run diff_test/generate_images.py"
                );
                std::process::exit(1);
            }
        }
    }

    print!("Building OCR pipeline (A+B+C+D: Level2, 640px, batch32)... ");
    std::io::stdout().flush()?;
    let ocr = build_ocr()?;
    println!("ready.");
    println!();

    let mut verdicts: Vec<bool> = Vec::new();

    for (idx, (name, before_path, after_path)) in tests.iter().enumerate() {
        println!("{}", "═".repeat(64));
        println!("{name}");
        println!("{}", "═".repeat(64));

        print!("  OCR before... ");
        std::io::stdout().flush()?;
        let before = ocr_file(&ocr, before_path)?;
        println!("{} regions", before.len());

        print!("  OCR after...  ");
        std::io::stdout().flush()?;
        let after = ocr_file(&ocr, after_path)?;
        println!("{} regions", after.len());

        let result = run_diff(&before, &after);

        let go = match idx {
            0 => verdict_test1(&result, &before, &after),
            1 => verdict_test2(&result, &before, &after),
            2 => verdict_test3(&result, &before, &after),
            _ => false,
        };
        verdicts.push(go);
        println!();
    }

    println!("{}", "═".repeat(64));
    println!("SUMMARY");
    println!("{}", "─".repeat(64));
    let names = ["Test 1 - Scroll", "Test 2 - Field Filled", "Test 3 - Error Appeared"];
    for (name, &go) in names.iter().zip(verdicts.iter()) {
        println!("  {}  {}", if go { "" } else { "No" }, name);
    }
    println!();
    let all_go = verdicts.iter().all(|&v| v);
    println!(
        "DIFF ENGINE: {}",
        if all_go {
            " GO - all 3 tests passed"
        } else {
            "No NO-GO - review failures above"
        }
    );

    Ok(())
}
