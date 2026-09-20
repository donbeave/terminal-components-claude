//! Tiny TSV reader for frozen adapter inventories.

/// Parsed TSV table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table<'a> {
    /// Column names.
    pub header: Vec<&'a str>,
    /// Row fields aligned to `header`.
    pub rows: Vec<Vec<&'a str>>,
}

impl<'a> Table<'a> {
    /// Parse a TSV document. Empty trailing lines are ignored.
    #[must_use]
    pub fn parse(text: &'a str) -> Option<Self> {
        let mut lines = text.lines().filter(|line| !line.is_empty());
        let header: Vec<&str> = lines.next()?.split('\t').collect();
        if header.is_empty() {
            return None;
        }
        let width = header.len();
        let mut rows = Vec::new();
        for line in lines {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != width {
                return None;
            }
            rows.push(fields);
        }
        Some(Self { header, rows })
    }

    /// Named field of `row`.
    #[must_use]
    pub fn get(&self, row: usize, name: &str) -> Option<&'a str> {
        let column = self.header.iter().position(|item| *item == name)?;
        self.rows.get(row)?.get(column).copied()
    }

    /// Number of data rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the table has no data rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}
