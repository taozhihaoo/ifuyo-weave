//! 批量处理测试（M6 下 §79–§90/§170-§171/§201-§203）：失败隔离/取消/
//! 输入快照防自回流/确定性命名。

use super::*;
use image::DynamicImage;

fn make_png(path: &Path, r: u8) {
    let mut img = image::RgbaImage::new(8, 8);
    for p in img.pixels_mut() {
        *p = image::Rgba([r, r, r, 255]);
    }
    let bytes = encode_image(&DynamicImage::ImageRgba8(img), ImageFormat::Png, None).expect("enc");
    std::fs::write(path, bytes).expect("write");
}

fn batch_plan(inputs: Vec<PathBuf>, dest: &Path) -> ImageBatchPlan {
    ImageBatchPlan {
        inputs,
        destination_dir: dest.to_path_buf(),
        target_format: ImageFormat::WebP,
        quality: None,
        overwrite_existing: false,
    }
}

#[test]
fn batch_success_all_files_with_deterministic_names() {
    let ws = tempfile::tempdir().expect("ws");
    let src = ws.path().join("src");
    let dest = ws.path().join("dest");
    std::fs::create_dir_all(&src).expect("mkdir");
    std::fs::create_dir_all(&dest).expect("mkdir");
    make_png(&src.join("a.png"), 10);
    make_png(&src.join("b.png"), 20);

    let inputs = vec![src.join("a.png"), src.join("b.png")];
    let result = run_batch(
        &batch_plan(inputs.clone(), &dest),
        &ImageLimits::default(),
        &CancellationToken::new(),
    );

    assert_eq!(
        (result.succeeded, result.failed, result.cancelled),
        (2, 0, 0),
        "{result:?}"
    );
    assert!(dest.join("a.webp").exists());
    assert!(dest.join("b.webp").exists());
    assert!(result.output_bytes > 0);
}

#[test]
fn batch_failure_isolation_and_skip_existing() {
    // §84/§86：损坏文件 Failed 不牵连；已存在目标（overwrite=false）⇒ 错误
    let ws = tempfile::tempdir().expect("ws");
    let src = ws.path().join("src");
    let dest = ws.path().join("dest");
    std::fs::create_dir_all(&src).expect("mkdir");
    std::fs::create_dir_all(&dest).expect("mkdir");
    make_png(&src.join("ok.png"), 10);
    std::fs::write(src.join("broken.png"), b"\x89PNG broken").expect("corrupt");
    make_png(&src.join("collide.png"), 30);
    std::fs::write(dest.join("collide.webp"), b"existing").expect("seed dest");

    let inputs = vec![
        src.join("ok.png"),
        src.join("broken.png"),
        src.join("collide.png"),
    ];
    let result = run_batch(
        &batch_plan(inputs.clone(), &dest),
        &ImageLimits::default(),
        &CancellationToken::new(),
    );

    assert_eq!(result.succeeded, 1);
    assert_eq!(result.failed, 2, "{result:?}");
    // 失败隔离：ok 仍成功
    assert!(dest.join("ok.webp").exists());
}

#[test]
fn input_snapshot_excludes_target_extension_outputs() {
    // §201：同目录输出 webp 不得回流进入任务（快照先于处理）
    let ws = tempfile::tempdir().expect("ws");
    let dir = ws.path().to_path_buf();
    make_png(&dir.join("a.png"), 10);
    // 预置一个已存在的 webp（模拟上一轮输出）
    std::fs::write(dir.join("old.webp"), b"old").expect("seed");

    let inputs = vec![dir.join("a.png")];
    let result = run_batch(
        &batch_plan(inputs.clone(), &dir),
        &ImageLimits::default(),
        &CancellationToken::new(),
    );
    assert_eq!(result.succeeded, 1);
    // 快照仅含 a.png，未把 old.webp 当输入
    assert!(!dir.join("old.webp").ends_with("a.webp"));
}

#[test]
fn cancellation_marks_remaining_as_cancelled() {
    // §87-§89：处理前取消 ⇒ 剩余 Cancelled，已处理数如实
    let ws = tempfile::tempdir().expect("ws");
    let src = ws.path().join("src");
    let dest = ws.path().join("dest");
    std::fs::create_dir_all(&src).expect("mkdir");
    std::fs::create_dir_all(&dest).expect("mkdir");
    for i in 0..4 {
        make_png(&src.join(format!("f{i}.png")), i * 10);
    }
    let inputs: Vec<PathBuf> = (0..4).map(|i| src.join(format!("f{i}.png"))).collect();

    let cancel = CancellationToken::new();
    // 先取消 ⇒ 全部 Cancelled
    cancel.cancel();
    let result = run_batch(
        &batch_plan(inputs.clone(), &dest),
        &ImageLimits::default(),
        &cancel,
    );
    assert_eq!((result.succeeded, result.cancelled), (0, 4), "{result:?}");
    assert!(
        result
            .results
            .iter()
            .all(|r| r.status == BatchFileStatus::Cancelled)
    );
}

#[test]
fn unsupported_extension_filtered_from_snapshot() {
    // §128-§130：pdf/txt 不进批量（Mixed Selection 由上层显式报告）
    let ws = tempfile::tempdir().expect("ws");
    let dir = ws.path().to_path_buf();
    make_png(&dir.join("a.png"), 10);
    std::fs::write(dir.join("doc.pdf"), b"%PDF-1.4").expect("write");
    std::fs::write(dir.join("note.txt"), "hello").expect("write");

    let inputs = vec![dir.join("a.png"), dir.join("doc.pdf"), dir.join("note.txt")];
    let result = run_batch(
        &batch_plan(inputs.clone(), &dir),
        &ImageLimits::default(),
        &CancellationToken::new(),
    );
    assert_eq!(result.succeeded, 1);
}
