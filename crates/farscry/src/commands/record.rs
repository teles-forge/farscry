use anyhow::{Context, Result};
use crossbeam_channel::bounded;
use farscry_core::vasf::{VasfFrame, VasfWriter};
use farscry_core::StateId;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct RecordOpts {
    pub output: PathBuf,
    pub fps: u32,
    pub daemon: bool,
    pub threshold: u8,
    #[allow(dead_code)]
    pub silent: bool,
}

pub fn record(opts: RecordOpts) -> Result<()> {
    if opts.daemon {
        return daemonize(opts);
    }
    run_capture_loop(opts)
}

fn daemonize(opts: RecordOpts) -> Result<()> {
    let exe = std::env::current_exe()?;
    let child = std::process::Command::new(&exe)
        .args([
            "record",
            "--output",
            &opts.output.to_string_lossy(),
            "--fps",
            &opts.fps.to_string(),
            "--threshold",
            &opts.threshold.to_string(),
            "--silent",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn record daemon")?;
    let pid_path = pid_file_path();
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&pid_path, child.id().to_string()).ok();
    Ok(())
}

fn pid_file_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".farscry")
        .join("sessions")
        .join(".current.pid")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn hamming(a: StateId, b: StateId) -> u8 {
    (a.to_bits() ^ b.to_bits()).count_ones() as u8
}

fn run_capture_loop(opts: RecordOpts) -> Result<()> {
    if let Some(parent) = opts.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let writer = Arc::new(Mutex::new(VasfWriter::create(&opts.output)?));
    let (cap_tx, cap_rx) = bounded::<image::DynamicImage>(3);
    let (ocr_tx, ocr_rx) = bounded::<(image::DynamicImage, StateId)>(1);

    // Shared stop flag — set by SIGINT or SIGTERM handler.
    let stop = Arc::new(AtomicBool::new(false));
    let stop_ctrlc = stop.clone();
    ctrlc::set_handler(move || {
        stop_ctrlc.store(true, Ordering::SeqCst);
    })
    .ok();

    let threshold = opts.threshold;
    let writer_phash = writer.clone();
    // Give phash thread sole ownership of ocr_tx: when it exits, the OCR
    // channel closes and the OCR thread's for-loop terminates naturally.
    let t_phash = thread::spawn(move || phash_thread(cap_rx, ocr_tx, writer_phash, threshold));

    let pipeline = crate::pipeline::get_or_build_pipeline()?;
    let writer_ocr = writer.clone();
    let t_ocr = thread::spawn(move || ocr_thread(ocr_rx, pipeline, writer_ocr));

    let fps = opts.fps.max(1);
    let interval = Duration::from_millis(1000 / fps as u64);

    // Capture loop — sleeps between frames, checks stop flag each iteration.
    loop {
        thread::sleep(interval);
        if stop.load(Ordering::SeqCst) {
            break;
        }
        if let Some(img) = capture_screen() {
            cap_tx.try_send(img).ok();
        }
    }

    // Graceful shutdown sequence:
    // 1. Drop cap_tx → phash_thread's for-loop sees a closed channel and exits,
    //    which in turn drops ocr_tx → ocr_thread's for-loop exits.
    drop(cap_tx);
    // 2. Wait for both threads to finish their in-flight work.
    t_phash.join().ok();
    t_ocr.join().ok();
    // 3. All threads done — no one holds the mutex, finalize is safe.
    if let Ok(mut w) = writer.lock() {
        w.finalize().ok();
    }

    Ok(())
}

fn phash_thread(
    rx: crossbeam_channel::Receiver<image::DynamicImage>,
    ocr_tx: crossbeam_channel::Sender<(image::DynamicImage, StateId)>,
    writer: Arc<Mutex<VasfWriter>>,
    threshold: u8,
) {
    let mut last_hash: Option<StateId> = None;
    for img in rx {
        let hash = farscry_core::phash_image(&img);
        let ts = now_ms();
        let is_new = last_hash
            .map(|prev| hamming(hash, prev) > threshold)
            .unwrap_or(true);
        if is_new {
            ocr_tx.try_send((img, hash)).ok();
            last_hash = Some(hash);
        } else {
            if let Ok(mut w) = writer.lock() {
                w.total_input = w.total_input.saturating_add(1);
                let _ = w.append_frame(&VasfFrame {
                    state_id: hash,
                    timestamp: ts,
                    vasp_data: vec![],
                    delta_data: None,
                });
            }
        }
    }
}

fn ocr_thread(
    rx: crossbeam_channel::Receiver<(image::DynamicImage, StateId)>,
    pipeline: std::sync::Arc<farscry_core::Pipeline>,
    writer: Arc<Mutex<VasfWriter>>,
) {
    for (img, hash) in rx {
        let (w, h) = (img.width(), img.height());
        let ts = now_ms();
        if let Ok(vasp) = pipeline.process(img) {
            let vasp_text =
                farscry_formatter::VaspFormatter::format_vasp(&vasp, "screen", w, h);
            if let Ok(mut wr) = writer.lock() {
                wr.total_input = wr.total_input.saturating_add(1);
                let _ = wr.append_frame(&VasfFrame {
                    state_id: hash,
                    timestamp: ts,
                    vasp_data: vasp_text.into_bytes(),
                    delta_data: None,
                });
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn capture_screen() -> Option<image::DynamicImage> {
    use core_graphics::display::CGDisplay;
    let display = CGDisplay::main();
    let image = display.image()?;
    let data = image.data();
    let bytes = data.bytes().to_vec();
    let width = image.width() as u32;
    let height = image.height() as u32;
    let bytes_per_row = image.bytes_per_row();
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let row_start = (y as usize) * bytes_per_row;
        for x in 0..width {
            let px = row_start + (x as usize) * 4;
            if px + 3 < bytes.len() {
                rgba.push(bytes[px + 2]);
                rgba.push(bytes[px + 1]);
                rgba.push(bytes[px]);
                rgba.push(bytes[px + 3]);
            }
        }
    }
    image::RgbaImage::from_raw(width, height, rgba)
        .map(image::DynamicImage::ImageRgba8)
}

#[cfg(not(target_os = "macos"))]
fn capture_screen() -> Option<image::DynamicImage> {
    use scrap::{Capturer, Display};
    let display = Display::primary().ok()?;
    let mut capturer = Capturer::new(display).ok()?;
    let (w, h) = (capturer.width(), capturer.height());
    loop {
        match capturer.frame() {
            Ok(frame) => {
                let mut rgba = Vec::with_capacity(w * h * 4);
                for chunk in frame.chunks(4) {
                    if chunk.len() == 4 {
                        rgba.push(chunk[2]);
                        rgba.push(chunk[1]);
                        rgba.push(chunk[0]);
                        rgba.push(255);
                    }
                }
                return image::RgbaImage::from_raw(w as u32, h as u32, rgba)
                    .map(image::DynamicImage::ImageRgba8);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(16));
            }
            Err(_) => return None,
        }
    }
}
