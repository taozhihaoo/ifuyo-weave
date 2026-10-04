//! M8（下）fixtures / golden / TOCTOU / 顺序 / property / fuzz 测试
//! （§131-§145/§175-§176）。
//!
//! fixtures = `tests/fixtures/documents/`，由 `examples/gen_fixtures.rs`
//! 生成（synthetic，可复现，来源明确 §132）。

use std::path::PathBuf;

use weave_documents::{
    DocumentFormat, DocumentResourceLimits, Field, execute_extract_pages, execute_merge,
    execute_reorder, inspect_document, parse_page_ranges, plan_merge, split_every_n,
};

fn limits() -> DocumentResourceLimits {
    DocumentResourceLimits::default()
}

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/documents")
        .join(rel)
}

fn page_markers(doc: &lopdf::Document) -> Vec<String> {
    doc.get_pages()
        .values()
        .map(|id| {
            let content = doc.get_page_content(*id);
            String::from_utf8_lossy(&content)
                .split("WeaveMarker:page-")
                .nth(1)
                .map(|tail| tail.split(')').next().unwrap_or("").to_owned())
                .unwrap_or_default()
        })
        .collect()
}

// ── §136 Golden ──

#[test]
fn golden_pdf_inspect_6page() {
    let facts = inspect_document(&fixture("valid/pdf-6page.pdf"), &limits());
    assert_eq!(facts.format, DocumentFormat::Pdf);
    assert!(matches!(facts.pages, Some(Field::Known(6.0))));
    assert_eq!(
        facts
            .metadata
            .iter()
            .find(|(k, _)| k == "Title")
            .map(|(_, v)| v.as_str()),
        Some("Fixture Meta"),
        "golden：元数据提取"
    );
}

#[test]
fn golden_docx_inspect_counts() {
    let facts = inspect_document(&fixture("valid/headings.docx"), &limits());
    assert_eq!(facts.format, DocumentFormat::Docx);
    let s = &facts.statistics;
    assert!(matches!(s.headings, Some(Field::Known(3.0))), "{s:?}");
    assert!(matches!(s.paragraphs, Some(Field::Known(7.0))), "{s:?}"); // 3 heading-p + 4 plain-p
    assert!(matches!(s.tables, Some(Field::Known(0.0))), "{s:?}");
    assert_eq!(
        facts
            .metadata
            .iter()
            .find(|(k, _)| k == "title")
            .map(|(_, v)| v.as_str()),
        Some("Fixture Document")
    );
}

#[test]
fn golden_xlsx_inspect_sheets() {
    let facts = inspect_document(&fixture("valid/sheets.xlsx"), &limits());
    let Some(Field::Known(sheets)) = &facts.sheets else {
        panic!("sheets {:?}", facts.sheets);
    };
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0].name, "Data");
    assert_eq!(sheets[0].visibility, "visible");
    assert_eq!(sheets[1].name, "Archive");
    assert_eq!(sheets[1].visibility, "hidden");
    assert_eq!(sheets[0].formula_cells, Some(1.0));
}

#[test]
fn golden_pptx_inspect_slides() {
    let facts = inspect_document(&fixture("valid/slides.pptx"), &limits());
    let Some(Field::Known(slides)) = &facts.slides else {
        panic!("slides {:?}", facts.slides);
    };
    assert_eq!(slides.len(), 2);
    assert!(!slides[0].hidden);
    assert!(slides[1].hidden);
}

#[test]
fn malformed_fixtures_never_panic_and_report() {
    // §86/§139：malformed corpus ⇒ 结构化诊断，无 panic
    for rel in [
        "malformed/pdf-truncated.pdf",
        "malformed/docx-broken.xml.docx",
        "malformed/xlsx-broken.xlsx",
        "malformed/pptx-broken.pptx",
    ] {
        let facts = inspect_document(&fixture(rel), &limits());
        assert!(!facts.diagnostics.is_empty(), "{rel}: {facts:?}");
    }
}

// ── §142 Merge 顺序（内容标记验证）──

#[test]
fn merge_order_is_input_order_verified_by_markers() {
    // A(1页) + B(6页) ⇒ 7 页，标记顺序 = A1, B1..B6
    let dir = tempfile::tempdir().expect("ws");
    let out = dir.path().join("merged.pdf");
    let a = fixture("valid/pdf-1page.pdf");
    let b = fixture("valid/pdf-6page.pdf");
    let plan = plan_merge(&[a, b], &limits()).expect("plan");
    let (pages, _) = execute_merge(&plan, &out, &limits()).expect("merge");
    assert_eq!(pages, 7);
    let doc = lopdf::Document::load(&out).expect("reload");
    let markers = page_markers(&doc);
    assert_eq!(
        markers,
        vec!["1", "1", "2", "3", "4", "5", "6"],
        "§142 顺序 = 输入序"
    );
}

// ── §143 Split 范围（标记验证）──

#[test]
fn split_ranges_select_exact_pages() {
    let dir = tempfile::tempdir().expect("ws");
    let out = dir.path().join("part.pdf");
    let src = fixture("valid/pdf-10page.pdf");
    let pages = execute_extract_pages(&src, "2-4,8-10", &out, &limits()).expect("extract");
    assert_eq!(pages, 6);
    let doc = lopdf::Document::load(&out).expect("reload");
    assert_eq!(
        page_markers(&doc),
        vec!["2", "3", "4", "8", "9", "10"],
        "§143"
    );
}

// ── §144 Reorder（非排序语义）──

#[test]
fn reorder_preserves_requested_permutation_not_sorted() {
    let dir = tempfile::tempdir().expect("ws");
    let out = dir.path().join("reordered.pdf");
    let src = fixture("valid/pdf-6page.pdf");
    // §24 完整置换；验证输出顺序 = 请求顺序（非排序）
    let pages = execute_reorder(&src, &[6, 2, 4, 1, 5, 3], &out, &limits()).expect("reorder");
    assert_eq!(pages, 6);
    let doc = lopdf::Document::load(&out).expect("reload");
    assert_eq!(
        page_markers(&doc),
        vec!["6", "2", "4", "1", "5", "3"],
        "§144"
    );
}

// ── §145 Rotate（选定页 / 未选定页不变）──

#[test]
fn rotated_fixture_reports_rotation_metadata() {
    let facts = inspect_document(&fixture("valid/pdf-rotated.pdf"), &limits());
    assert!(matches!(facts.pages, Some(Field::Known(2.0))));
    // 域内直读 /Rotate（inspect.facts 的 rotations 字段在 PdfFacts）
    let pdf_facts = weave_documents::pdf_inspect(&fixture("valid/pdf-rotated.pdf"), &limits())
        .expect("inspect");
    assert_eq!(pdf_facts.rotations, vec![90, 270], "fixture 预置旋转");
}

// ── §141 TOCTOU ──

#[test]
fn merge_detects_changed_input_since_plan() {
    let dir = tempfile::tempdir().expect("ws");
    let a = dir.path().join("a.pdf");
    let b = dir.path().join("b.pdf");
    // 从 fixtures 复制（可变副本）
    std::fs::copy(fixture("valid/pdf-1page.pdf"), &a).expect("copy");
    std::fs::copy(fixture("valid/pdf-6page.pdf"), &b).expect("copy");
    let plan = plan_merge(&[a.clone(), b.clone()], &limits()).expect("plan");
    // Inspect 后外部修改（重写文件 ⇒ size/mtime 变化）
    std::fs::write(&a, b"%PDF-1.4 tampered").expect("modify");
    let out = dir.path().join("merged.pdf");
    let e = execute_merge(&plan, &out, &limits()).expect_err("TOCTOU");
    assert_eq!(
        e.code, "pdf.changedSincePlan",
        "§141 DetectedChangedSincePlan"
    );
    assert!(!out.exists(), "盲执行不得产出");
}

// ── §138 Property：split 划分恰好覆盖一次 ──

#[test]
fn property_split_partitions_cover_exactly_once() {
    for total in [1u64, 5, 12, 37, 100] {
        for n in [1u64, 2, 3, 7, 50] {
            let chunks = split_every_n(total, n).expect("ok");
            let mut covered: Vec<u64> = Vec::new();
            for (start, end) in &chunks {
                assert!(start <= end);
                covered.extend(*start..=*end);
            }
            covered.sort_unstable();
            let expected: Vec<u64> = (1..=total).collect();
            assert_eq!(covered, expected, "total={total} n={n}");
        }
    }
}

#[test]
fn property_range_parser_roundtrip_sorted_unique() {
    // 种子确定性伪随机（§117/§199 同源）
    let mut seed: u64 = 0xA11C_E8F0_00B5;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..500 {
        let max = (next() % 50) + 1;
        let mut tokens = Vec::new();
        for _ in 0..(next() % 4 + 1) {
            let a = (next() % max) + 1;
            let b = (next() % max) + 1;
            let (lo, hi) = (a.min(b), a.max(b));
            if lo == hi {
                tokens.push(lo.to_string());
            } else {
                tokens.push(format!("{lo}-{hi}"));
            }
        }
        let input = tokens.join(",");
        let pages = parse_page_ranges(&input, Some(max)).expect("valid by construction");
        assert!(pages.windows(2).all(|w| w[0] < w[1]), "升序去重 {input}");
        assert!(pages.iter().all(|p| (1..=max).contains(p)), "界内 {input}");
    }
}

// ── §139 Fuzz：随机字节 ⇒ no panic ──

#[test]
fn fuzz_detector_and_inspector_never_panic_on_random_bytes() {
    let dir = tempfile::tempdir().expect("ws");
    let mut seed: u64 = 0xF00D_BEEF_CAFE;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for i in 0..300 {
        let len = (next() % 4096) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| (next() & 0xFF) as u8).collect();
        let mut data = bytes;
        // 部分样本带 PDF/ZIP 魔数前缀（覆盖检测分支）
        if i % 3 == 0 {
            data.splice(0..0, b"%PDF-1.7\n".to_vec());
        } else if i % 3 == 1 {
            data.splice(0..0, b"PK\x03\x04".to_vec());
        }
        let path = dir.path().join(format!("fuzz-{i}.bin"));
        std::fs::write(&path, &data).expect("write");
        let _ = inspect_document(&path, &limits()); // 不 panic 即目标
        let _ = std::fs::remove_file(&path);
    }
}

// ── §175 Unicode / §176 Long path ──

#[test]
fn unicode_document_names_inspect_cleanly() {
    let dir = tempfile::tempdir().expect("ws");
    for name in [
        "中文 文档.pdf",
        "日本語の資料.pdf",
        "한국어.pdf",
        "doc émoji ✓.pdf",
    ] {
        let p = dir.path().join(name);
        std::fs::copy(fixture("valid/pdf-1page.pdf"), &p).expect("copy");
        let facts = inspect_document(&p, &limits());
        assert!(
            matches!(facts.pages, Some(Field::Known(1.0))),
            "{name}: {facts:?}"
        );
    }
}

#[test]
fn long_nested_path_inspects() {
    let dir = tempfile::tempdir().expect("ws");
    let mut deep = dir.path().to_path_buf();
    for i in 0..8 {
        deep = deep.join(format!("level-{i}-directory-with-long-name"));
    }
    std::fs::create_dir_all(&deep).expect("mkdir");
    let p = deep.join(format!("{}.pdf", "n".repeat(120)));
    std::fs::copy(fixture("valid/pdf-1page.pdf"), &p).expect("copy");
    let facts = inspect_document(&p, &limits());
    assert!(matches!(facts.pages, Some(Field::Known(1.0))));
}
