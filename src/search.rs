use crate::fs::ProjectFile;
use nucleo_matcher::{pattern::Pattern, Config, Matcher, Utf32String};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub struct RecentFiles {
    items: VecDeque<String>,
}

impl RecentFiles {
    pub fn record(&mut self, relative: impl Into<String>) {
        let relative = relative.into();
        self.items.retain(|item| item != &relative);
        self.items.push_front(relative);
        self.items.truncate(32);
    }

    pub fn rank(&self, relative: &str) -> Option<usize> {
        self.items.iter().position(|item| item == relative)
    }
}

#[derive(Debug, Clone)]
pub struct SearchIndex {
    files: Vec<ProjectFile>,
}

impl SearchIndex {
    pub fn new(files: Vec<ProjectFile>) -> Self {
        Self { files }
    }

    pub fn query(&self, query: &str, recent: &RecentFiles) -> Vec<ProjectFile> {
        if query.trim().is_empty() {
            return self.files.clone();
        }
        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(query, nucleo_matcher::pattern::CaseMatching::Smart, nucleo_matcher::pattern::Normalization::Smart);
        let mut scored: Vec<(i64, ProjectFile)> = self
            .files
            .iter()
            .filter_map(|file| {
                let haystack = Utf32String::from(file.relative.as_str());
                pattern.score(haystack.slice(..), &mut matcher).map(|score| {
                    let boost = recent.rank(&file.relative).map(|rank| 10_000 - rank as i64).unwrap_or(0);
                    (score as i64 + boost, file.clone())
                })
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.relative.cmp(&b.1.relative)));
        scored.into_iter().map(|(_, file)| file).collect()
    }
}
