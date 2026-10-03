//! 有界 Top-K 统计结构（M1 §12.3）。
//!
//! 扫描 100k+ 文件时只保留 Top 20，不缓存全量再排序。
//! 排序规则固定（M1 §17）并在比较器中体现；同榜并列用相对路径 ASC 定序，
//! 保证完全确定。

use std::cmp::Ordering;

use super::model::FileLineItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ranking {
    /// size DESC → path ASC
    Largest,
    /// modified ASC → path ASC（modified 不可知的条目不参与时间榜）
    Oldest,
    /// modified DESC → path ASC
    Newest,
}

impl Ranking {
    /// best-first 比较：`a` 排在 `b` 之前返回 Less。
    fn compare(&self, a: &FileLineItem, b: &FileLineItem) -> Ordering {
        let path = |x: &FileLineItem| x.relative_path.to_lowercase();
        match self {
            Ranking::Largest => b.size.cmp(&a.size).then_with(|| path(a).cmp(&path(b))),
            Ranking::Oldest => match (a.modified, b.modified) {
                (Some(x), Some(y)) => x.cmp(&y).then_with(|| path(a).cmp(&path(b))),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => path(a).cmp(&path(b)),
            },
            Ranking::Newest => match (a.modified, b.modified) {
                (Some(x), Some(y)) => y.cmp(&x).then_with(|| path(a).cmp(&path(b))),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => path(a).cmp(&path(b)),
            },
        }
    }
}

/// 有界 best-first 列表。容量固定（K），插入 O(log K)（二分定位）。
#[derive(Debug)]
pub struct TopK {
    ranking: Ranking,
    capacity: usize,
    items: Vec<FileLineItem>,
}

impl TopK {
    pub fn new(ranking: Ranking, capacity: usize) -> Self {
        Self {
            ranking,
            capacity,
            items: Vec::with_capacity(capacity.min(16)),
        }
    }

    pub fn push(&mut self, item: FileLineItem) {
        // modified 不可知的条目不进时间榜（事实缺失不伪造）。
        if matches!(self.ranking, Ranking::Oldest | Ranking::Newest) && item.modified.is_none() {
            return;
        }
        let pos = self
            .items
            .partition_point(|existing| self.ranking.compare(existing, &item) != Ordering::Greater);
        if pos >= self.capacity {
            return;
        }
        self.items.insert(pos, item);
        if self.items.len() > self.capacity {
            self.items.truncate(self.capacity);
        }
    }

    pub fn into_sorted(self) -> Vec<FileLineItem> {
        self.items
    }
}

pub const TOP_K: usize = 20;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn item(path: &str, size: u64, modified: Option<SystemTime>) -> FileLineItem {
        FileLineItem {
            relative_path: path.to_string(),
            size,
            modified,
        }
    }

    fn t(offset: u64) -> Option<SystemTime> {
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(offset))
    }

    #[test]
    fn largest_orders_by_size_desc_then_path_asc() {
        let mut top = TopK::new(Ranking::Largest, 3);
        top.push(item("b.txt", 10, t(1)));
        top.push(item("a.txt", 10, t(1)));
        top.push(item("c.txt", 99, t(1)));
        top.push(item("d.txt", 1, t(1)));
        let list = top.into_sorted();
        let paths: Vec<&str> = list.iter().map(|i| i.relative_path.as_str()).collect();
        assert_eq!(paths, vec!["c.txt", "a.txt", "b.txt"]);
    }

    #[test]
    fn oldest_and_newest_order_by_time_with_path_ties() {
        let mut oldest = TopK::new(Ranking::Oldest, 3);
        oldest.push(item("new.txt", 1, t(300)));
        oldest.push(item("old.txt", 1, t(100)));
        oldest.push(item("mid.txt", 1, t(200)));
        let list = oldest.into_sorted();
        let paths: Vec<&str> = list.iter().map(|i| i.relative_path.as_str()).collect();
        assert_eq!(paths, vec!["old.txt", "mid.txt", "new.txt"]);

        let mut newest = TopK::new(Ranking::Newest, 3);
        newest.push(item("new.txt", 1, t(300)));
        newest.push(item("old.txt", 1, t(100)));
        newest.push(item("mid.txt", 1, t(200)));
        let list = newest.into_sorted();
        let paths: Vec<&str> = list.iter().map(|i| i.relative_path.as_str()).collect();
        assert_eq!(paths, vec!["new.txt", "mid.txt", "old.txt"]);
    }

    #[test]
    fn unknown_modified_time_is_excluded_from_time_rankings_not_faked() {
        let mut oldest = TopK::new(Ranking::Oldest, 5);
        oldest.push(item("unknown.txt", 1, None));
        oldest.push(item("known.txt", 1, t(100)));
        let list = oldest.into_sorted();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].relative_path, "known.txt");
    }

    #[test]
    fn capacity_is_bounded() {
        let mut top = TopK::new(Ranking::Largest, 2);
        for i in 0..100 {
            top.push(item(&format!("f{i}.txt"), i, t(1)));
        }
        let list = top.into_sorted();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].relative_path, "f99.txt");
    }

    #[test]
    fn case_insensitive_path_tiebreak_is_deterministic() {
        let mut top = TopK::new(Ranking::Largest, 2);
        top.push(item("B.txt", 5, t(1)));
        top.push(item("a.txt", 5, t(1)));
        let list = top.into_sorted();
        assert_eq!(list[0].relative_path, "a.txt");
    }
}
