// M6 clean-process measurement: CGContext 32×32 direct render, no model, no prior contamination.

use std::time::Duration;

fn rss_mb() -> u64 {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse::<u64>()
        .unwrap_or(0)
        / 1024
}

fn probe(label: &str) -> u64 {
    let mb = rss_mb();
    eprintln!("[rss] {label}: {mb}MB");
    mb
}

#[cfg(target_os = "macos")]
fn main() {
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_graphics::display::CGDisplay;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};

    eprintln!("=== M6 CLEAN: CGContext 32×32 direct render, no model ===");
    let baseline = probe("process baseline");

    let mut peak_rss: Vec<u64> = Vec::new();
    let mut steady_rss: Vec<u64> = Vec::new();

    for i in 0..20 {
        let before = rss_mb();

        let cg_img = match CGDisplay::main().image() {
            Some(img) => img,
            None => { eprintln!("capture failed"); break; }
        };
        let (img_w, img_h) = (cg_img.width(), cg_img.height());
        let after_capture = rss_mb();

        let mut pixels = vec![0u8; 32 * 32];
        let cs = CGColorSpace::create_device_gray();
        {
            let ctx = CGContext::create_bitmap_context(
                Some(pixels.as_mut_ptr() as *mut std::os::raw::c_void),
                32, 32, 8, 32, &cs, 0u32,
            );
            let r = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(32.0, 32.0));
            ctx.draw_image(r, &cg_img);
        }
        let after_draw = rss_mb();

        drop(cg_img);
        drop(cs);
        let after_drop = rss_mb();

        let luma = image::GrayImage::from_raw(32, 32, pixels)
            .map(image::DynamicImage::ImageLuma8)
            .expect("luma");
        let _hash = farscry_core::phash_image(&luma);
        drop(luma);
        let after_phash = rss_mb();

        peak_rss.push(after_draw);
        steady_rss.push(after_phash);

        eprintln!(
            "[M6c] frame {:02} {}x{} | before={} capture={} draw={} drop_cg={} phash={} | peak_delta=+{} steady_delta=+{}",
            i, img_w, img_h,
            before, after_capture, after_draw, after_drop, after_phash,
            after_draw.saturating_sub(before),
            after_phash.saturating_sub(before),
        );

        std::thread::sleep(Duration::from_millis(1000));
    }

    let final_rss = rss_mb();
    let max_peak = *peak_rss.iter().max().unwrap_or(&0);
    let last_steady = *steady_rss.last().unwrap_or(&0);
    let min_steady = *steady_rss.iter().min().unwrap_or(&0);

    eprintln!("\n=== CLEAN M6 RESULTS ===");
    eprintln!("baseline:        {}MB", baseline);
    eprintln!("max peak (draw): {}MB (+{}MB above baseline)", max_peak, max_peak.saturating_sub(baseline));
    eprintln!("min steady:      {}MB (+{}MB above baseline)", min_steady, min_steady.saturating_sub(baseline));
    eprintln!("last steady:     {}MB (+{}MB above baseline)", last_steady, last_steady.saturating_sub(baseline));
    eprintln!("final:           {}MB", final_rss);
    eprintln!("growth over 20 frames: +{}MB", last_steady.saturating_sub(min_steady));
    eprintln!("< 7MB target: {}",
        if last_steady.saturating_sub(baseline) < 7 { "ACHIEVED" }
        else if last_steady.saturating_sub(baseline) < 20 { "CLOSE — need madvise" }
        else { "NOT achieved" }
    );
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("macOS only");
}
