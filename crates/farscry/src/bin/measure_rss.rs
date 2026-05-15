
use std::time::{Duration, Instant};

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

fn section(title: &str) {
    eprintln!("\n=== {} ===", title);
}


#[cfg(target_os = "macos")]
fn capture_main_display() -> Option<image::DynamicImage> {
    use core_graphics::display::CGDisplay;

    let cg_img = CGDisplay::main().image()?;
    let width = cg_img.width() as u32;
    let height = cg_img.height() as u32;
    let bpr = cg_img.bytes_per_row();

    let bytes = {
        let data = cg_img.data();
        data.bytes().to_vec()
    };
    drop(cg_img);

    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let row = (y as usize) * bpr;
        for x in 0..width {
            let px = row + (x as usize) * 4;
            if px + 3 < bytes.len() {
                rgba.push(bytes[px + 2]);
                rgba.push(bytes[px + 1]);
                rgba.push(bytes[px]);
                rgba.push(bytes[px + 3]);
            }
        }
    }
    drop(bytes);

    image::RgbaImage::from_raw(width, height, rgba).map(image::DynamicImage::ImageRgba8)
}

#[cfg(not(target_os = "macos"))]
fn capture_main_display() -> Option<image::DynamicImage> {
    None
}


fn measurement_1_phash_only() {
    section("MEASUREMENT 1 — pHash only, no model loading");

    let baseline = probe("process baseline");

    let mut peaks: Vec<u64> = Vec::new();
    let mut steadies: Vec<u64> = Vec::new();
    let mut deltas: Vec<u64> = Vec::new();

    for i in 0..10 {
        let before = rss_mb();

        let img = match capture_main_display() {
            Some(img) => img,
            None => {
                eprintln!("[M1] capture failed (no screen capture permission?)");
                return;
            }
        };
        let (w, h) = (img.width(), img.height());
        let after_capture = rss_mb();

        let _hash = farscry_core::phash_image(&img);
        let after_phash = rss_mb();

        drop(img);
        let after_drop = rss_mb();

        let peak = after_capture.saturating_sub(before);
        let steady = after_drop.saturating_sub(before);
        let retained = after_drop.saturating_sub(baseline);

        peaks.push(peak);
        steadies.push(steady);
        deltas.push(retained);

        eprintln!(
            "[M1] frame {:02} {}x{} | before={}MB capture={}MB phash={}MB drop={}MB | peak_delta=+{}MB steady={}MB retained_vs_baseline=+{}MB",
            i, w, h, before, after_capture, after_phash, after_drop, peak, steady, retained
        );

        std::thread::sleep(Duration::from_millis(500));
    }

    let avg_peak = peaks.iter().sum::<u64>() / peaks.len().max(1) as u64;
    let avg_steady = steadies.iter().sum::<u64>() / steadies.len().max(1) as u64;
    let max_retained = *deltas.iter().max().unwrap_or(&0);

    eprintln!("[M1] RESULT: avg_peak_during_capture=+{}MB  avg_steady_after_drop=+{}MB  max_retained_vs_baseline=+{}MB",
        avg_peak, avg_steady, max_retained);
    eprintln!("[M1] baseline={}MB  max_steady={}MB", baseline, baseline + max_retained);
}


#[cfg(target_os = "macos")]
fn measurement_2_cgwindow_cost() {
    use core_graphics::window::{
        copy_window_info, create_image, kCGNullWindowID, kCGWindowImageBoundsIgnoreFraming,
        kCGWindowListExcludeDesktopElements, kCGWindowListOptionAll,
        kCGWindowListOptionIncludingWindow, kCGWindowNumber, kCGWindowOwnerPID,
    };
    use std::os::raw::c_void;

    section("MEASUREMENT 2 — CGWindowListCreateImage: peak vs steady vs CGDisplay");

    #[allow(improper_ctypes)]
    extern "C" {
        fn CFDictionaryGetValue(dict: *const c_void, key: *const c_void) -> *const c_void;
        fn CFNumberGetValue(num: *const c_void, ty: i32, val: *mut c_void) -> bool;
    }
    const CF_NUMBER_SINT32: i32 = 3;

    let self_pid = std::process::id();
    let win_id: Option<u32> = (|| {
        let wins = copy_window_info(
            kCGWindowListOptionAll | kCGWindowListExcludeDesktopElements,
            kCGNullWindowID,
        )?;
        for raw in wins.get_all_values() {
            unsafe {
                let pid_cf =
                    CFDictionaryGetValue(raw, kCGWindowOwnerPID as *const c_void);
                if pid_cf.is_null() {
                    continue;
                }
                let mut pid: i32 = 0;
                if !CFNumberGetValue(
                    pid_cf,
                    CF_NUMBER_SINT32,
                    &mut pid as *mut i32 as *mut c_void,
                ) {
                    continue;
                }
                if pid as u32 != self_pid {
                    continue;
                }
                let wid_cf =
                    CFDictionaryGetValue(raw, kCGWindowNumber as *const c_void);
                if wid_cf.is_null() {
                    continue;
                }
                let mut wid: i32 = 0;
                if CFNumberGetValue(
                    wid_cf,
                    CF_NUMBER_SINT32,
                    &mut wid as *mut i32 as *mut c_void,
                ) {
                    return Some(wid as u32);
                }
            }
        }
        None
    })();

    let baseline = probe("baseline");

    eprintln!("[M2-A] --- CGDisplay full-screen capture ---");
    for i in 0..5 {
        let before = rss_mb();
        let img = capture_main_display();
        let after_capture = rss_mb();
        let dims = img.as_ref().map(|i| (i.width(), i.height()));
        drop(img);
        let after_drop = rss_mb();
        eprintln!(
            "[M2-A] frame {:02} {:?} | before={}MB capture={}MB drop={}MB | peak=+{}MB retained=+{}MB",
            i, dims, before, after_capture, after_drop,
            after_capture.saturating_sub(before),
            after_drop.saturating_sub(before)
        );
        std::thread::sleep(Duration::from_millis(200));
    }

    if let Some(wid) = win_id {
        eprintln!("[M2-B] --- CGWindowListCreateImage wid={} ---", wid);
        for i in 0..5 {
            let before = rss_mb();
            let bounds = unsafe { core_graphics::display::CGRectNull };
            let cg_img = create_image(
                bounds,
                kCGWindowListOptionIncludingWindow,
                wid,
                kCGWindowImageBoundsIgnoreFraming,
            );
            let after_cg = rss_mb();
            let dims = cg_img.as_ref().map(|img| (img.width(), img.height()));
            drop(cg_img);
            let after_drop_cg = rss_mb();
            eprintln!(
                "[M2-B] frame {:02} {:?} | before={}MB after_cg={}MB drop_cg={}MB | peak=+{}MB retained=+{}MB",
                i, dims, before, after_cg, after_drop_cg,
                after_cg.saturating_sub(before),
                after_drop_cg.saturating_sub(before)
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    } else {
        eprintln!("[M2-B] no own window found (terminal process has no GUI window — expected)");
    }

    eprintln!("[M2-C] --- CGWindowListCopyWindowInfo enumeration cost ---");
    let before = rss_mb();
    let wins = copy_window_info(
        kCGWindowListOptionAll | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    );
    let after_enum = rss_mb();
    let count = wins.as_ref().map(|w| w.len()).unwrap_or(0);
    drop(wins);
    let after_drop = rss_mb();
    eprintln!(
        "[M2-C] windows={} | before={}MB after={}MB drop={}MB | peak=+{}MB retained=+{}MB",
        count, before, after_enum, after_drop,
        after_enum.saturating_sub(before),
        after_drop.saturating_sub(before)
    );

    eprintln!("[M2] RESULT: baseline={}MB", baseline);
}

#[cfg(not(target_os = "macos"))]
fn measurement_2_cgwindow_cost() {
    eprintln!("[M2] macOS only — skipped");
}


fn measurement_3_ipc_overhead() {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    section("MEASUREMENT 3 — Unix socket IPC RSS overhead");

    let baseline = probe("baseline");

    let sock_path = std::env::temp_dir().join("farscry_measure.sock");
    let _ = std::fs::remove_file(&sock_path);

    let listener = UnixListener::bind(&sock_path).expect("bind");
    probe("after UnixListener::bind");

    let _sock_path2 = sock_path.clone();
    let t = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).unwrap_or(0);
        let _ = stream.write_all(&buf[..n]);
    });
    probe("after listener thread spawned");

    let mut client = UnixStream::connect(&sock_path).expect("connect");
    probe("after client connect");

    client.write_all(b"ping").unwrap();
    let mut resp = [0u8; 4];
    client.read_exact(&mut resp).unwrap();
    probe("after ping/pong roundtrip");

    drop(client);
    t.join().unwrap();
    probe("after socket teardown");

    let _ = std::fs::remove_file(&sock_path);
    let final_rss = rss_mb();
    eprintln!("[M3] RESULT: baseline={}MB  final={}MB  net_overhead=+{}MB",
        baseline, final_rss, final_rss.saturating_sub(baseline));
}


#[cfg(all(target_os = "macos", target_arch = "aarch64", feature = "coreml"))]
fn measurement_4_coreml() {
    section("MEASUREMENT 4 — CoreML model RSS (mmap / ANE)");

    let baseline = probe("baseline");

    let models_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".farscry")
        .join("models");

    if !models_dir.join("coreml").exists() {
        eprintln!("[M4] no coreml models at {} — skipped", models_dir.display());
        return;
    }

    probe("before model load");
    let engine = farscry_ocr::build_ocr_engine(&models_dir);
    let after_load = probe("after CoreML model load");
    eprintln!("[M4] model load delta: +{}MB", after_load.saturating_sub(baseline));

    if let Ok(engine) = engine {
        let dummy = image::DynamicImage::ImageRgba8(image::RgbaImage::new(640, 480));
        let _ = farscry_core::OcrEngine::extract(&engine, &dummy);
        let after_infer = probe("after first CoreML inference");
        eprintln!("[M4] inference delta: +{}MB", after_infer.saturating_sub(baseline));

        for i in 1..6 {
            let _ = farscry_core::OcrEngine::extract(&engine, &dummy);
            let mb = rss_mb();
            eprintln!("[M4] inference {} — {}MB (delta=+{}MB)", i, mb, mb.saturating_sub(baseline));
        }

        drop(engine);
        let after_drop = probe("after drop CoreML engine");
        eprintln!("[M4] RESULT: baseline={}MB  peak={}MB  after_drop={}MB",
            baseline, after_infer, after_drop);
    }
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64", feature = "coreml")))]
fn measurement_4_coreml() {
    section("MEASUREMENT 4 — CoreML model");
    eprintln!("[M4] not enabled — rebuild with: cargo run --release --features coreml --bin measure_rss");
    eprintln!("[M4] running ORT measurement instead:");
    measurement_4_ort();
}

fn measurement_4_ort() {
    let baseline = probe("ORT baseline");

    let models_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".farscry")
        .join("models");

    probe("before ORT model load");
    let engine = farscry_ocr::build_ocr_engine(&models_dir);
    let after_load = probe("after ORT model load");
    eprintln!("[M4-ORT] model load delta: +{}MB", after_load.saturating_sub(baseline));

    if let Ok(engine) = engine {
        let dummy = image::DynamicImage::ImageRgba8(image::RgbaImage::new(640, 480));
        let _ = farscry_core::OcrEngine::extract(&engine, &dummy);
        let after_infer = probe("after first ORT inference (640x480)");
        eprintln!("[M4-ORT] inference delta vs baseline: +{}MB", after_infer.saturating_sub(baseline));

        let screen = image::DynamicImage::ImageRgba8(image::RgbaImage::new(3600, 2338));
        let _ = farscry_core::OcrEngine::extract(&engine, &screen);
        let after_large = probe("after ORT inference (3600x2338 Retina)");
        eprintln!("[M4-ORT] large image delta: +{}MB", after_large.saturating_sub(baseline));
        drop(screen);

        for i in 1..4 {
            let s = image::DynamicImage::ImageRgba8(image::RgbaImage::new(3600, 2338));
            let _ = farscry_core::OcrEngine::extract(&engine, &s);
            drop(s);
            let mb = rss_mb();
            eprintln!("[M4-ORT] repeat inference {} — {}MB", i, mb);
        }

        drop(engine);
        let after_drop = probe("after drop ORT engine");
        eprintln!("[M4-ORT] RESULT: baseline={}MB  after_large_infer={}MB  after_drop={}MB",
            baseline, after_large, after_drop);
    }
}


fn measurement_daemon_simulation() {
    section("MEASUREMENT 5 — Single-daemon steady-state simulation");
    eprintln!("[M5] Simulating: capture + phash + drop, 10 cycles, no model");

    let baseline = probe("simulation baseline");
    let start = Instant::now();
    let mut max_rss: u64 = 0;
    let mut min_rss_after_drop: u64 = u64::MAX;

    for i in 0..10 {
        let img = match capture_main_display() {
            Some(img) => img,
            None => {
                eprintln!("[M5] capture failed");
                return;
            }
        };
        let after_capture = rss_mb();
        max_rss = max_rss.max(after_capture);

        let _hash = farscry_core::phash_image(&img);
        drop(img);

        let after_drop = rss_mb();
        min_rss_after_drop = min_rss_after_drop.min(after_drop);

        if i == 0 || i == 9 {
            eprintln!("[M5] frame {:02} | capture={}MB  drop={}MB", i, after_capture, after_drop);
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    let elapsed = start.elapsed().as_secs_f32();
    let final_rss = rss_mb();
    eprintln!("[M5] RESULT: baseline={}MB  max_peak={}MB  min_after_drop={}MB  final={}MB  elapsed={:.1}s",
        baseline, max_rss, min_rss_after_drop, final_rss, elapsed);
    eprintln!("[M5] steady-state overhead above baseline: +{}MB",
        final_rss.saturating_sub(baseline));
    eprintln!("[M5] peak overhead above baseline: +{}MB",
        max_rss.saturating_sub(baseline));
    eprintln!("[M5] < 7MB target feasible: {}",
        if final_rss < 7 { "YES — already there" }
        else if final_rss < 20 { "LIKELY with tuning" }
        else { "NO without architecture changes" }
    );
}


#[cfg(target_os = "macos")]
fn measurement_6_cgcontext_direct() {
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_graphics::display::CGDisplay;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};

    section("MEASUREMENT 6 — CGContext direct 32×32 render (proposed zero-copy pHash)");
    let baseline = probe("baseline");

    for i in 0..10 {
        let before = rss_mb();

        let cg_img = match CGDisplay::main().image() {
            Some(img) => img,
            None => { eprintln!("[M6] capture failed"); return; }
        };
        let (img_w, img_h) = (cg_img.width(), cg_img.height());
        let after_capture = rss_mb();

        let mut pixels = vec![0u8; 32 * 32];
        let cs = CGColorSpace::create_device_gray();
        {
            let ctx = CGContext::create_bitmap_context(
                Some(pixels.as_mut_ptr() as *mut std::os::raw::c_void),
                32, 32, 8, 32, &cs,
                0u32,
            );
            let r = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(32.0, 32.0));
            ctx.draw_image(r, &cg_img);
        }
        let after_draw = rss_mb();

        drop(cg_img);
        drop(cs);
        let after_drop_cg = rss_mb();

        let luma_img = image::GrayImage::from_raw(32, 32, pixels)
            .map(image::DynamicImage::ImageLuma8)
            .expect("32x32 luma image");
        let _hash = farscry_core::phash_image(&luma_img);
        drop(luma_img);
        let after_phash = rss_mb();

        eprintln!(
            "[M6] frame {:02} {}x{} | before={}MB capture={}MB draw32={}MB drop_cg={}MB phash={}MB | peak=+{}MB steady=+{}MB",
            i, img_w, img_h, before, after_capture, after_draw, after_drop_cg, after_phash,
            after_capture.saturating_sub(before),
            after_phash.saturating_sub(before),
        );
        std::thread::sleep(Duration::from_millis(300));
    }

    let final_rss = rss_mb();
    eprintln!("[M6] RESULT: baseline={}MB  final={}MB  delta=+{}MB",
        baseline, final_rss, final_rss.saturating_sub(baseline));
    eprintln!("[M6] < 7MB steady achievable: {}",
        if final_rss.saturating_sub(baseline) < 7 { "YES" } else { "NO — need madvise or jemalloc" });
}

#[cfg(not(target_os = "macos"))]
fn measurement_6_cgcontext_direct() {
    eprintln!("[M6] macOS only — skipped");
}

fn main() {
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║   farscry daemon RSS measurement suite   ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!("pid={}", std::process::id());
    probe("absolute baseline (main entry)");

    measurement_1_phash_only();
    measurement_2_cgwindow_cost();
    measurement_3_ipc_overhead();
    measurement_4_coreml();
    measurement_daemon_simulation();
    measurement_6_cgcontext_direct();

    eprintln!("\n=== SUMMARY ===");
    probe("final RSS");
}
