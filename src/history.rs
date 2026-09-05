use crate::font_data::FontProject;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub project: FontProject,
    pub current_glyph: Option<String>,
}

#[derive(Debug)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
    pub current_index: usize,
    max_entries: usize,
}

impl History {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            current_index: 0,
            max_entries,
        }
    }

    pub fn push(&mut self, project: &FontProject, current_glyph: &Option<String>) {
        if self.max_entries == 0 {
            self.entries.clear();
            self.current_index = 0;
            return;
        }
        if let Some(last) = self.entries.get(self.current_index) {
            if last.project == *project && last.current_glyph == *current_glyph {
                return;
            }
        }
        let entry = HistoryEntry {
            project: project.clone(),
            current_glyph: current_glyph.clone(),
        };

        // Remove any entries after current index (we're branching)
        if !self.entries.is_empty() {
            self.entries.truncate(self.current_index + 1);
        }
        self.entries.push(entry);

        // Limit history size
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.current_index = self.entries.len().saturating_sub(1);
    }

    pub fn undo(&mut self) -> Option<&HistoryEntry> {
        if self.current_index > 0 {
            self.current_index -= 1;
            Some(&self.entries[self.current_index])
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&HistoryEntry> {
        if self.current_index + 1 < self.entries.len() {
            self.current_index += 1;
            Some(&self.entries[self.current_index])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(n: usize) -> (FontProject, Option<String>) {
        let mut project = FontProject::new();
        project.add_glyph(format!("g{n}"), None);
        (project, None)
    }

    #[test]
    fn empty_history_can_redo_is_safe() {
        let mut history = History::new(2);
        assert!(history.redo().is_none());
    }

    #[test]
    fn bounded_history_keeps_latest_entries() {
        let mut history = History::new(2);
        for n in 0..3 {
            let (project, current) = state(n);
            history.push(&project, &current);
        }
        assert_eq!(history.entries.len(), 2);
        assert!(history.undo().is_some());
        assert!(history.undo().is_none());
    }

    #[test]
    fn identical_snapshots_are_not_added_twice() {
        let mut history = History::new(10);
        let (project, current) = state(0);
        history.push(&project, &current);
        history.push(&project, &current);
        assert_eq!(history.entries.len(), 1);
    }

    #[test]
    fn pushing_after_undo_discards_redo_branch() {
        let mut history = History::new(10);
        let first = state(0);
        let second = state(1);
        let branch = state(2);
        history.push(&first.0, &first.1);
        history.push(&second.0, &second.1);
        assert!(history.undo().is_some());
        history.push(&branch.0, &branch.1);
        assert!(history.redo().is_none());
        assert_eq!(
            history.entries.last().unwrap().project.glyph_names_sorted(),
            vec!["g2"]
        );
    }

    #[test]
    fn zero_capacity_history_is_safe() {
        let mut history = History::new(0);
        let (project, current) = state(0);
        history.push(&project, &current);
        assert!(history.entries.is_empty());
        assert!(history.undo().is_none());
        assert!(history.redo().is_none());
    }
}
