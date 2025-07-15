# 🤖 AI Integration Roadmap for NeoTerm Terminal

## 🧱 Phase 1: Foundation (Core AI Engine)
- [x] Create `ai/` module with layered architecture
- [x] Design prompt system (`prompts.rs`)
- [x] Implement provider abstraction (`providers/`)
- [x] Add AI context collector (cwd, env, history)
- [x] Add configuration toggle (AI on/off, model choice)

## 💬 Phase 2: Natural Language to Command
- [ ] Accept free-text commands from input bar
- [ ] Generate a valid shell command from prompt
- [ ] Inject terminal context (directory, file names)
- [ ] Auto-fill the command input field

## 🔁 Phase 3: Smart Suggestions & Completion
- [ ] Suggest next likely command
- [ ] Autocomplete command as user types
- [ ] Use history and context to inform results
- [ ] Live preview below input

## 🛠️ Phase 4: Command Fix + Error Recovery
- [ ] Detect command failure (exit code or stderr)
- [ ] Ask AI to fix or suggest corrected command
- [ ] User can 1-click apply fix

## 🔍 Phase 5: Explain Output or Errors
- [ ] Highlight a block of output
- [ ] Click “Explain Output”
- [ ] AI gives natural-language description

## 🧠 Phase 6: Agent Mode + Workflow Inference
- [ ] Turn user request into multi-step workflows
- [ ] Create & save workflows as macros
- [ ] Enable interactive agents

## 📦 Phase 7: Provider Flexibility
- [x] Support OpenAI GPT-4o
- [ ] Support local LLMs (`ollama`, `llama.cpp`)
- [ ] Switchable in settings
- [ ] Offline fallback mode

## 🔐 Phase 8: Privacy, Security, Rate Limiting
- [ ] Redact tokens/envs before sending prompts
- [ ] Show usage quota (if using OpenAI)
- [ ] Allow opt-out or full local-only mode
- [ ] Mask sensitive directory/file names

# 📁 AI Module Directory Structure

```plaintext
src/ai/
├── mod.rs               # Top-level module file
├── assistant.rs         # suggest(), fix(), explain(), etc.
├── prompts.rs           # Static + dynamic prompt builders
├── context.rs           # Collect env, cwd, history
├── providers/           # Abstraction over AI backends
│   ├── openai.rs        # OpenAI GPT API support
│   ├── ollama.rs        # Local llama.cpp/Ollama support
│   └── anthropic.rs     # Optional Claude API support
