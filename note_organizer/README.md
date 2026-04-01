# note-organizer

An AI-powered CLI note organizer built in Rust, using Claude as the AI backend.

## Features

- **Auto-tagging** — Claude automatically assigns tags, a category, and a one-sentence summary when you add a note
- **Semantic search** — Ask in plain English; Claude finds the relevant notes
- **Batch organize** — Re-tag and re-categorize your entire collection for consistent taxonomy
- **Q&A assistant** — Ask questions about your notes in natural language
- **Local storage** — Notes saved as JSON in your OS data directory (no cloud sync)

## Setup

```bash
export ANTHROPIC_API_KEY=your-key-here
cargo build --release
./target/release/note-organizer --help
```

## Usage

```bash
# Add a note (AI auto-tags it)
note-organizer add --title "Meeting notes" --content "Discussed Q3 roadmap..."

# List all notes
note-organizer list

# Filter by tag
note-organizer list --tag project

# Show a note (by ID prefix or title fragment)
note-organizer show "Meeting"

# Semantic search
note-organizer search "what did I write about the roadmap?"

# Ask the AI assistant
note-organizer ask "summarize my project notes"

# Re-organize all notes with consistent AI taxonomy
note-organizer organize

# Stats
note-organizer stats

# Delete
note-organizer delete <id>
```

## Categories

`work` | `personal` | `learning` | `project` | `health` | `finance` | `ideas` | `other`

## Storage

Notes are stored at:
- Linux: `~/.local/share/note-organizer/notes.json`
- macOS: `~/Library/Application Support/note-organizer/notes.json`
- Windows: `%APPDATA%\note-organizer\notes.json`
