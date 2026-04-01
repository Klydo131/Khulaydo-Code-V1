use crate::note::Note;
use dirs::data_dir;
use std::fs;
use std::path::PathBuf;

pub struct NoteStore {
    notes: Vec<Note>,
    path: PathBuf,
}

impl NoteStore {
    pub fn load() -> Result<Self, String> {
        let path = store_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let notes = if path.exists() {
            let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&data).map_err(|e| e.to_string())?
        } else {
            vec![]
        };

        Ok(Self { notes, path })
    }

    pub fn save(&self) -> Result<(), String> {
        let data = serde_json::to_string_pretty(&self.notes).map_err(|e| e.to_string())?;
        fs::write(&self.path, data).map_err(|e| e.to_string())
    }

    pub fn add(&mut self, note: Note) {
        self.notes.push(note);
    }

    pub fn all(&self) -> Vec<&Note> {
        self.notes.iter().collect()
    }

    pub fn all_mut(&mut self) -> Vec<Note> {
        self.notes.clone()
    }

    pub fn replace_all(&mut self, notes: Vec<Note>) {
        self.notes = notes;
    }

    pub fn list(&self, tag: Option<&str>) -> Vec<&Note> {
        match tag {
            Some(t) => self.notes.iter().filter(|n| n.tags.iter().any(|tag| tag == t)).collect(),
            None => self.notes.iter().collect(),
        }
    }

    pub fn find(&self, query: &str) -> Option<&Note> {
        // Try exact ID match first
        if let Some(n) = self.notes.iter().find(|n| n.id == query) {
            return Some(n);
        }
        // Try ID prefix match
        if let Some(n) = self.notes.iter().find(|n| n.id.starts_with(query)) {
            return Some(n);
        }
        // Try title contains match (case-insensitive)
        let lower = query.to_lowercase();
        self.notes.iter().find(|n| n.title.to_lowercase().contains(&lower))
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let before = self.notes.len();
        self.notes.retain(|n| !n.id.starts_with(id) && n.id != id);
        self.notes.len() < before
    }
}

fn store_path() -> PathBuf {
    data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("note-organizer")
        .join("notes.json")
}
