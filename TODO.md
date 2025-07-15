# ✅ NeoPilot Terminal — Enhanced Feature Roadmap & To-Do

> Modern terminal with AI integration, workflows, plugins, language adaptation, and cloud sync

---

## 🧱 Core Engine

- [x] `command/`: Shell lifecycle & PTY I/O *(scaffolded)*
- [x] `string_offset/`: Unicode-aware text slicing/indexing *(scaffolded)*
- [x] `sum_tree/`: Undo/redo history with tree structure *(scaffolded)*
- [x] `syntax_tree/`: Shell/code syntax parser *(scaffolded)*
- [x] `virtual_fs/`: Sandboxed command execution *(scaffolded)*
- [x] `watcher/`: File system + command runtime monitoring *(scaffolded)*
- [ ] **Runtime Permissions System**: Granular access controls
- [ ] **Memory Optimization**: Reduce footprint for long sessions

---

## 🖥️ UI + Terminal Experience

### Core Features
- [x] Command blocks (Running/Done/Error status) *(block.rs)*
- [x] Input history (↑ ↓) *(input.rs)*
- [x] Scrollable block view *(renderer.rs)*
- [ ] Collapsible output blocks
- [x] Tabbed sessions *(partial implementation)*
- [x] Custom themes (YAML-based) *(theme_editor.rs)*
- [ ] GPU acceleration with `wgpu`

### Visual Enhancements
- [ ] Rounded corners and padding controls
- [ ] Font ligature support
- [ ] Pane dimming & focus highlighting
- [ ] Animated transitions between states
- [ ] Live theme preview in editor
- [ ] Custom cursor styles

---

## 🔍 Search & Navigation

- [x] `fuzzy_match/`: Command & block fuzzy matching *(scaffolded)*
- [x] `mcq/`: Multi-choice prompts (fuzzy UI) *(scaffolded)*
- [x] Keybinding remapper *(keybinding_editor.rs)*
- [ ] **Universal Search**: Commands + files + workflows
- [ ] Block jump (Ctrl+J/K)
- [ ] Command palette (⌘K)
- [ ] Session history timeline
- [ ] Smart output folding

---

## 🧠 AI Integration

### Core Capabilities
- [x] `agent_mode_eval/`: Multi-provider AI support *(implemented)*
- [x] `lpc/`: Cross-shell translation (bash↔pwsh↔zsh) *(scaffolded)*
- [x] `natural_language_detection/` *(scaffolded)*
- [x] **Multi-Provider Support**: OpenAI, Claude, Gemini, Ollama, Groq
- [x] **Comprehensive Model Library**: 20+ models including:
  - **OpenAI**: GPT-4o, GPT-4, GPT-4-turbo, GPT-4-mini, GPT-3.5-turbo, O3, O3-mini
  - **Claude**: Claude 4 Sonnet, Claude 4 Opus, Claude 3.7 Sonnet, Claude 3.5 Sonnet, Claude 3.7 Haiku
  - **Gemini**: Gemini 2.0 Flash, Gemini 2.0 Pro, Gemini 1.5 Pro, Gemini 1.5 Flash
  - **Ollama**: Llama 3.2, Llama 3.1, CodeLlama, Mistral, Phi3, Qwen 2.5, DeepSeek Coder
  - **Groq**: Llama 3.1 70B, Llama 3.1 8B, Mixtral 8x7B, Gemma2 9B
- [x] **Tool System**: 8 built-in tools (command execution, file operations, git status, system info)
- [x] **Conversation Management**: Full history with metadata and persistence
- [x] **Async Architecture**: Non-blocking AI operations
- [ ] **AI Assistant Sidebar**: Chat interface with context injection
- [ ] Local model support (`ollama`, `llama.cpp`)
- [ ] "Explain this output" button

### Advanced Features
- [x] **Agent Conversations**: Multi-turn AI dialogues *(foundation implemented)*
- [ ] **Voice Interaction**: Speech-to-command
- [ ] **Active AI**: Proactive suggestions based on context
- [ ] **Error Diagnosis**: AI-powered command debugging
- [ ] **Model Context Protocol**: Standardized AI context format

---

## ⚙️ Workflows & Automation

- [x] `asset_macro/`: Command macros *(scaffolded)*
- [x] `drive/`: Workflow storage *(scaffolded)*
- [x] YAML workflow definitions *(implemented)*
- [ ] **Workflow Debugger**: Step-through execution
- [ ] **Workflow Marketplace**: Shareable templates
- [ ] **Environment Variables Manager**: Per-session profiles
- [ ] **Prompt Customization**: Dynamic workflow inputs
- [ ] **Rules Engine**: Conditional automation triggers

---

## 🧩 Plugins & Extensions

- [x] `serve_wasm/`: WASM plugin runtime *(scaffolded)*
- [x] `resources/`: Plugin assets *(scaffolded)*
- [ ] **Plugin Manager UI**: Install/update/remove
- [ ] **Lua Scripting Engine**: `mlua` integration
- [ ] **Runtime Hooks**: Pre-command/post-output triggers
- [ ] **Hot Reload**: Instant plugin updates
- [ ] **Sandboxed Execution**: Security boundaries

---

## 🔌 Integrations

- [x] `integration/`: Git, Docker, SSH *(scaffolded)*
- [x] `websocket/`: Real-time events *(scaffolded)*
- [ ] **Enhanced SSH**: Jump hosts + session manager
- [ ] **Notebooks Integration**: Terminal ↔ Jupyter bridge
- [ ] **CI/CD Tools**: GitHub Actions, GitLab CI helpers
- [ ] **Cloud SDKs**: AWS/Azure/GCP helpers
- [ ] **API Client**: Built-in HTTP tool

---

## 🌐 Cloud & Collaboration

- [x] `graphql/`: Local API *(scaffolded)*
- [ ] **Teams System**: Shared workflows/environments
- [ ] **Session Sharing**: Real-time collaboration
- [ ] **Warp Drive Web**: Browser access to workflows
- [ ] **Conflict Resolution**: Sync merge strategies
- [ ] **Usage Analytics**: Team activity dashboard

---

## ♿ Accessibility

- [ ] Screen reader support
- [ ] High contrast themes
- [ ] Keyboard navigation mode
- [ ] Adjustable animation levels
- [ ] Closed captioning for audio

---

## 🛠️ Dev Tooling & QA

- [ ] Unit test coverage (aim for 85%)
- [ ] **Performance Benchmarks**: PTY, AI, rendering
- [ ] **UI Snapshot Testing**: Visual regression
- [ ] **Error Tracking**: Sentry integration
- [ ] **CI/CD Pipeline**: GitHub Actions
- [ ] **Docker Dev Environment**
- [ ] **Linux Packaging**: AppImage/Deb/RPM

---

## 📊 Module Progress

| Module                      | Status         | Notes |
|----------------------------|----------------|-------|
| `command/`                 | 🟡 Scaffolded  | Needs PTY implementation |
| `agent_mode_eval/`         | 🟢 Implemented | Multi-provider AI support |
| `fuzzy_match/`             | 🟢 Implemented | Needs performance tuning |
| `lpc/`                     | 🟡 Scaffolded  | Translation engine |
| `sum_tree/`                | 🟡 Scaffolded  | History management |
| `serve_wasm/`              | 🟡 Scaffolded  | WASM runtime |
| `drive/`                   | 🟡 Scaffolded  | Needs cloud sync |
| `syntax_tree/`             | 🟡 Scaffolded  | Shell parsing |
| `virtual_fs/`              | 🟡 Scaffolded  | Sandboxing |
| `workflows/`               | 🟢 Implemented | Needs debugger |
| `config/`                  | 🟢 Implemented | Add live preview |
| `settings/`                | 🟢 Implemented | Plugin manager UI needed |

---

## 🎯 AI Integration Status

### ✅ Completed Features
- **Multi-Provider Support**: OpenAI, Claude, Gemini, Ollama, Groq
- **Comprehensive Model Library**: 20+ models including latest GPT-4o, Claude 4, Gemini 2.0
- **Tool System**: 8 built-in tools (command execution, file ops, git status, etc.)
- **Conversation Management**: Full history with metadata and persistence
- **Async Architecture**: Non-blocking AI operations
- **Error Handling**: Robust error recovery and reporting

### 🔄 In Progress
- **Streaming Responses**: Real-time AI output (foundation implemented)
- **Context Injection**: Terminal state awareness
- **Provider Selection UI**: Settings interface for model switching

### 📋 Next Steps
- **AI Sidebar Implementation**: Dedicated chat interface
- **Voice Integration**: Speech-to-text and text-to-speech
- **Model Benchmarking**: Performance comparison tools
- **Custom Tool Creation**: User-defined tool system

---

## 🚀 Priority Roadmap

### Phase 1: Core Experience (Next 4 Weeks)
1. **Complete PTY implementation** - Robust command execution
2. **Collapsible blocks** - Better output management
3. **Command palette** - Universal action hub
4. **AI sidebar MVP** - Basic chat integration
5. **Workflow debugger** - Step-through execution

### Phase 2: Advanced Features (Weeks 5-8)
1. **Plugin system** - WASM + Lua runtime
2. **Session sharing** - Real-time collaboration
3. **Local AI integration** - Ollama support
4. **Enhanced theming** - Live preview + marketplace
5. **Accessibility features** - Screen reader support

### Phase 3: Ecosystem (Weeks 9-12)
1. **Cloud sync** - Supabase backend
2. **Team management** - Shared environments
3. **Plugin marketplace** - Discover/install extensions
4. **Performance optimization** - Benchmarking
5. **Linux packaging** - AppImage/Deb support

---

## 🌟 Vision Alignment

```mermaid
graph LR
A[Core Terminal] --> B[AI Integration]
A --> C[Workflows]
A --> D[Plugins]
B --> E[Cloud Sync]
C --> E
D --> E
E --> F[Collaboration]
F --> G[Team Environments]
