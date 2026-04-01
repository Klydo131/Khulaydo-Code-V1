mod api;
mod note;
mod agents;
mod store;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::env;

#[derive(Parser)]
#[command(name = "note-organizer")]
#[command(about = "AI-powered note organizer using Claude agents")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new note
    Add {
        /// Note title
        #[arg(short, long)]
        title: String,
        /// Note content
        #[arg(short, long)]
        content: String,
    },
    /// List all notes
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },
    /// Show a note by ID or title fragment
    Show {
        /// Note ID or title fragment
        query: String,
    },
    /// Search notes using AI semantic search
    Search {
        /// Search query
        query: String,
    },
    /// Organize all notes with AI (auto-tag, categorize, summarize)
    Organize,
    /// Ask the AI assistant about your notes
    Ask {
        /// Your question
        question: String,
    },
    /// Delete a note by ID
    Delete {
        /// Note ID
        id: String,
    },
    /// Show stats about your notes collection
    Stats,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("{}", "Error: ANTHROPIC_API_KEY environment variable not set.".red().bold());
        eprintln!("Set it with: export ANTHROPIC_API_KEY=your-key-here");
        std::process::exit(1);
    }

    let store = store::NoteStore::load().expect("Failed to load note store");

    match cli.command {
        Commands::Add { title, content } => {
            cmd_add(store, title, content).await;
        }
        Commands::List { tag } => {
            cmd_list(store, tag);
        }
        Commands::Show { query } => {
            cmd_show(store, query);
        }
        Commands::Search { query } => {
            cmd_search(store, query).await;
        }
        Commands::Organize => {
            cmd_organize(store).await;
        }
        Commands::Ask { question } => {
            cmd_ask(store, question).await;
        }
        Commands::Delete { id } => {
            cmd_delete(store, id);
        }
        Commands::Stats => {
            cmd_stats(store);
        }
    }
}

async fn cmd_add(mut store: store::NoteStore, title: String, content: String) {
    println!("{}", "Analyzing your note with AI...".cyan());
    let note = agents::tagger_agent(&title, &content).await
        .unwrap_or_else(|e| {
            eprintln!("{}: {}", "Warning: AI tagging failed".yellow(), e);
            note::Note::new(title, content, vec![], None, None)
        });

    println!("{} {}", "Category:".dimmed(), note.category.as_deref().unwrap_or("uncategorized").blue());
    println!("{} {}", "Tags:".dimmed(), note.tags.join(", ").green());
    if let Some(ref summary) = note.summary {
        println!("{} {}", "Summary:".dimmed(), summary.italic());
    }

    let id = note.id.clone();
    store.add(note);
    store.save().expect("Failed to save note store");
    println!("{} Note saved with ID: {}", "✓".green().bold(), id.yellow());
}

fn cmd_list(store: store::NoteStore, tag: Option<String>) {
    let notes = store.list(tag.as_deref());
    if notes.is_empty() {
        println!("{}", "No notes found.".dimmed());
        return;
    }
    println!("{}", format!("Found {} note(s):", notes.len()).bold());
    println!();
    for n in notes {
        let tags_display = if n.tags.is_empty() {
            "no tags".dimmed().to_string()
        } else {
            n.tags.join(", ").green().to_string()
        };
        println!(
            "{} {} [{}]",
            n.id[..8].yellow(),
            n.title.bold(),
            tags_display
        );
        if let Some(ref summary) = n.summary {
            println!("   {}", summary.italic().dimmed());
        }
        println!(
            "   {} {}",
            "Category:".dimmed(),
            n.category.as_deref().unwrap_or("none").blue()
        );
        println!();
    }
}

fn cmd_show(store: store::NoteStore, query: String) {
    match store.find(&query) {
        Some(n) => {
            println!("{}", "─".repeat(60).dimmed());
            println!("{} {}", "ID:".dimmed(), n.id.yellow());
            println!("{} {}", "Title:".dimmed(), n.title.bold());
            println!("{} {}", "Created:".dimmed(), n.created_at.format("%Y-%m-%d %H:%M").to_string().dimmed());
            println!("{} {}", "Category:".dimmed(), n.category.as_deref().unwrap_or("none").blue());
            println!("{} {}", "Tags:".dimmed(), if n.tags.is_empty() { "none".to_string() } else { n.tags.join(", ").green().to_string() });
            if let Some(ref summary) = n.summary {
                println!("{} {}", "Summary:".dimmed(), summary.italic());
            }
            println!("{}", "─".repeat(60).dimmed());
            println!("{}", n.content);
        }
        None => {
            println!("{}", format!("No note found matching: {}", query).red());
        }
    }
}

async fn cmd_search(store: store::NoteStore, query: String) {
    println!("{}", "Searching notes with AI...".cyan());
    let notes = store.all();
    if notes.is_empty() {
        println!("{}", "No notes to search.".dimmed());
        return;
    }
    match agents::search_agent(&query, &notes).await {
        Ok(results) => {
            if results.is_empty() {
                println!("{}", "No matching notes found.".dimmed());
            } else {
                println!("{}", format!("Found {} relevant note(s):", results.len()).bold());
                for (note, explanation) in results {
                    println!();
                    println!("{} {}", note.id[..8].yellow(), note.title.bold());
                    println!("   {}", explanation.italic().dimmed());
                }
            }
        }
        Err(e) => eprintln!("{}: {}", "Search failed".red(), e),
    }
}

async fn cmd_organize(mut store: store::NoteStore) {
    let notes = store.all_mut();
    if notes.is_empty() {
        println!("{}", "No notes to organize.".dimmed());
        return;
    }
    println!("{}", format!("Organizing {} note(s) with AI...", notes.len()).cyan());
    match agents::organizer_agent(notes).await {
        Ok(updated) => {
            let count = updated.len();
            store.replace_all(updated);
            store.save().expect("Failed to save");
            println!("{} Organized {} note(s) with updated tags, categories, and summaries.", "✓".green().bold(), count);
        }
        Err(e) => eprintln!("{}: {}", "Organization failed".red(), e),
    }
}

async fn cmd_ask(store: store::NoteStore, question: String) {
    let notes = store.all();
    println!("{}", "Consulting AI assistant...".cyan());
    match agents::qa_agent(&question, &notes).await {
        Ok(answer) => {
            println!();
            println!("{}", answer);
        }
        Err(e) => eprintln!("{}: {}", "Failed".red(), e),
    }
}

fn cmd_delete(mut store: store::NoteStore, id: String) {
    if store.delete(&id) {
        store.save().expect("Failed to save");
        println!("{} Note deleted.", "✓".green().bold());
    } else {
        println!("{}", format!("No note found with ID: {}", id).red());
    }
}

fn cmd_stats(store: store::NoteStore) {
    let notes = store.all();
    let total = notes.len();
    if total == 0 {
        println!("{}", "No notes yet.".dimmed());
        return;
    }

    let mut categories: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut all_tags: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for n in &notes {
        let cat = n.category.clone().unwrap_or_else(|| "uncategorized".to_string());
        *categories.entry(cat).or_insert(0) += 1;
        for tag in &n.tags {
            *all_tags.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    println!("{}", "─".repeat(40).dimmed());
    println!("{}", "Note Statistics".bold());
    println!("{}", "─".repeat(40).dimmed());
    println!("{} {}", "Total notes:".dimmed(), total.to_string().yellow().bold());
    println!();
    println!("{}", "Categories:".bold());
    let mut cats: Vec<_> = categories.iter().collect();
    cats.sort_by(|a, b| b.1.cmp(a.1));
    for (cat, count) in cats {
        println!("  {} {}", format!("{}", count).yellow(), cat.blue());
    }
    println!();
    println!("{}", "Top Tags:".bold());
    let mut tags: Vec<_> = all_tags.iter().collect();
    tags.sort_by(|a, b| b.1.cmp(a.1));
    for (tag, count) in tags.iter().take(10) {
        println!("  {} {}", format!("{}", count).yellow(), tag.green());
    }
}
