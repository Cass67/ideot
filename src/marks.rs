#[derive(Debug, Clone, Default)]
pub struct SessionMarks {
    slots: [Option<String>; 9],
}

impl SessionMarks {
    pub fn mark(&mut self, relative: impl Into<String>) -> usize {
        let relative = relative.into();
        if let Some(index) = self.slots.iter().position(|item| item.as_deref() == Some(relative.as_str())) {
            return index + 1;
        }
        let index = self.slots.iter().position(Option::is_none).unwrap_or(0);
        self.slots[index] = Some(relative);
        index + 1
    }

    pub fn get(&self, slot: usize) -> Option<&String> {
        if !(1..=9).contains(&slot) {
            return None;
        }
        self.slots[slot - 1].as_ref()
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, &String)> {
        self.slots.iter().enumerate().filter_map(|(index, value)| value.as_ref().map(|path| (index + 1, path)))
    }
}
