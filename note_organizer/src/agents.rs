/// AI agents for note organization tasks.
///
/// Each agent is a focused Claude call with a specific system prompt
/// and structured output. Agents parse JSON responses to extract
/// structured data like tags, categories, and summaries.

use crate::api::{self, ApiError};
use crate::note::Note;
use serde_json::Value;

// ─── Tagger Agent ────────────────────────────────────────────────────────────
//
// When a note is added, the tagger agent reads the title + content and
// returns tags, a category, and a one-sentence summary.

const TAGGER_SYSTEM: &str = r#"You are a note-tagging assistant. Given a note title and content,
extract metadata and return ONLY a valid JSON object (no markdown, no explanation) with this exact shape:

{
  "tags": ["tag1", "tag2", "tag3"],
  "category": "work|personal|learning|project|health|finance|ideas|other",
  "summary": "One concise sentence summarizing the note."
}

Rules:
- tags: 2-6 lowercase, hyphenated keywords that best describe the content
- category: pick the single best fit from the provided options
- summary: one sentence, max 120 characters
- Return ONLY the JSON object, nothing else"#;

pub async fn tagger_agent(title: &str, content: &str) -> Result<Note, ApiError> {
    let prompt = format!("Title: {}\n\nContent:\n{}", title, content);
    let raw = api::ask(TAGGER_SYSTEM, &prompt, 512).await?;

    let json: Value = parse_json(&raw)
        .map_err(|e| ApiError::Parse(format!("Tagger response not valid JSON: {} | raw: {}", e, raw)))?;

    let tags: Vec<String> = json["tags"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let category = json["category"].as_str().map(String::from);
    let summary = json["summary"].as_str().map(String::from);

    Ok(Note::new(title.to_string(), content.to_string(), tags, category, summary))
}

// ─── Organizer Agent ─────────────────────────────────────────────────────────
//
// Batch re-tags and re-categorizes all notes at once to apply consistent
// taxonomy across the entire collection.

const ORGANIZER_SYSTEM: &str = r#"You are a note organization assistant. Given a list of notes,
return ONLY a valid JSON array (no markdown, no explanation) where each element has:

{
  "id": "<original note id>",
  "tags": ["tag1", "tag2"],
  "category": "work|personal|learning|project|health|finance|ideas|other",
  "summary": "One concise sentence."
}

Apply consistent tagging across all notes. Return ONLY the JSON array."#;

pub async fn organizer_agent(notes: Vec<Note>) -> Result<Vec<Note>, ApiError> {
    if notes.is_empty() {
        return Ok(vec![]);
    }

    // Build a compact representation of all notes for the prompt
    let notes_text: String = notes
        .iter()
        .map(|n| format!("ID: {}\nTitle: {}\nContent: {}\n---", n.id, n.title, &n.content[..n.content.len().min(400)]))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!("Organize these notes:\n\n{}", notes_text);
    let raw = api::ask(ORGANIZER_SYSTEM, &prompt, 2048).await?;

    let json: Value = parse_json(&raw)
        .map_err(|e| ApiError::Parse(format!("Organizer response not valid JSON: {} | raw: {}", e, raw)))?;

    let updates = json.as_array()
        .ok_or_else(|| ApiError::Parse("Expected JSON array from organizer".to_string()))?;

    let updated_notes: Vec<Note> = notes
        .into_iter()
        .map(|mut note| {
            if let Some(update) = updates.iter().find(|u| u["id"].as_str() == Some(&note.id)) {
                note.tags = update["tags"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or(note.tags);
                note.category = update["category"].as_str().map(String::from).or(note.category);
                note.summary = update["summary"].as_str().map(String::from).or(note.summary);
                note.updated_at = chrono::Utc::now();
            }
            note
        })
        .collect();

    Ok(updated_notes)
}

// ─── Search Agent ─────────────────────────────────────────────────────────────
//
// Uses Claude to semantically match a natural-language query against all notes,
// returning the most relevant ones with a brief explanation.

const SEARCH_SYSTEM: &str = r#"You are a semantic search assistant for personal notes.
Given a search query and a list of notes, return ONLY a valid JSON array of the most relevant notes
(no markdown, no explanation):

[
  {"id": "<note id>", "explanation": "Why this note matches the query."}
]

Return an empty array [] if nothing matches. Be selective — only include genuinely relevant notes.
Return ONLY the JSON array."#;

pub async fn search_agent<'a>(
    query: &str,
    notes: &'a [&Note],
) -> Result<Vec<(&'a Note, String)>, ApiError> {
    if notes.is_empty() {
        return Ok(vec![]);
    }

    let notes_text: String = notes
        .iter()
        .map(|n| {
            format!(
                "ID: {}\nTitle: {}\nTags: {}\nContent: {}\n---",
                n.id,
                n.title,
                n.tags.join(", "),
                &n.content[..n.content.len().min(300)]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!("Query: {}\n\nNotes:\n{}", query, notes_text);
    let raw = api::ask(SEARCH_SYSTEM, &prompt, 1024).await?;

    let json: Value = parse_json(&raw)
        .map_err(|e| ApiError::Parse(format!("Search response not valid JSON: {} | raw: {}", e, raw)))?;

    let matches = json.as_array()
        .ok_or_else(|| ApiError::Parse("Expected JSON array from search".to_string()))?;

    let results: Vec<(&Note, String)> = matches
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?;
            let explanation = m["explanation"].as_str().unwrap_or("").to_string();
            let note = notes.iter().find(|n| n.id == id)?;
            Some((*note, explanation))
        })
        .collect();

    Ok(results)
}

// ─── Q&A Agent ────────────────────────────────────────────────────────────────
//
// Answers free-form questions about the note collection.

const QA_SYSTEM: &str = r#"You are a helpful personal assistant with access to the user's notes.
Answer questions about their notes accurately and concisely.
If the answer is not in their notes, say so honestly.
Be conversational and helpful."#;

pub async fn qa_agent(question: &str, notes: &[&Note]) -> Result<String, ApiError> {
    let notes_context = if notes.is_empty() {
        "The user has no notes yet.".to_string()
    } else {
        notes
            .iter()
            .map(|n| {
                format!(
                    "Title: {}\nCategory: {}\nTags: {}\nContent: {}\n",
                    n.title,
                    n.category.as_deref().unwrap_or("none"),
                    n.tags.join(", "),
                    &n.content[..n.content.len().min(500)]
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n")
    };

    let prompt = format!(
        "Here are the user's notes:\n\n{}\n\nUser question: {}",
        notes_context, question
    );

    api::ask(QA_SYSTEM, &prompt, 1024).await
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Parse JSON, stripping any markdown code fences if present.
fn parse_json(raw: &str) -> Result<Value, serde_json::Error> {
    let cleaned = raw.trim();
    // Strip ```json ... ``` or ``` ... ``` fences
    let cleaned = if cleaned.starts_with("```") {
        cleaned
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        cleaned
    };
    serde_json::from_str(cleaned)
}
