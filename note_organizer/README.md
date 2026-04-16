# Klyde Reader

A terminal book reader for `Klyde.md`. No internet, no API keys, no cost — runs entirely offline.

## Build & run

```bash
cargo build --release
cp target/release/klyde-reader .
./klyde-reader
```

`Klyde.md` must be in the current directory when you run the binary.

## Controls

| Key | Action |
|-----|--------|
| ↓ / j / Space / Enter | Next page |
| ↑ / k / Backspace | Previous page |
| g / Home | First page |
| G / End | Last page |
| q / Esc | Quit |

## Writing your book

Edit `Klyde.md` in any text editor. Standard Markdown is supported:

- `# Heading 1` — rendered in cyan bold
- `## Heading 2` — rendered in yellow bold
- `### Heading 3` — rendered in green
- `**bold**` — rendered bold
- `` `code` `` — rendered in magenta
- ` ``` ` fenced code blocks
- `---` horizontal rules
