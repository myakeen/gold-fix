use std::collections::HashMap;
use super::Field;
use crate::error::FixError;
use crate::Result;

/// Represents a repeating group in a FIX message
#[derive(Debug, Clone)]
pub struct RepeatingGroup {
    /// Number of entries in the group
    pub count: usize,
    /// Fields for each entry in the group
    entries: Vec<HashMap<i32, Field>>,
    /// Delimited field tag that starts each entry
    delimiter_tag: i32,
    /// Required field tags for each entry
    required_tags: Vec<i32>,
}

impl RepeatingGroup {
    /// Create a new repeating group
    pub fn new(delimiter_tag: i32, required_tags: Vec<i32>) -> Self {
        RepeatingGroup {
            count: 0,
            entries: Vec::new(),
            delimiter_tag,
            required_tags,
        }
    }

    /// Add a new entry to the group
    pub fn add_entry(&mut self) -> &mut HashMap<i32, Field> {
        self.entries.push(HashMap::new());
        self.count += 1;
        self.entries.last_mut().unwrap()
    }

    /// Get a field from a specific entry by position
    pub fn get_field_at(&self, tag: i32, position: usize) -> Option<&Field> {
        self.entries.get(position).and_then(|entry| entry.get(&tag))
    }

    /// Set a field in a specific entry
    pub fn set_field_at(&mut self, position: usize, field: Field) -> Result<()> {
        if position >= self.count {
            return Err(FixError::ParseError(format!(
                "Invalid group entry position: {}", position
            )));
        }
        self.entries[position].insert(field.tag(), field);
        Ok(())
    }

    /// Check if this group handles a specific field tag
    pub fn has_field(&self, tag: i32) -> bool {
        tag == self.delimiter_tag || self.required_tags.contains(&tag)
    }

    /// Validate the group structure
    pub fn validate(&self) -> Result<()> {
        // Check group count matches actual entries
        if self.entries.len() != self.count {
            return Err(FixError::ParseError(format!(
                "Group count mismatch: expected {}, found {}",
                self.count, self.entries.len()
            )));
        }

        // Validate each entry has required fields
        for (i, entry) in self.entries.iter().enumerate() {
            // Check delimiter tag
            if !entry.contains_key(&self.delimiter_tag) {
                return Err(FixError::ParseError(format!(
                    "Missing delimiter tag {} in entry {}", self.delimiter_tag, i
                )));
            }

            // Check other required tags
            for &tag in &self.required_tags {
                if !entry.contains_key(&tag) {
                    return Err(FixError::ParseError(format!(
                        "Missing required tag {} in entry {}", tag, i
                    )));
                }
            }
        }
        Ok(())
    }

    /// Get the number of entries in the group
    pub fn entry_count(&self) -> usize {
        self.count
    }

    /// Get an iterator over the entries
    pub fn entries(&self) -> impl Iterator<Item = &HashMap<i32, Field>> {
        self.entries.iter()
    }

    /// Get a mutable iterator over the entries
    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut HashMap<i32, Field>> {
        self.entries.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::field;

    #[test]
    fn test_repeating_group_creation() {
        let group = RepeatingGroup::new(
            field::MD_ENTRY_TYPE,
            vec![field::MD_ENTRY_PX, field::MD_ENTRY_SIZE],
        );
        assert_eq!(group.entry_count(), 0);
    }

    #[test]
    fn test_add_entries() {
        let mut group = RepeatingGroup::new(
            field::MD_ENTRY_TYPE,
            vec![field::MD_ENTRY_PX, field::MD_ENTRY_SIZE],
        );

        // Add first entry
        {
            let entry = group.add_entry();
            entry.insert(field::MD_ENTRY_TYPE, Field::new(field::MD_ENTRY_TYPE, "0"));
            entry.insert(field::MD_ENTRY_PX, Field::new(field::MD_ENTRY_PX, "100.00"));
            entry.insert(field::MD_ENTRY_SIZE, Field::new(field::MD_ENTRY_SIZE, "500"));
        }

        assert_eq!(group.entry_count(), 1);
        assert!(group.validate().is_ok());
    }

    #[test]
    fn test_group_validation() {
        let mut group = RepeatingGroup::new(
            field::MD_ENTRY_TYPE,
            vec![field::MD_ENTRY_PX, field::MD_ENTRY_SIZE],
        );

        // Add incomplete entry
        {
            let entry = group.add_entry();
            entry.insert(field::MD_ENTRY_TYPE, Field::new(field::MD_ENTRY_TYPE, "0"));
            // Missing MD_ENTRY_PX and MD_ENTRY_SIZE
        }

        assert!(group.validate().is_err());
    }

    #[test]
    fn test_get_field_at() {
        let mut group = RepeatingGroup::new(
            field::MD_ENTRY_TYPE,
            vec![field::MD_ENTRY_PX, field::MD_ENTRY_SIZE],
        );

        // Add an entry
        {
            let entry = group.add_entry();
            entry.insert(field::MD_ENTRY_TYPE, Field::new(field::MD_ENTRY_TYPE, "0"));
            entry.insert(field::MD_ENTRY_PX, Field::new(field::MD_ENTRY_PX, "100.00"));
            entry.insert(field::MD_ENTRY_SIZE, Field::new(field::MD_ENTRY_SIZE, "500"));
        }

        assert_eq!(
            group.get_field_at(field::MD_ENTRY_PX, 0).unwrap().value(),
            "100.00"
        );
    }
}