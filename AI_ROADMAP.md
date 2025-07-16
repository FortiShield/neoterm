# NeoTerm AI Roadmap

This document outlines the planned phases for integrating advanced AI capabilities into NeoTerm.

## Phase 1: Core Terminal Architecture (Completed)
- **Objective**: Establish a robust foundation for terminal emulation and command execution.
- **Key Features**:
    - Basic command input and execution.
    - Display of command output.
    - Integration with `portable-pty` for cross-platform pseudo-terminal management.
    - Basic ANSI escape code parsing for colored output using `vte`.
    - Collapsible output blocks for better organization.

## Phase 2: Block Management & UI Enhancements (Completed)
- **Objective**: Improve the user interface for managing command history and output.
- **Key Features**:
    - **Blocks**: Group commands and their outputs into atomic, manageable units.
        - Copy a command.
        - Copy a command's output.
        - Scroll directly to the start of a command's output.
        - Re-input commands.
        - Share both a command and its output (with formatting!).
        - Bookmark commands.
    - Collapsible blocks for output.
    - Basic styling for different block types (command, output, error, info).
    - Navigation and selection of blocks.

## Phase 3: Input Engine & Autocomplete (Completed)
- **Objective**: Enhance the command input experience with intelligent suggestions and history navigation.
- **Key Features**:
    - **Enhanced TextInput**:
        - Command history navigation (Up/Down arrows).
        - Autocomplete suggestions for commands, files, directories, flags, and history.
        - Fuzzy matching for suggestions.
        - Application of suggestions (e.g., via Tab key).
    - Basic command parsing for suggestion context.

## Phase 4: AI Integration (In Progress)
- **Objective**: Integrate a backend AI service to provide intelligent assistance.
- **Key Features**:
    - **`/api/ai` route**: Implement a backend API endpoint (using `warp` or `axum`) to accept block content and return AI-generated explanations or command rewrites.
    - **Inline AI responses**: Display AI responses directly within the block UI.
    - **AI command triggers**: Add special prefixes (e.g., `#` or `/ai`) to the input bar to trigger AI interactions.
    - **Streaming AI responses**: Stream AI output for real-time updates.

## Phase 5: Contextual AI & Tooling
- **Objective**: Make AI responses more relevant by providing context and enabling AI to interact with the terminal.
- **Key Features**:
    - **Contextual AI**: Send selected block content (command, output, error) as context to the AI for more relevant explanations or debugging help.
    - **AI-driven command generation**: Allow AI to suggest and generate executable commands based on user queries and context.
    - **Tool integration**: Enable AI to use internal tools (e.g., `read_file`, `list_dir`) to gather information or perform actions within the terminal environment.
    - **Agent mode enhancements**: Improve the conversational flow and multi-turn interactions with the AI agent.

## Phase 6: Plugin Marketplace & Extensibility
- **Objective**: Create a system for users to extend NeoTerm's functionality with AI-powered plugins.
- **Key Features**:
    - **Plugin API**: Define a robust API for developing plugins (e.g., using WebAssembly or Lua).
    - **Plugin Marketplace UI**: A user interface within NeoTerm to browse, install, and manage plugins.
    - **AI-powered plugins**: Examples of plugins that leverage AI for specific tasks (e.g., code generation, log analysis, smart search).

## Phase 7: Theme Editor & Customization
- **Objective**: Provide extensive customization options for the terminal's appearance.
- **Key Features**:
    - **Live Theme Editor**: A GUI for real-time editing of terminal themes (colors, fonts, etc.).
    - **Import/Export Themes**: Ability to share and import custom themes (e.g., via YAML files).
    - Integration with existing theme formats (e.g., iTerm2, Alacritty themes).

## Phase 8: Collaboration & Cloud Sync
- **Objective**: Enable collaborative terminal sessions and cloud synchronization of settings and history.
- **Key Features**:
    - **WebSocket Server**: Implement a WebSocket server for real-time session sharing.
    - **Session Sharing**: Allow multiple users to view and interact with the same terminal session.
    - **Cloud Sync Manager**: Synchronize user settings, command history, and workflows across devices via a cloud service.

## Phase 9: Performance & Optimization
- **Objective**: Ensure NeoTerm is highly performant and responsive.
- **Key Features**:
    - **Benchmarking Tools**: Built-in tools to measure terminal performance (rendering speed, input latency).
    - **Profiling**: Integration with profiling tools to identify performance bottlenecks.
    - **Optimized rendering**: Further optimizations for text rendering and scrolling.

## Phase 10: Distribution & Packaging
- **Objective**: Prepare NeoTerm for wider distribution.
- **Key Features**:
    - **Linux Packages**: Generate AppImage, Deb, and RPM packages.
    - **macOS Installer**: Create a `.dmg` installer.
    - **Windows Installer**: Create an `.msi` installer.
    - **CLI Integration**: Add a CLI for headless operations or scripting.

## Future Considerations:
- **Ollama CLI Integration**: Direct integration with Ollama for local LLM inference.
- **Mouse/Keyboard Selection**: Advanced text selection and copying within the GUI blocks.
- **Workflow Debugger**: A visual debugger for complex workflows.
- **Virtual File System**: A sandboxed file system for isolated environments.
- **Natural Language Detection**: Automatically detect language for better AI processing.
- **GraphQL Integration**: If a backend uses GraphQL.
- **LPC (Language Processing Core)**: A dedicated module for advanced language processing.
- **MCQ (Multi-Choice Questions)**: For interactive learning or onboarding.
- **Markdown Parser**: Render rich Markdown content directly in blocks.
- **Drive Management**: Integration with file system drives.
- **Fuzzy Matching**: Enhanced fuzzy matching for all search/autocomplete.
- **Asset Macro**: For embedding binary assets.
- **WebAssembly Serving**: For web-based components or plugins.
- **Watcher**: File system watcher for live updates.
