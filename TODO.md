# ✅ NeoPilot Terminal — Full Feature Roadmap & To-Do

> A complete modern terminal architecture with AI, workflows, plugin support, language adaptation, and cloud sync.

---

## 🧱 Core Engine

- [x] `command/`: Shell lifecycle & PTY I/O *(scaffolded)*
- [x] `string_offset/`: Unicode-aware text slicing/indexing *(scaffolded)*
- [x] `sum_tree/`: Undo/redo history with tree structure *(scaffolded)*
- [x] `syntax_tree/`: Shell/code syntax parser *(scaffolded)*
- [x] `virtual_fs/`: Sandboxed command execution *(scaffolded)*
- [x] `watcher/`: File system + command runtime monitoring *(scaffolded)*

---

## 🖥️ UI + Terminal Features

- [x] Command blocks (status: Running, Done, Error) *(implemented in block.rs)*
- [x] Input history (↑ ↓) *(implemented in input.rs)*
- [x] Scrollable block view *(implemented in renderer.rs)*
- [ ] Collapsible output blocks
- [ ] Command palette (⌘K)
- [x] Fuzzy finder for files/commands *(scaffolded)*
- [ ] Tabbed sessions
- [x] Custom themes (light/dark/custom) *(implemented with YAML themes)*
- [ ] Rounded corners, font config, padding
- [ ] GPU acceleration with `wgpu`
- [ ] Notifications system

---

## 🔍 Search + Interaction

- [x] `fuzzy_match/`: Command & block fuzzy matching *(scaffolded)*
- [x] `mcq/`: Multi-choice prompts (fuzzy UI) *(scaffolded)*
- [x] `markdown_parser/`: Markdown/rich text block output *(scaffolded)*
- [x] Keybinding remapper *(implemented in settings/keybinding_editor.rs)*
- [ ] File explorer panel
- [ ] Block jump (Ctrl+J / Ctrl+K)

---

## 🧠 AI Integration

- [x] `agent_mode_eval/`: Wrap terminal context into AI task *(scaffolded)*
- [x] `lpc/`: Cross-shell command translation (bash ↔ pwsh) *(scaffolded)*
- [x] `languages/`: Shell/language detection + switching *(scaffolded)*
- [x] `natural_language_detection/`: Detect & adapt user input language *(scaffolded)*
- [ ] AI Assistant sidebar (OpenAI, Claude, or local model)
- [ ] "Explain this output" button
- [ ] AI command auto-fix + smart suggestions
- [ ] Local model support (`ollama`, `llama.cpp`)
- [ ] Context injection: cwd, env, history

---

## 📦 Workflows & Cloud Sync

- [x] `asset_macro/`: Reusable command macros/workflows *(scaffolded)*
- [x] `drive/`: Warp Drive clone — store workflows & prefs *(scaffolded)*
- [x] `graphql/`: Expose local workflows via API *(scaffolded)*
- [x] Workflow manager: create/edit/execute/save *(implemented in workflows/)*
- [x] YAML workflow definitions *(implemented with sample workflows)*
- [ ] Team workflow sharing
- [ ] SQLite + cloud sync (Supabase or Firebase)

---

## 🧩 Plugins & Extensibility

- [x] `serve_wasm/`: WASM-based plugin runtime *(scaffolded)*
- [ ] Plugin manifest format (JSON)
- [ ] Plugin manager sidebar
- [ ] `mlua`: Lua scripting engine for plugins
- [ ] Runtime hooks: pre-command, post-output
- [ ] Hot reload plugin system
- [x] `resources/`: Icons, manifests, plugin assets *(scaffolded)*

---

## 🌐 Integrations

- [x] `integration/`: Git, Docker, SSH, GitHub CLI *(scaffolded)*
- [x] `websocket/`: Real-time bi-directional events *(scaffolded)*
- [ ] SSH session manager
- [ ] Remote session support
- [ ] Terminal state sync across devices

---

## 🎨 Theming & Configuration

- [x] YAML theme system *(implemented)*
- [x] Theme editor UI *(implemented in settings/theme_editor.rs)*
- [x] Gruvbox Dark theme *(included)*
- [x] Nord theme *(included)*
- [x] Preferences management *(implemented in config/preferences.rs)*
- [ ] Theme marketplace/sharing
- [ ] Live theme preview
- [ ] Custom color picker

---

## 🔧 Dev Infra & Tooling

- [ ] Unit tests for all modules
- [ ] UI snapshot testing
- [ ] Benchmark PTY, AI, fuzzy search
- [ ] GitHub Actions CI (build, lint, test)
- [ ] DevContainer + Dockerfile
- [ ] Installable binary/AppImage

---

## 📁 Current Module Status

| Module                      | Status         | Notes |
|----------------------------|----------------|-------|
| `command/`                 | 🟡 Scaffolded  | Core shell execution |
| `agent_mode_eval/`         | 🟡 Scaffolded  | AI context wrapper |
| `fuzzy_match/`             | 🟡 Scaffolded  | Search functionality |
| `lpc/`                     | 🟡 Scaffolded  | Language translation |
| `mcq/`                     | 🟡 Scaffolded  | Multi-choice UI |
| `natural_language_detection/` | 🟡 Scaffolded  | Language detection |
| `sum_tree/`                | 🟡 Scaffolded  | History management |
| `serve_wasm/`              | 🟡 Scaffolded  | Plugin runtime |
| `drive/`                   | 🟡 Scaffolded  | Cloud sync |
| `graphql/`                 | 🟡 Scaffolded  | API layer |
| `asset_macro/`             | 🟡 Scaffolded  | Workflow macros |
| `syntax_tree/`             | 🟡 Scaffolded  | Code parsing |
| `string_offset/`           | 🟡 Scaffolded  | Text handling |
| `resources/`               | 🟡 Scaffolded  | Asset management |
| `virtual_fs/`              | 🟡 Scaffolded  | Sandboxing |
| `watcher/`                 | 🟡 Scaffolded  | File monitoring |
| `integration/`             | 🟡 Scaffolded  | External tools |
| `websocket/`               | 🟡 Scaffolded  | Real-time events |
| `languages/`               | 🟡 Scaffolded  | Shell detection |
| `markdown_parser/`         | 🟡 Scaffolded  | Rich text output |
| `workflows/`               | 🟢 Implemented | Manager, executor, UI |
| `config/`                  | 🟢 Implemented | Themes, preferences |
| `settings/`                | 🟢 Implemented | Theme & keybinding editors |

---

## 🚀 Next Priority Tasks

### High Priority (Core Functionality)
1. **Complete command execution engine** - Implement actual PTY handling in `command/`
2. **Finish block rendering** - Add collapsible blocks, better status indicators
3. **Command palette** - Implement ⌘K fuzzy command finder
4. **File explorer** - Add sidebar file browser

### Medium Priority (User Experience)
1. **Theme system polish** - Live preview, custom colors
2. **Keybinding system** - Complete remapping functionality
3. **Search improvements** - Block content search, history search
4. **Workflow execution** - Connect YAML workflows to actual execution

### Low Priority (Advanced Features)
1. **AI integration** - Connect to OpenAI/Claude APIs
2. **Plugin system** - Implement WASM runtime
3. **Cloud sync** - Database integration for settings/workflows
4. **Testing infrastructure** - Unit tests and benchmarks

---

## 📦 Architecture Notes

- **Core**: Built with Rust for performance and safety
- **UI**: Custom renderer (likely using a GUI framework like egui or tauri)
- **Themes**: YAML-based configuration system
- **Workflows**: YAML definitions with Rust execution engine
- **Modularity**: Well-structured module system for extensibility

The project has excellent scaffolding and architecture. Most core modules are set up but need implementation details filled in.
