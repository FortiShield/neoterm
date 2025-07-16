# NeoTerm

NeoTerm is a next-generation terminal designed to enhance your command-line experience with modern UI, AI assistance, and powerful extensibility.

## Features

- **Modern GUI**: Built with Iced, providing a smooth and responsive user interface.
- **Block-based Output**: Commands and their outputs are grouped into atomic, collapsible blocks for better organization and readability.
- **Intelligent Input**: Features command history, smart autocomplete with fuzzy matching, and contextual suggestions.
- **AI Integration**: Leverage AI for explanations, command rewrites, and intelligent assistance directly within your terminal session.
- **Extensible**: Designed with a plugin architecture to allow users to extend functionality.
- **Customizable Themes**: Personalize your terminal's appearance with flexible theming options.
- **Environment Profiles**: Easily switch between different environment configurations.

## Roadmap

NeoTerm is under active development. Here's a glimpse of what's planned:

- **Phase 1: Core Terminal Architecture (Completed)**
- **Phase 2: Block Management & UI Enhancements (Completed)**
- **Phase 3: Input Engine & Autocomplete (Completed)**
- **Phase 4: AI Integration (In Progress)**
- Phase 5: Contextual AI & Tooling
- Phase 6: Plugin Marketplace & Extensibility
- Phase 7: Theme Editor & Customization
- Phase 8: Collaboration & Cloud Sync
- Phase 9: Performance & Optimization
- Phase 10: Distribution & Packaging

For a detailed breakdown of the roadmap, see `AI_ROADMAP.md`.

## Getting Started

### Prerequisites

- Rust (latest stable version recommended)
- `cargo` (Rust's package manager)

### Building and Running

1.  **Clone the repository:**
    \`\`\`bash
    git clone https://github.com/your-repo/neoterm.git
    cd neoterm
    \`\`\`

2.  **Set up OpenAI API Key (for AI features):**
    NeoTerm uses the OpenAI API for its AI capabilities. You'll need to set your API key as an environment variable:
    \`\`\`bash
    export OPENAI_API_KEY="YOUR_OPENAI_API_KEY"
    \`\`\`
    Replace `"YOUR_OPENAI_API_KEY"` with your actual OpenAI API key.

3.  **Run the application:**
    \`\`\`bash
    cargo run
    \`\`\`

This will compile and run the NeoTerm application.

## Usage

- **Command Input**: Type commands in the input bar at the bottom and press Enter.
- **History Navigation**: Use `Up` and `Down` arrow keys to navigate through command history.
- **Autocomplete**: Press `Tab` to cycle through command suggestions.
- **AI Commands**: Type `#` or `/ai` followed by your query to ask the AI for assistance (e.g., `# how to list files in a directory`).
- **Toggle AI Agent**: Click the "🤖 Agent ON/OFF" button in the toolbar to enable/disable the AI conversational agent.
- **Settings**: Click the "⚙️ Settings" button to open the settings panel.

## Contributing

We welcome contributions! Please see our `CONTRIBUTING.md` (placeholder) for guidelines.

## License

This project is licensed under the MIT License. See the `LICENSE` (placeholder) file for details.
