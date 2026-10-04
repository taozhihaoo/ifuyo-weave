//! M8（上）文档 Stage 测试（§113-§116）：批量 inspect 聚合事实 + 批量
//! rotate 走 weave-documents 域函数。PDF fixtures = synthetic（lopdf
//! 生成，§7 明确标识）。

use lopdf::dictionary;
use std::path::PathBuf;

use crate::{Pipeline, StageSpec, build_job_plan, execute_plan};
use weave_core::prelude::CancellationToken;

fn ws() -> tempfile::TempDir {
    tempfile::tempdir().expect("ws")
}

/// synthetic PDF（n 页空白 A4）。
fn make_pdf(path: &std::path::Path, pages: usize) {
    let mut doc = lopdf::Document::with_version("1.5");
    let pages_id = doc.add_object(lopdf::dictionary! {});
    let mut kids = Vec::new();
    for _ in 0..pages {
        let content_id = doc.add_object(lopdf::Stream::new(
            lopdf::dictionary! {},
            b"1 0 0 1 0 0 cm\n".to_vec(),
        ));
        let page_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Page",
            "Parent" => lopdf::Object::Reference(pages_id),
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => lopdf::Object::Reference(content_id),
            "Resources" => lopdf::dictionary! {},
        });
        kids.push(lopdf::Object::Reference(page_id));
    }
    {
        let d = doc
            .objects
            .get_mut(&pages_id)
            .expect("pages")
            .as_dict_mut()
            .expect("d");
        d.set("Kids", lopdf::Object::Array(kids));
        d.set("Count", lopdf::Object::Integer(pages as i64));
        d.set("MediaBox", vec![0.into(), 0.into(), 595.into(), 842.into()]);
    }
    let catalog_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Catalog",
        "Pages" => lopdf::Object::Reference(pages_id),
    });
    doc.trailer
        .set("Root", lopdf::Object::Reference(catalog_id));
    doc.save(path).expect("save");
}

#[test]
fn batch_inspect_aggregates_document_facts() {
    // §113：多文档批量 inspect ⇒ per-item stage 日志含格式与页数
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let pdf_a = dir.path().join("a.pdf");
    let pdf_b = dir.path().join("b.pdf");
    make_pdf(&pdf_a, 3);
    make_pdf(&pdf_b, 2);
    let txt = dir.path().join("c.txt");
    std::fs::write(&txt, "plain").expect("w");

    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::DocumentInspect,
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan =
        build_job_plan(vec![pdf_a, pdf_b, txt], pipeline, dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.failed), (3, 0), "{result:?}");
    // inspect 是只读旁路：Export 产物 = 原样字节（3 个输出 + .txt）
    assert_eq!(result.items.len(), 3);
    let pdf_item = result
        .items
        .iter()
        .find(|i| i.source.extension().map(|e| e == "pdf").unwrap_or(false))
        .expect("pdf item");
    let inspect_log = pdf_item
        .stage_results
        .iter()
        .find(|s| s.stage == "document_inspect")
        .expect("inspect stage");
    assert!(inspect_log.ok);
    // §114：capability 不符由 FormatFilter/Filter 表达，inspect 不改语义
    assert!(pdf_item.output.is_some());
}

#[test]
fn batch_rotate_updates_pdf_pages() {
    // §58/§116：Source → PdfRotate 90° → Export；§27 输出校验在域函数内
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let pdf = dir.path().join("doc.pdf");
    make_pdf(&pdf, 4);

    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Filter {
                extensions_in: vec!["pdf".into()],
                max_bytes: None,
            },
            StageSpec::PdfRotate {
                degrees: 90.0,
                pages: String::new(),
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(vec![pdf], pipeline, dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.failed), (1, 0), "{result:?}");
    let out = dest.join("doc.pdf");
    let doc = lopdf::Document::load(&out).expect("reload");
    let pages = doc.get_pages();
    for n in 1..=4 {
        let rot = doc
            .get_object(pages[&n])
            .and_then(|o| o.as_dict())
            .and_then(|d| d.get(b"Rotate"))
            .and_then(|o| o.as_i64())
            .unwrap_or(0);
        assert_eq!(rot, 90, "page {n}");
    }
}

#[test]
fn batch_rotate_rejects_mixed_inputs_via_filter() {
    // §114：capability 不符（.txt 进 rotate 管线）⇒ Filter 拒绝 = Skipped
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let pdf = dir.path().join("d.pdf");
    make_pdf(&pdf, 1);
    let txt = dir.path().join("n.txt");
    std::fs::write(&txt, "x").expect("w");

    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Filter {
                extensions_in: vec!["pdf".into()],
                max_bytes: None,
            },
            StageSpec::PdfRotate {
                degrees: 180.0,
                pages: String::new(),
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(vec![pdf, txt], pipeline, dest, true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.skipped), (1, 1), "{result:?}");
}

#[test]
fn batch_rotate_bad_range_is_item_failure_not_job_failure() {
    // §71 失败隔离：坏页范围 ⇒ 该条 Failed（Validation 类，不可重试）
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let pdf = dir.path().join("x.pdf");
    make_pdf(&pdf, 2);
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::PdfRotate {
                degrees: 90.0,
                pages: "9-12".into(),
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(vec![pdf], pipeline, dest, true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(result.failed, 1);
    let item = &result.items[0];
    assert!(
        item.error
            .as_deref()
            .unwrap_or("")
            .starts_with("Validation:")
    );
    assert!(!item.retryable, "Validation 不可重试");
}

#[allow(unused)]
fn _keep_pathbuf_import(_: PathBuf) {}

// ── M8（下）§172-§173：混合文档批量 + 不支持格式路由 ──

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../weave-documents/tests/fixtures/documents")
        .join(rel)
}

#[test]
fn batch_inspect_mixed_documents_aggregates() {
    // §172：PDF/DOCX/XLSX/PPTX 混合进入 DocumentInspect ⇒ 检测/结果正确
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let inputs = vec![
        fixture("valid/pdf-1page.pdf"),
        fixture("valid/minimal.docx"),
        fixture("valid/sheets.xlsx"),
        fixture("valid/slides.pptx"),
    ];
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::DocumentInspect,
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(inputs, pipeline, dest, true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.failed), (4, 0), "{result:?}");
    // 每条 inspect 日志含对应格式
    let mut seen = std::collections::HashSet::new();
    for item in &result.items {
        let note = item
            .stage_results
            .iter()
            .find(|s| s.stage == "document_inspect")
            .map(|s| s.note.clone())
            .unwrap_or_default();
        seen.insert(note.split(' ').next().unwrap_or("").to_owned());
    }
    for format in ["pdf", "docx", "xlsx", "pptx"] {
        assert!(seen.contains(format), "missing {format} in {seen:?}");
    }
}

#[test]
fn batch_inspect_unsupported_inputs_routed_not_crashed() {
    // §173：ZIP/EXE/图片混入 ⇒ Unsupported 事实（不 crash、不中断其余）
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    // 最小 stored-entry zip（手工字节，无 zip 依赖）
    let zip = dir.path().join("archive.zip");
    let name = b"x.txt";
    let data = b"data";
    let crc = 0u32; // 检测层只看容器/扩展名，不校验内容
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04, 20, 0, 0, 0, 0, 0]);
    body.extend_from_slice(&crc.to_le_bytes());
    body.extend_from_slice(&(data.len() as u32).to_le_bytes());
    body.extend_from_slice(&(data.len() as u32).to_le_bytes());
    body.extend_from_slice(&(name.len() as u16).to_le_bytes());
    body.extend_from_slice(&[0, 0]);
    body.extend_from_slice(name);
    body.extend_from_slice(data);
    body.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0]);
    body.extend_from_slice(&(1u16).to_le_bytes());
    body.extend_from_slice(&(1u16).to_le_bytes());
    body.extend_from_slice(&(name.len() as u16).to_le_bytes());
    body.extend_from_slice(&(name.len() as u16).to_le_bytes());
    body.extend_from_slice(&[0, 0, 0, 0]);
    body.extend_from_slice(&22u32.to_le_bytes());
    std::fs::write(&zip, &body).expect("w");
    let exe = dir.path().join("prog.exe");
    std::fs::write(&exe, b"MZ fake executable").expect("w");
    let pdf = fixture("valid/pdf-1page.pdf");

    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::DocumentInspect,
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(vec![pdf, zip, exe], pipeline, dest, true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(
        result.succeeded, 3,
        "全部通过（inspect 只读旁路，§173 不 crash）"
    );
    let zip_item = result
        .items
        .iter()
        .find(|i| i.source.extension().map(|e| e == "zip").unwrap_or(false))
        .expect("zip item");
    let note = zip_item
        .stage_results
        .iter()
        .find(|s| s.stage == "document_inspect")
        .map(|s| s.note.clone())
        .unwrap_or_default();
    // §173/§7：zip/exe 如实落 unknown/unsupported（绝不冒充支持）
    assert!(
        note.starts_with("unknown") || note.starts_with("unsupported"),
        "{note}"
    );
}

// ── M8（下）§168：批量 inspect 吞吐（10/100/1000）──

#[test]
#[ignore = "perf: release 手动运行"]
fn perf_batch_document_inspection() {
    use std::time::Instant;
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let pdf = fixture("valid/pdf-1page.pdf");
    let docx = fixture("valid/minimal.docx");
    let inputs: Vec<PathBuf> = (0..1000)
        .map(|i| {
            let src = if i % 2 == 0 { &pdf } else { &docx };
            let dst = dir.path().join(format!(
                "doc-{i:04}.{}",
                if i % 2 == 0 { "pdf" } else { "docx" }
            ));
            std::fs::copy(src, &dst).expect("copy");
            dst
        })
        .collect();
    for n in [10usize, 100, 1000] {
        let d = dest.join(format!("n{n}"));
        std::fs::create_dir_all(&d).expect("d");
        let pipeline = Pipeline {
            stages: vec![
                StageSpec::Source,
                StageSpec::DocumentInspect,
                StageSpec::Export {
                    destination_dir: d,
                    overwrite: false,
                },
            ],
        };
        let plan = build_job_plan(
            inputs[..n].to_vec(),
            pipeline,
            dest.join(format!("n{n}")),
            true,
            false,
        )
        .expect("plan");
        let t0 = Instant::now();
        let r = execute_plan(&plan, &CancellationToken::new(), None);
        let dt = t0.elapsed();
        assert_eq!(r.succeeded, n as u64);
        eprintln!(
            "PERF batch document inspect {n} docs: {dt:?} ({:.3} ms/doc)",
            dt.as_millis() as f64 / n as f64
        );
    }
}
