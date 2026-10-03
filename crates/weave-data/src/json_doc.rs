//! JSON as Data（M5 上 §49–§51/§92–§94）：root 语义 + 观测 schema
//! profiling。类型是 potential 观测（§6/§93），混合类型如实报告为集合。

use serde_json::Value;

use crate::diagnostics::{Basis, DataDiagnostic, DataSeverity};

/// JSON root 形态（§51）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRootKind {
    Array,
    Object,
    Primitive,
}

/// 一个 schema path 的观测（§92/§94）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonSchemaPath {
    /// 稳定表示：`$.id`、`$.profile.age`（对象点路径；数组值整体为一个
    /// cell 值，不做 `[*]` 展开——§54 Array as JSON cell 的同源决策）。
    pub path: String,
    /// 观测到的 potential 类型集合（排序后，如 "Integer | Null"）。
    pub potential_types: Vec<String>,
    /// 出现率分子（非缺失行数）与分母（观测行数）。
    pub presence_count: u64,
    pub presence_total: u64,
    /// null 出现数。
    pub null_count: u64,
}

impl JsonSchemaPath {
    /// 出现率百分比（观测范围内，精确；§48 honesty：total = 观测行数）。
    pub fn presence_rate(&self) -> Option<f64> {
        if self.presence_total == 0 {
            None
        } else {
            Some(self.presence_count as f64 / self.presence_total as f64 * 100.0)
        }
    }

    pub fn null_rate(&self) -> Option<f64> {
        if self.presence_total == 0 {
            None
        } else {
            Some(self.null_count as f64 / self.presence_total as f64 * 100.0)
        }
    }
}

/// JSON profile（§49）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonProfile {
    pub root: JsonRootKind,
    /// root = Array 时的记录数；Object/Primitive 为 1/1。
    pub records: u64,
    /// 按 path 首次出现顺序排序（确定性；serde_json preserve_order 保证）。
    pub paths: Vec<JsonSchemaPath>,
    /// 采样事实（§48）：是否只观测了前 N 条。
    pub sampled: bool,
    pub observed_rows: u64,
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "Null",
        Value::Bool(_) => "Boolean",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "Integer"
            } else {
                "Decimal"
            }
        }
        Value::String(_) => "String",
        Value::Array(_) => "Array",
        Value::Object(_) => "Object",
    }
}

/// 展平对象为点路径（§53 flatten 同源）；数组值不展开（整体为一个 cell，
/// §54）。空对象产生零路径——由调用方以 `$.`（empty object）占位。
fn flatten<'a>(prefix: &str, value: &'a Value, out: &mut Vec<(String, &'a Value)>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (k, v) in map {
                let path = if prefix == "$" {
                    format!("$.{k}")
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&path, v, out);
            }
        }
        other => out.push((prefix.to_string(), other)),
    }
}

/// 对 JSON 记录集做观测 schema profiling（§92）。
/// `max_records`：超过则只观测前 N 条并 `sampled=true`（§48 如实标注）。
pub fn profile_json(root: &Value, max_records: usize) -> Result<JsonProfile, Vec<DataDiagnostic>> {
    let (root_kind, records): (JsonRootKind, Vec<&Value>) = match root {
        Value::Array(items) => (JsonRootKind::Array, items.iter().collect()),
        Value::Object(_) => (JsonRootKind::Object, vec![root]),
        other => (JsonRootKind::Primitive, vec![other]),
    };
    if root_kind == JsonRootKind::Primitive {
        return Err(vec![
            DataDiagnostic::new(
                "data.jsonRootPrimitive",
                DataSeverity::ValidationError,
                "root is a primitive; record profiling requires an array or object",
            )
            .with_basis(Basis::Fact),
        ]);
    }

    let total = records.len();
    let observed = total.min(max_records);
    let sampled = observed < total;

    // path -> (types set, presence, null)
    let mut order: Vec<String> = Vec::new();
    let mut stats: std::collections::HashMap<String, (Vec<&'static str>, u64, u64)> =
        std::collections::HashMap::new();

    for record in records.iter().take(observed) {
        let mut cells: Vec<(String, &Value)> = Vec::new();
        match *record {
            Value::Object(_) => flatten("$", record, &mut cells),
            Value::Array(items) => {
                // root = object 数组之外也允许 array-of-arrays？§51：仅保证
                // object 记录；array-of-non-objects 视为单列（path "$"）
                if items.iter().any(|v| matches!(v, Value::Object(_))) {
                    return Err(vec![
                        DataDiagnostic::new(
                            "data.jsonMixedRecords",
                            DataSeverity::ValidationError,
                            "array mixes objects with other value types",
                        )
                        .with_basis(Basis::Fact),
                    ]);
                }
                for v in items {
                    cells.push(("$".to_string(), v));
                }
            }
            _ => unreachable!("root primitive filtered above"),
        }
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (path, v) in cells {
            let entry = stats.entry(path.clone()).or_insert_with(|| {
                order.push(path.clone());
                (Vec::new(), 0, 0)
            });
            let t = type_name(v);
            if !entry.0.contains(&t) {
                entry.0.push(t);
            }
            if v.is_null() {
                entry.2 += 1;
            }
            seen.insert(path);
        }
        for path in &order {
            if seen.contains(path) {
                stats.get_mut(path).expect("exists").1 += 1;
            }
        }
    }

    // presence 分母 = 观测行数（含缺失行——缺失即 presence 减少）
    let mut paths: Vec<JsonSchemaPath> = order
        .into_iter()
        .map(|path| {
            let (types, presence, nulls) = stats.remove(&path).expect("tracked");
            let mut types: Vec<String> = types.into_iter().map(str::to_string).collect();
            types.sort();
            JsonSchemaPath {
                path,
                potential_types: types,
                presence_count: presence,
                presence_total: observed as u64,
                null_count: nulls,
            }
        })
        .collect();
    paths.sort_by_key(|p| p.path.clone());

    Ok(JsonProfile {
        root: root_kind,
        records: total as u64,
        paths,
        sampled,
        observed_rows: observed as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn profiles_paths_types_presence_null_and_mixed() {
        // §50/§93：缺失属性 presence<100%；混合类型报告集合
        let root = json!([
            {"id": 1, "name": "A"},
            {"id": 2, "name": "B", "meta": {"age": 18}},
            {"id": 3, "name": null, "meta": {"age": "unknown"}}
        ]);
        let profile = profile_json(&root, 10_000).expect("profile");
        assert_eq!(profile.root, JsonRootKind::Array);
        assert_eq!(profile.records, 3);
        assert!(!profile.sampled);

        let id = profile.paths.iter().find(|p| p.path == "$.id").expect("id");
        assert_eq!(id.potential_types, vec!["Integer"]);
        assert_eq!(id.presence_count, 3);
        assert_eq!(id.presence_total, 3);

        let meta_age = profile
            .paths
            .iter()
            .find(|p| p.path == "$.meta.age")
            .expect("meta.age");
        assert_eq!(meta_age.presence_count, 2, "缺失行降低 presence（§50）");
        assert_eq!(
            meta_age.potential_types,
            vec!["Integer", "String"],
            "§93 混合类型"
        );

        let name = profile
            .paths
            .iter()
            .find(|p| p.path == "$.name")
            .expect("name");
        assert_eq!(name.null_count, 1);
        assert_eq!(
            name.potential_types,
            vec!["Null", "String"],
            "null 计入类型集合"
        );
    }

    #[test]
    fn sampling_is_honest() {
        // §48：超过 max_records ⇒ sampled=true，observed 如实
        let root = json!([{"i": 0}, {"i": 1}, {"j": 2}]);
        let profile = profile_json(&root, 2).expect("profile");
        assert!(profile.sampled);
        assert_eq!(profile.observed_rows, 2);
        assert_eq!(profile.records, 3);
        assert!(
            profile.paths.iter().all(|p| p.path != "$.j"),
            "未观测路径不出现"
        );
    }

    #[test]
    fn primitive_root_is_structured_rejection() {
        // §51：primitive root 的记录 profiling ⇒ 结构化拒绝
        let err = profile_json(&json!(42), 10).expect_err("primitive");
        assert_eq!(err[0].code, "data.jsonRootPrimitive");
    }

    #[test]
    fn arrays_are_cells_not_expanded() {
        // §54 同源：数组整体为一个 cell（类型 Array），不展开 [*]
        let root = json!([{"tags": ["a", "b"]}]);
        let profile = profile_json(&root, 10).expect("profile");
        let tags = profile
            .paths
            .iter()
            .find(|p| p.path == "$.tags")
            .expect("tags");
        assert_eq!(tags.potential_types, vec!["Array"]);
    }
}
