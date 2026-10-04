//! 文档 fixtures 生成器（M8 下 §131-§135）。
//!
//! 运行：`cargo run -p weave-documents --example gen_fixtures tests/fixtures/documents`
//! 产出小型 synthetic fixtures（§132 明确允许 synthetic 并注明来源——
//! 本仓库全部 fixtures 由本生成器产出，可复现、无许可证负担）。
//!
//! PDF 页内含文本标记 `WeaveMarker:page-N`，供 merge 顺序验证（§142）
//! 通过 get_page_content 断言页序。

use std::io::Write;
use std::path::PathBuf;

use lopdf::{Document, Object, Stream, dictionary};

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tests/fixtures/documents"));
    std::fs::create_dir_all(root.join("valid")).expect("mkdir valid");
    std::fs::create_dir_all(root.join("malformed")).expect("mkdir malformed");

    // ── PDF valid ──
    write_pdf(&root.join("valid/pdf-1page.pdf"), 1, None);
    write_pdf(&root.join("valid/pdf-6page.pdf"), 6, Some("Fixture Meta"));
    write_pdf(&root.join("valid/pdf-10page.pdf"), 10, None);
    write_pdf(&root.join("valid/pdf-rotated.pdf"), 2, None);
    set_rotations(&root.join("valid/pdf-rotated.pdf"), &[90, 270]);
    write_pdf(&root.join("valid/pdf-large-200page.pdf"), 200, None);
    // malformed：PDF 头 + 垃圾
    std::fs::write(
        root.join("malformed/pdf-truncated.pdf"),
        b"%PDF-1.4\n1 0 obj\n<< broken",
    )
    .expect("write");

    // ── Office valid ──
    write_docx(&root.join("valid/minimal.docx"), 2, 1, 1);
    write_docx(&root.join("valid/headings.docx"), 4, 0, 3);
    std::fs::write(
        root.join("malformed/docx-broken.xml.docx"),
        b"PK\x03\x04 not really structured",
    )
    .expect("write");

    write_xlsx(&root.join("valid/sheets.xlsx"));
    std::fs::write(root.join("malformed/xlsx-broken.xlsx"), b"PK\x03\x04 junk").expect("write");

    write_pptx(&root.join("valid/slides.pptx"));
    std::fs::write(root.join("malformed/pptx-broken.pptx"), b"PK\x03\x04 junk").expect("write");

    println!("fixtures written to {}", root.display());
}

/// synthetic PDF：n 页，页 i 内容 = `WeaveMarker:page-{i}` 文本标记。
fn write_pdf(path: &std::path::Path, pages: usize, title: Option<&str>) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.add_object(dictionary! {});
    let mut kids = Vec::new();
    for i in 1..=pages {
        let text = format!("BT /F1 12 Tf 72 720 Td (WeaveMarker:page-{i}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary! {}, text.into_bytes()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => Object::Reference(content_id),
            "Resources" => dictionary! {},
        });
        kids.push(Object::Reference(page_id));
    }
    {
        let d = doc
            .objects
            .get_mut(&pages_id)
            .expect("pages")
            .as_dict_mut()
            .expect("d");
        d.set("Kids", Object::Array(kids));
        d.set("Count", Object::Integer(pages as i64));
        d.set("MediaBox", vec![0.into(), 0.into(), 595.into(), 842.into()]);
    }
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    if let Some(title) = title {
        let info_id = doc.add_object(dictionary! {
            "Title" => Object::String(title.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        });
        doc.trailer.set("Info", Object::Reference(info_id));
    }
    doc.save(path).expect("save pdf");
}

fn set_rotations(path: &std::path::Path, rotations: &[i64]) {
    let mut doc = Document::load(path).expect("load");
    for ((_index, id), rot) in doc.get_pages().iter().zip(rotations) {
        let dict = doc
            .get_object_mut(*id)
            .and_then(|o| o.as_dict_mut())
            .expect("page");
        dict.set("Rotate", Object::Integer(*rot));
    }
    doc.save(path).expect("save");
}

fn write_docx(path: &std::path::Path, paragraphs: usize, tables: usize, headings: usize) {
    let file = std::fs::File::create(path).expect("f");
    let mut zw = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    zw.start_file("[Content_Types].xml", opts).expect("e");
    zw.write_all(b"<Types/>").expect("w");
    zw.start_file("docProps/core.xml", opts).expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><cp:coreProperties xmlns:cp=\"cp\">",
            "<dc:title xmlns:dc=\"dc\">Fixture Document</dc:title>",
            "<cp:creator>gen_fixtures</cp:creator></cp:coreProperties>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.start_file("word/document.xml", opts).expect("e");
    let mut body = String::from("<?xml version=\"1.0\"?><w:document xmlns:w=\"w\"><w:body>");
    for i in 0..headings {
        body.push_str(&format!(
            "<w:p><w:pPr><w:pStyle w:val=\"Heading{i}\"/></w:pPr><w:r><w:t>H{i}</w:t></w:r></w:p>"
        ));
    }
    for i in 0..paragraphs {
        body.push_str(&format!(
            "<w:p><w:r><w:t>fixture paragraph {i} content</w:t></w:r></w:p>"
        ));
    }
    for _ in 0..tables {
        body.push_str(
            "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
        );
    }
    body.push_str("</w:body></w:document>");
    zw.write_all(body.as_bytes()).expect("w");
    zw.finish().expect("finish");
}

fn write_xlsx(path: &std::path::Path) {
    let file = std::fs::File::create(path).expect("f");
    let mut zw = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    zw.start_file("[Content_Types].xml", opts).expect("e");
    zw.write_all(b"<Types/>").expect("w");
    zw.start_file("xl/workbook.xml", opts).expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><workbook><sheets>",
            "<sheet name=\"Data\" sheetId=\"1\" state=\"visible\"/>",
            "<sheet name=\"Archive\" sheetId=\"2\" state=\"hidden\"/>",
            "</sheets></workbook>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.start_file("xl/worksheets/sheet1.xml", opts).expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><worksheet><dimension ref=\"A1:B3\"/><sheetData>",
            "<row r=\"1\"><c r=\"A1\"><v>1</v></c><c r=\"B1\"><f>SUM(A1:A2)</f><v>3</v></c></row>",
            "<row r=\"2\"><c r=\"A2\"><v>2</v></c></row>",
            "</sheetData></worksheet>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.start_file("xl/worksheets/sheet2.xml", opts).expect("e");
    zw.write_all(b"<?xml version=\"1.0\"?><worksheet><sheetData/></worksheet>")
        .expect("w");
    zw.finish().expect("finish");
}

fn write_pptx(path: &std::path::Path) {
    let file = std::fs::File::create(path).expect("f");
    let mut zw = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    zw.start_file("[Content_Types].xml", opts).expect("e");
    zw.write_all(b"<Types/>").expect("w");
    zw.start_file("ppt/slides/slide1.xml", opts).expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><p:sld xmlns:p=\"p\"><p:cSld><p:spTree>",
            "<p:sp><p:txBody><a:p><a:t>Slide one</a:t></a:p></p:txBody></p:sp>",
            "</p:spTree></p:cSld></p:sld>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.start_file("ppt/slides/slide2.xml", opts).expect("e");
    zw.write_all(
        concat!(
            "<?xml version=\"1.0\"?><p:sld xmlns:p=\"p\" show=\"0\"><p:cSld><p:spTree>",
            "<p:sp><p:txBody><a:p><a:t>Hidden</a:t></a:p></p:txBody></p:sp>",
            "</p:spTree></p:cSld></p:sld>",
        )
        .as_bytes(),
    )
    .expect("w");
    zw.finish().expect("finish");
}
