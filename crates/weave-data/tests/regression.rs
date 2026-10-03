//! 性质 / 回归测试（M5 下 §126–§138）：Unicode、roundtrip、escaping、
//! 行列计数、确定性——用确定性文本夹具（§177），不用玩具 hello,world。

use std::io::Cursor;

use weave_data::{
    CsvDialect, DataLimits, DataSession, FilterOperator, FilterRule, SortSpec, parse_csv,
    parse_jsonl, table_to_delimited, table_to_json,
};

fn parse(src: &str, dialect: &CsvDialect) -> weave_data::CsvParseOutput {
    parse_csv(
        Cursor::new(src.as_bytes().to_vec()),
        dialect,
        &DataLimits::default(),
    )
    .expect("parse")
}

fn roundtrip(src: &str, dialect: &CsvDialect) -> String {
    let out = parse(src, dialect);
    let table = weave_data::DataTable {
        columns: out.columns,
        rows: out.rows,
    };
    table_to_delimited(
        &table,
        dialect.delimiter,
        dialect.quote,
        true,
        weave_data::LineEnding::Lf,
    )
    .expect("serialize")
}

#[test]
fn csv_unicode_roundtrip_semantically_lossless() {
    // §126/§129：中日/emoji/全角——语义等价而非字节等价（引号风格可归一）
    let src = "姓名,城市,备注\n张三,重庆,\"你好 🌱\"\n李四,東京,\"こんにちは\"\n";
    let out = parse(src, &CsvDialect::csv());
    assert_eq!(out.rows[0], vec!["张三", "重庆", "你好 🌱"]);
    assert_eq!(out.rows[1][1], "東京");
    let table = weave_data::DataTable {
        columns: out.columns,
        rows: out.rows,
    };
    let json = table_to_json(&table, weave_data::TypedMode::PreserveStrings);
    assert_eq!(json[0]["姓名"], "张三");
    assert_eq!(json[0]["备注"], "你好 🌱");
}

#[test]
fn csv_escaping_roundtrip_preserves_semantics() {
    // §133：逗号/引号/换行/tab/首尾空格/空 cell
    let src = "a,b\n\"x,y\",\"say \"\"hi\"\"\"\n\"multi\nline\",\"  padded  \"\n,\"\"\n";
    let reparsed = roundtrip(src, &CsvDialect::csv());
    let again = parse(&reparsed, &CsvDialect::csv());
    let original = parse(src, &CsvDialect::csv());
    assert_eq!(again.rows, original.rows, "语义等价（§129）");
    assert_eq!(again.columns.len(), original.columns.len());
}

#[test]
fn numeric_strings_never_coerced() {
    // §131：大整数/科学计数/前导零——Preserve Strings 下全部原样
    let src = "v\n0\n-1\n1.5\n0.1\n99999999999999999999999\n1e10\n00123\n";
    let out = parse(src, &CsvDialect::csv());
    let table = weave_data::DataTable {
        columns: out.columns,
        rows: out.rows,
    };
    let json = table_to_json(&table, weave_data::TypedMode::PreserveStrings);
    let expected = [
        "0",
        "-1",
        "1.5",
        "0.1",
        "99999999999999999999999",
        "1e10",
        "00123",
    ];
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(json[i]["v"], *want, "row {i} 值不得被改写（§131）");
    }
}

#[test]
fn jsonl_unicode_and_escaped_forms_equal() {
    // §127：escaped Unicode 与实际 Unicode 解析结果一致
    let escaped = "{\"name\":\"\\u5f20\\u4e09\"}\n";
    let actual = "{\"name\":\"张三\"}\n";
    let r1 = parse_jsonl(
        Cursor::new(escaped.as_bytes().to_vec()),
        weave_data::JsonlErrorMode::FailFast,
        100,
        100,
    )
    .expect("escaped");
    let r2 = parse_jsonl(
        Cursor::new(actual.as_bytes().to_vec()),
        weave_data::JsonlErrorMode::FailFast,
        100,
        100,
    )
    .expect("actual");
    assert_eq!(r1.records, r2.records);
}

#[test]
fn row_and_column_count_edges() {
    // §134/§135：0 行 / 1 行 / 宽表 / 重复表头 / 空 header
    let empty = parse("a,b\n", &CsvDialect::csv());
    assert_eq!(empty.data_rows, 0);

    let one = parse("a\n1\n", &CsvDialect::csv());
    assert_eq!(one.data_rows, 1);

    let wide_src = format!(
        "{}\n{}\n",
        "h,".repeat(500).trim_end_matches(','),
        "v,".repeat(500).trim_end_matches(',')
    );
    let wide = parse(&wide_src, &CsvDialect::csv());
    assert_eq!(wide.columns.len(), 500);

    let dup = parse("n,n\n1,2\n", &CsvDialect::csv());
    assert_eq!(dup.columns[1].name, "n_2");
}

#[test]
fn session_filter_sort_deterministic_and_stable() {
    // §124/§121：同输入同选项必同输出；相等键保持原序
    let columns = weave_data::table::build_columns(&["k".to_string(), "v".to_string()]);
    let rows = vec![
        vec!["b".to_string(), "1".to_string()],
        vec!["a".to_string(), "2".to_string()],
        vec!["b".to_string(), "3".to_string()],
    ];
    let mut session = DataSession::new(weave_data::DataTable { columns, rows });
    session.sort = Some(SortSpec {
        column_id: "col_1".into(),
        descending: false,
        ignore_case: false,
    });
    session.filters = vec![FilterRule {
        column_id: "col_2".into(),
        operator: FilterOperator::NotEmpty,
        value: String::new(),
        case_sensitive: true,
    }];
    session.refresh_view();
    let first = session.view.indices.clone();
    session.refresh_view();
    assert_eq!(first, session.view.indices, "确定性（§121）");
    // 稳定：两行 k=b 保持原相对顺序
    assert_eq!(first, vec![1, 0, 2]);
}

#[test]
fn jsonl_malformed_collect_mode_counts() {
    // §62/§118：错误预算内收集 + 计数，不无限累积
    let src = "{\"a\":1}\nbad\n{\"a\":2}\nalso bad\n";
    let r = parse_jsonl(
        Cursor::new(src.as_bytes().to_vec()),
        weave_data::JsonlErrorMode::CollectErrors,
        100,
        1, // 错误预算 = 1 ⇒ 第二条错误不再收集但 invalid 计数继续
    )
    .expect("collect");
    assert_eq!(r.valid_count, 2);
    assert_eq!(r.invalid_count, 2);
    assert_eq!(r.diagnostics.len(), 1, "错误预算（§118）");
}
