# rp6-tui

Interactive cheatsheet for the Roland P-6 sampler. It runs in the terminal, and in the
browser through [Ratzilla](https://github.com/orhun/ratzilla) (ratatui on WebAssembly).

## Development

The dev shell is managed by [devenv](https://devenv.sh) (`direnv allow`, or `devenv shell`):

| Command                 | What it does                                          |
| ----------------------- | ----------------------------------------------------- |
| `tui`                   | Run in the terminal (`cargo run --release`)           |
| `web`                   | Serve the web version with live reload (`trunk serve`) |
| `web-build [/path/]`    | Build the static site into `dist/`                    |

## Layout

- `src/lib.rs` – app state and key handling, using its own `Key` type
- `src/ui.rs`, `src/unit.rs` – drawing (shared by both front ends)
- `src/main.rs` – front ends: crossterm (native) and Ratzilla (`wasm32`)
- `data/p6.toml` – the cheatsheet data, compiled in
- `index.html` – the page Trunk builds the web version from

## Deploying

`.github/workflows/pages.yml` builds the site on every push to `main` and publishes it to
GitHub Pages. In the repository settings, set **Pages → Source** to **GitHub Actions**.
The site is served at `https://<user>.github.io/<repo>/`.
