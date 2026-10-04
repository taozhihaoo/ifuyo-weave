//! Office inspection 测试（M8 上 §29-§52/§87）。
//!
//! Fixtures = **synthetic OOXML**（测试内用 zip crate 构造的最小合法包，
//! §7 明确标识）——验证的是 Weave 的解析逻辑，不是 Office 兼容性。

use std::io::Write;

use zip::ZipWriter;

use crate::{DocumentFormat, DocumentResourceLimits, Field, inspect_document};

fn limits() -> DocumentResourceLimits {
    DocumentResourceLimits::default()
}

fn ws() -> tempfile::TempDir {
    tempfile::tempdir().expect("ws")
}

/// 最小 DOCX：word/document.xml + docProps/core.xml + media。
fn make_docx(path: &std::path::Path, paragraphs: usize, tables: usize, headings: usize) {
    let file = std::fs::File::create(path).expect("f");
    let mut zw = ZipWriter::new(file);
    zw.start_file(
        "[Content_Types].xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(b"<Types/>").expect("w");
    zw.start_file(
        "docProps/core.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><core:coreProperties xmlns:core=\"cp\">",
            "<dc:title>Office Doc</dc:title><cp:creator>Weave</cp:creator>",
            "</core:coreProperties>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.start_file(
        "word/document.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    let mut body = String::from("<?xml version=\"1.0\"?><w:document xmlns:w=\"w\"><w:body>");
    for i in 0..headings {
        body.push_str(&format!(
            "<w:p><w:pPr><w:pStyle w:val=\"Heading{i}\"/></w:pPr><w:r><w:t>Head {i}</w:t></w:r></w:p>"
        ));
    }
    for i in 0..paragraphs {
        body.push_str(&format!(
            "<w:p><w:r><w:t>lorem ipsum dolor {i} sit amet</w:t></w:r></w:p>"
        ));
    }
    for _ in 0..tables {
        body.push_str(
            "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
        );
    }
    body.push_str("<w:hyperlink r:id=\"r1\"><w:r><w:t>link</w:t></w:r></w:hyperlink>");
    body.push_str("</w:body></w:document>");
    zw.write_all(body.as_bytes()).expect("w");
    zw.start_file(
        "word/media/image1.png",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(b"\x89PNG fake bytes").expect("w");
    zw.finish().expect("finish");
}

#[test]
fn docx_inspect_reports_structure_and_metadata() {
    let dir = ws();
    let docx = dir.path().join("synthetic.docx");
    make_docx(&docx, 5, 2, 2);
    let facts = inspect_document(&docx, &limits());
    assert_eq!(facts.format, DocumentFormat::Docx);
    let stats = &facts.statistics;
    assert!(
        matches!(stats.paragraphs, Some(Field::Known(9))),
        "{stats:?}"
    ); // 2 heading + 5 plain + 2 表格单元格
    assert!(matches!(stats.headings, Some(Field::Known(2))), "{stats:?}");
    assert!(matches!(stats.tables, Some(Field::Known(2))));
    assert!(matches!(stats.images, Some(Field::Known(1))));
    assert!(matches!(stats.hyperlinks, Some(Field::Known(1))));
    // §34：words = Estimated（非精确）
    assert!(
        matches!(stats.words, Some(Field::Estimated(_))),
        "{stats:?}"
    );
    // §32：core properties
    assert_eq!(
        facts
            .metadata
            .iter()
            .find(|(k, _)| k == "title")
            .map(|(_, v)| v.as_str()),
        Some("Office Doc")
    );
}

/// 最小 XLSX：workbook（2 sheets，其一 hidden）+ worksheets。
fn make_xlsx(path: &std::path::Path) {
    let file = std::fs::File::create(path).expect("f");
    let mut zw = ZipWriter::new(file);
    zw.start_file(
        "[Content_Types].xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(b"<Types/>").expect("w");
    zw.start_file("xl/workbook.xml", zip::write::SimpleFileOptions::default())
        .expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><workbook><sheets>",
            "<sheet name=\"Data\" sheetId=\"1\" state=\"visible\"/>",
            "<sheet name=\"Secret\" sheetId=\"2\" state=\"veryHidden\"/>",
            "</sheets></workbook>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.start_file(
        "xl/worksheets/sheet1.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(concat!(
        "<?xml version=\"1.0\"?><worksheet><dimension ref=\"A1:C4\"/><sheetData>",
        "<row r=\"1\"><c r=\"A1\" t=\"n\"><v>1</v></c><c r=\"B1\"><f>SUM(A1:A2)</f><v>3</v></c></row>",
        "<row r=\"2\"><c r=\"A2\" t=\"n\"><v>2</v></c></row>",
        "</sheetData><mergeCells count=\"1\"><mergeCell ref=\"A1:B1\"/></mergeCells></worksheet>",
    ).as_bytes())
    .expect("w");
    zw.start_file(
        "xl/worksheets/sheet2.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(b"<?xml version=\"1.0\"?><worksheet><sheetData/></worksheet>")
        .expect("w");
    zw.finish().expect("finish");
}

#[test]
fn xlsx_inspect_reports_sheets_visibility_and_stats() {
    let dir = ws();
    let xlsx = dir.path().join("synthetic.xlsx");
    make_xlsx(&xlsx);
    let facts = inspect_document(&xlsx, &limits());
    assert_eq!(facts.format, DocumentFormat::Xlsx);
    let Some(Field::Known(sheets)) = &facts.sheets else {
        panic!("sheets: {:?}", facts.sheets);
    };
    assert_eq!(sheets.len(), 2);
    // §44：veryHidden 如实区分（不是统统 Hidden）
    assert_eq!(sheets[0].visibility, "visible");
    assert_eq!(sheets[1].visibility, "veryHidden");
    // §41：dimension（声明）≠ populated cells（实际）——两者都报
    assert_eq!(sheets[0].dimension.as_deref(), Some("A1:C4"));
    assert_eq!(sheets[0].populated_cells, Some(3));
    assert_eq!(sheets[0].row_count, Some(4), "dimension 行数");
    // §42：公式只报 presence 计数（1 个 <f>）
    assert_eq!(sheets[0].formula_cells, Some(1));
    assert_eq!(sheets[0].merged_cells, Some(1));
}

/// 最小 PPTX：2 slides（其一 hidden）+ notes。
fn make_pptx(path: &std::path::Path) {
    let file = std::fs::File::create(path).expect("f");
    let mut zw = ZipWriter::new(file);
    zw.start_file(
        "[Content_Types].xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(b"<Types/>").expect("w");
    zw.start_file(
        "ppt/slides/slide1.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><p:sld xmlns:p=\"p\"><p:cSld><p:spTree>",
            "<p:sp><p:txBody><a:p><a:t>Hello</a:t></a:p></p:txBody></p:sp>",
            "<p:pic/><p:sp/><p:sp/>",
            "</p:spTree></p:cSld></p:sld>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.start_file(
        "ppt/slides/slide2.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><p:sld xmlns:p=\"p\" show=\"0\"><p:cSld><p:spTree>",
            "<p:sp><p:txBody><a:p><a:t>Hidden</a:t></a:p></p:txBody></p:sp>",
            "</p:spTree></p:cSld></p:sld>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.start_file(
        "ppt/notesSlides/notesSlide1.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(b"<?xml version=\"1.0\"?><p:notes/>")
        .expect("w");
    zw.finish().expect("finish");
}

#[test]
fn pptx_inspect_reports_slides_hidden_and_shapes() {
    let dir = ws();
    let pptx = dir.path().join("synthetic.pptx");
    make_pptx(&pptx);
    let facts = inspect_document(&pptx, &limits());
    assert_eq!(facts.format, DocumentFormat::Pptx);
    let Some(Field::Known(slides)) = &facts.slides else {
        panic!("slides: {:?}", facts.slides);
    };
    assert_eq!(slides.len(), 2);
    assert!(!slides[0].hidden);
    assert!(slides[1].hidden, "show=\"0\" ⇒ hidden（如实）");
    assert!(slides[0].has_text);
    assert_eq!(slides[0].image_count, Some(1));
    assert_eq!(slides[0].shape_count, Some(3)); // 3 个 sp
    assert_eq!(slides[0].has_notes, Some(true));
    assert_eq!(slides[1].has_notes, Some(false));
}

#[test]
fn malformed_office_zip_is_structured_error() {
    // §86：invalid zip ⇒ 结构化错误不 panic
    let dir = ws();
    let bad = dir.path().join("bad.docx");
    std::fs::write(&bad, b"PK\x03\x04 truncated garbage").expect("write");
    let facts = inspect_document(&bad, &limits());
    assert!(!facts.diagnostics.is_empty(), "{facts:?}");
}

#[test]
fn zip_bomb_entry_rejected() {
    // §87：单条目解压超限 ⇒ 拒绝（用小 limit 模拟）
    let dir = ws();
    let big = dir.path().join("bomb.docx");
    let file = std::fs::File::create(&big).expect("f");
    let mut zw = ZipWriter::new(file);
    zw.start_file(
        "[Content_Types].xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(b"<Types/>").expect("w");
    zw.start_file(
        "word/document.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .expect("e");
    zw.write_all(&vec![b'a'; 1024]).expect("w");
    zw.finish().expect("finish");
    let tight = DocumentResourceLimits {
        max_entry_size: 512,
        ..DocumentResourceLimits::default()
    };
    let facts = inspect_document(&big, &tight);
    assert!(
        facts
            .diagnostics
            .iter()
            .any(|d| d.message.contains("over limit")),
        "{facts:?}"
    );
}

#[test]
fn txt_inspect_is_supported_lite() {
    let dir = ws();
    let txt = dir.path().join("note.txt");
    std::fs::write(&txt, "plain text").expect("write");
    let facts = inspect_document(&txt, &limits());
    assert_eq!(facts.format, DocumentFormat::Txt);
    assert!(matches!(facts.pages, Some(Field::Unavailable)));
}
