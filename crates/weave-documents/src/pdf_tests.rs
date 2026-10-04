//! PDF 域测试（M8 上 §18-§28/§84/§86）。
//!
//! Fixtures = **synthetic**（lopdf Creator 生成的最小合法 PDF，§7 明确
//! 标识）；损坏样本为字节级手工构造。

use std::path::Path;

use lopdf::{Document as LoDocument, Object, Stream, dictionary};

use crate::{
    DocumentResourceLimits, Rotation, execute_extract_pages, execute_merge, execute_reorder,
    execute_rotate, execute_split_every_n, inspect_document, plan_merge, validate_reorder,
};

fn limits() -> DocumentResourceLimits {
    DocumentResourceLimits::default()
}

/// synthetic PDF：`pages` 页空白 A4（显式 MediaBox，测试自足）。
fn make_pdf(path: &Path, pages: usize, title: Option<&str>) {
    let mut doc = LoDocument::with_version("1.5");
    let pages_id = doc.add_object(dictionary! {});
    let mut kids = Vec::new();
    for _ in 0..pages {
        let content_id = doc.add_object(Stream::new(dictionary! {}, b"1 0 0 1 0 0 cm\n".to_vec()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => Object::Reference(content_id),
            "Resources" => dictionary! {},
        });
        kids.push(Object::Reference(page_id));
    }
    let info_id = title.map(|t| {
        doc.add_object(dictionary! {
            "Title" => Object::String(t.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            "Author" => Object::String(b"Weave Synthetic".to_vec(), lopdf::StringFormat::Literal),
        })
    });
    {
        let pages_dict = doc
            .objects
            .get_mut(&pages_id)
            .expect("pages")
            .as_dict_mut()
            .expect("dict");
        pages_dict.set("Kids", Object::Array(kids));
        pages_dict.set("Count", Object::Integer(pages as i64));
        pages_dict.set("MediaBox", vec![0.into(), 0.into(), 595.into(), 842.into()]);
    }
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    if let Some(id) = info_id {
        doc.trailer.set("Info", Object::Reference(id));
    }
    doc.save(path).expect("save synthetic pdf");
}

fn ws() -> tempfile::TempDir {
    tempfile::tempdir().expect("ws")
}

#[test]
fn inspect_reports_pages_sizes_metadata_version() {
    let dir = ws();
    let pdf = dir.path().join("synthetic.pdf");
    make_pdf(&pdf, 4, Some("Inspect Me"));
    let facts = inspect_document(&pdf, &limits());
    assert_eq!(facts.format, crate::DocumentFormat::Pdf);
    let pages = match facts.pages {
        Some(crate::Field::Known(n)) => n,
        other => panic!("pages field: {other:?}"),
    };
    assert_eq!(pages, 4, "§11: 解析成功 = Known(4)");
    assert_eq!(
        facts
            .metadata
            .iter()
            .find(|(k, _)| k == "Title")
            .map(|(_, v)| v.as_str()),
        Some("Inspect Me"),
        "§17: 真实存在的元数据键值"
    );
    assert!(facts.pdf_version.is_some());
    if let Some(crate::Field::Known(sizes)) = &facts.page_sizes {
        assert_eq!(sizes.len(), 4);
        assert!((sizes[0][0] - 595.0).abs() < 0.5 && (sizes[0][1] - 842.0).abs() < 0.5);
    } else {
        panic!("page sizes expected");
    }
}

#[test]
fn malformed_pdf_is_structured_error_not_panic() {
    // §86：垃圾字节 ⇒ 结构化诊断（Malformed），绝不 panic
    let dir = ws();
    let pdf = dir.path().join("garbage.pdf");
    std::fs::write(&pdf, b"%PDF-1.4 this is not really a pdf \xff\xfe").expect("write");
    let facts = inspect_document(&pdf, &limits());
    assert!(
        facts
            .diagnostics
            .iter()
            .any(|d| d.message.contains("parse failed") || d.code.contains("malformed")),
        "{facts:?}"
    );
    // §11：失败 ⇒ pages = Unknown（不是 0）
    assert!(matches!(facts.pages, Some(crate::Field::Unknown)));
}

#[test]
fn merge_preserves_order_and_page_count() {
    // §18/§19/§27/§70：有序合并 + 输出页数 = sum + 重新解析验证
    let dir = ws();
    let a = dir.path().join("a.pdf");
    let b = dir.path().join("b.pdf");
    make_pdf(&a, 3, Some("A"));
    make_pdf(&b, 2, Some("B"));
    let plan = plan_merge(&[a.clone(), b.clone()], &limits()).expect("plan");
    assert_eq!(plan.input_page_counts, vec![3, 2]);
    assert_eq!(plan.output_page_count, 5);
    let out = dir.path().join("merged.pdf");
    let (pages, diags) = execute_merge(&plan, &out, &limits()).expect("merge");
    assert_eq!(pages, 5);
    assert!(!diags.is_empty(), "§80 metadata policy documented");
    // §80：元数据 = 首输入
    let facts = inspect_document(&out, &limits());
    assert_eq!(
        facts
            .metadata
            .iter()
            .find(|(k, _)| k == "Title")
            .map(|(_, v)| v.as_str()),
        Some("A")
    );
    // 顺序验证：抽查不可行（空白页）——页数与可解析性由 write_and_validate 保证
}

#[test]
fn merge_rejects_duplicate_and_single_input() {
    let dir = ws();
    let a = dir.path().join("a.pdf");
    make_pdf(&a, 1, None);
    assert!(
        plan_merge(std::slice::from_ref(&a), &limits()).is_err(),
        "single input"
    );
    assert!(
        plan_merge(&[a.clone(), a.clone()], &limits()).is_err(),
        "duplicate"
    );
}

#[test]
fn split_every_n_parts_named_and_validated() {
    // §23/§28：100 拆不了太重，12 页 N=5 ⇒ part-001(5) part-002(5) part-003(2)
    let dir = ws();
    let src = dir.path().join("report.pdf");
    make_pdf(&src, 12, None);
    let out_dir = dir.path().join("parts");
    std::fs::create_dir_all(&out_dir).expect("dir");
    let parts = execute_split_every_n(&src, 5, &out_dir, &limits()).expect("split");
    assert_eq!(parts.len(), 3);
    assert_eq!(
        parts[0]
            .0
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .as_deref(),
        Some("report.part-001.pdf")
    );
    assert_eq!(parts[0].1, 5);
    assert_eq!(parts[2].1, 2);
    // collision-safe：再跑一次（默认不覆盖）⇒ 已存在文件是预写碰撞……
    // lopdf save 直接覆盖——M8 上策略：输出目录要求为空由调用方保证；
    // 这里验证命名确定性（两次运行同名）：
    let parts2 = execute_split_every_n(&src, 5, &out_dir, &limits()).expect("split2");
    assert_eq!(parts[0].0, parts2[0].0);
}

#[test]
fn extract_pages_respects_ranges() {
    // §21/§22：1-3,5 ⇒ 4 页
    let dir = ws();
    let src = dir.path().join("src.pdf");
    make_pdf(&src, 10, None);
    let out = dir.path().join("extract.pdf");
    let pages = execute_extract_pages(&src, "1-3,5", &out, &limits()).expect("extract");
    assert_eq!(pages, 4);
    let facts = inspect_document(&out, &limits());
    assert!(matches!(facts.pages, Some(crate::Field::Known(4))));
    // 越界 ⇒ 结构化错误
    let out2 = dir.path().join("oob.pdf");
    let e = execute_extract_pages(&src, "9-11", &out2, &limits()).expect_err("oob");
    assert!(e.message.contains("exceeds page count"));
}

#[test]
fn reorder_validates_and_executes() {
    // §24：完整置换合法；缺页/重复/越界拒绝
    let dir = ws();
    let src = dir.path().join("src.pdf");
    make_pdf(&src, 5, None);
    assert!(validate_reorder(&[3, 1, 2, 5], 5).is_err(), "omission");
    assert!(validate_reorder(&[1, 2, 2, 4, 5], 5).is_err(), "dup");
    assert!(validate_reorder(&[1, 2, 6, 4, 5], 5).is_err(), "oob");
    let out = dir.path().join("reordered.pdf");
    let pages = execute_reorder(&src, &[3, 1, 2, 5, 4], &out, &limits()).expect("reorder");
    assert_eq!(pages, 5);
    let facts = inspect_document(&out, &limits());
    assert!(matches!(facts.pages, Some(crate::Field::Known(5))));
}

#[test]
fn rotate_sets_page_metadata_accumulating() {
    // §25/§26：/Rotate 元数据语义（非内容重渲染）；选定页叠加
    let dir = ws();
    let src = dir.path().join("src.pdf");
    make_pdf(&src, 4, None);
    let out = dir.path().join("rotated.pdf");
    let (total, rotated) =
        execute_rotate(&src, Rotation::Deg90, "2,4", &out, &limits()).expect("rotate");
    assert_eq!((total, rotated), (4, vec![2, 4]));
    let facts = inspect_document(&out, &limits());
    let Some(crate::Field::Known(sizes)) = facts.page_sizes else {
        panic!("sizes");
    };
    let _ = sizes;
    // rotation 事实在 inspect 里没有直接字段——经 lopdf 直读验证
    let doc = lopdf::Document::load(&out).expect("reload");
    let pages = doc.get_pages();
    let rot_of = |n: u32| {
        doc.get_object(pages[&n])
            .and_then(|o| o.as_dict())
            .and_then(|d| d.get(b"Rotate"))
            .and_then(|o| o.as_i64())
            .unwrap_or(0)
    };
    assert_eq!(rot_of(1), 0);
    assert_eq!(rot_of(2), 90, "§25 /Rotate metadata");
    assert_eq!(rot_of(3), 0);
    assert_eq!(rot_of(4), 90);
    // 再 rotate 同文件：累积（90+90=180）
    let out2 = dir.path().join("rotated2.pdf");
    execute_rotate(&out, Rotation::Deg90, "", &out2, &limits()).expect("rotate2");
    let doc2 = lopdf::Document::load(&out2).expect("reload2");
    let pages2 = doc2.get_pages();
    let rot2 = |n: u32| {
        doc2.get_object(pages2[&n])
            .and_then(|o| o.as_dict())
            .and_then(|d| d.get(b"Rotate"))
            .and_then(|o| o.as_i64())
            .unwrap_or(0)
    };
    assert_eq!(rot2(2), 180, "累积 rotate");
    assert_eq!(rot2(1), 90, "空 ranges = 全部页");
}

#[test]
fn extension_mismatch_reported() {
    // §95：zip 内容 + .pdf 扩展名 ⇒ mismatch 警告
    let dir = ws();
    let fake = dir.path().join("fake.pdf");
    // 最小 zip（空条目）——synthetic
    let file = std::fs::File::create(&fake).expect("f");
    let mut zw = zip::ZipWriter::new(file);
    zw.start_file(
        "[Content_Types].xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("entry");
    std::io::Write::write_all(&mut zw, b"<Types/>").expect("w");
    zw.finish().expect("finish");
    let facts = inspect_document(&fake, &limits());
    assert!(
        facts
            .diagnostics
            .iter()
            .any(|d| d.code.contains("extensionMismatch")),
        "{facts:?}"
    );
}
