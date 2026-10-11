//! One forest index per validation; ancestor queries use DFS intervals.
use crate::protocol::SessionEntry;
use std::collections::HashMap;

pub(super) struct Index<'a> {
    entries: &'a [SessionEntry],
    ids: HashMap<&'a str, usize>,
    intervals: Vec<(usize, usize)>,
}
impl<'a> Index<'a> {
    /// IDs and append-order parents have already been checked by validate.
    pub(super) fn new(entries: &'a [SessionEntry]) -> Self {
        let ids: HashMap<_, _> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.as_str(), i))
            .collect();
        let mut children = vec![Vec::new(); entries.len()];
        let mut roots = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            if let Some(parent) = &entry.parent_id {
                children[ids[parent.as_str()]].push(i);
            } else {
                roots.push(i);
            }
        }
        let mut intervals = vec![(0, 0); entries.len()];
        let mut clock = 0;
        let mut stack: Vec<_> = roots.into_iter().map(|i| (i, false)).collect();
        while let Some((i, leaving)) = stack.pop() {
            if leaving {
                intervals[i].1 = clock;
            } else {
                intervals[i].0 = clock;
                clock += 1;
                stack.push((i, true));
                stack.extend(children[i].iter().map(|&child| (child, false)));
            }
        }
        Self {
            entries,
            ids,
            intervals,
        }
    }
    pub(super) fn on_branch(&self, target: &str, leaf: Option<&str>) -> Option<&'a SessionEntry> {
        let target = *self.ids.get(target)?;
        let leaf = *self.ids.get(leaf?)?;
        let (start, end) = self.intervals[target];
        (start <= self.intervals[leaf].0 && self.intervals[leaf].0 < end)
            .then_some(&self.entries[target])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::EntryData;
    #[test]
    fn intervals_match_parent_walks_across_branches_and_roots() {
        let entries: Vec<_> = (0..200)
            .map(|i| SessionEntry {
                id: i.to_string(),
                parent_id: if i % 17 == 0 {
                    None
                } else {
                    Some(((i - 1) / 2).to_string())
                },
                timestamp: "t".into(),
                data: EntryData::SessionInfo { name: None },
            })
            .collect();
        let index = Index::new(&entries);
        for target in &entries {
            assert!(index.on_branch(&target.id, None).is_none());
            for leaf in &entries {
                let mut cursor = Some(leaf);
                let mut expected = false;
                while let Some(entry) = cursor {
                    if entry.id == target.id {
                        expected = true;
                        break;
                    }
                    cursor = entry
                        .parent_id
                        .as_deref()
                        .and_then(|id| index.ids.get(id).map(|&i| &entries[i]));
                }
                assert_eq!(
                    index.on_branch(&target.id, Some(&leaf.id)).is_some(),
                    expected
                );
            }
        }
    }
}
