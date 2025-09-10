# mdbook-translator

**Languages:** [English](README_EN.md) | [中文](README.md)

A translation preprocessor plugin for mdBook that automatically translates Markdown documents using the DeepSeek API.

## Features

- 🌐 Automatic translation of mdBook document content
- 🔄 Smart caching mechanism to avoid duplicate translations
- 🎯 Preserves code blocks and technical terms
- 🚀 High-quality translation based on DeepSeek API
- ⚙️ Configurable prompts
- 📚 Multi-language translation support

## Installation

### Using cargo install

```bash
cargo install mdbook-translator
```

## Configuration

### 1. Get DeepSeek API Key

Visit [DeepSeek Official Website](https://platform.deepseek.com/) to get your API key and set the environment variable:

```bash
export DEEPSEEK_API_KEY="your-api-key-here"
```

### 2. Configure book.toml

Add the following configuration to your mdBook project's `book.toml` file:

```toml
[book]
title = "Your Book Title"
authors = ["Author Name"]

[build]
build-dir = "book-zh"  # Optional: specify output directory

[preprocessor.translator]
command = "mdbook-translator"
language = "Chinese"  # Target translation language
prompt = "Please keep technical terms like Send, Future, Futures in Rust untranslated"  # Optional: custom translation prompt
# support configure a proxy like:
proxy = "http://127.0.0.1:8099"  # Optional: HTTP proxy URL
```

### Configuration Options

- `language`: Target translation language (e.g., "Chinese", "Japanese", "Korean", etc.)
- `prompt`: Optional custom translation prompt to guide translation behavior
- `proxy`: Optional HTTP proxy URL
- `build-dir`: Optional output directory, defaults to "book"

## Usage

### Basic Usage

```bash
# Run in your mdBook project directory
mdbook build
```

The plugin will automatically:
1. Read source documents
2. Call DeepSeek API for translation
3. Cache translation results
4. Generate translated documents

### Clear Cache

If you need to retranslate, you can delete the cache file:

```bash
rm deepseek_cache.json
```

### Debug Mode

The plugin outputs debug information to standard error output, including cache hit information.

## How It Works

1. **Document Parsing**: The plugin traverses all chapters and pages in mdBook
2. **Content Chunking**: Splits long text into chunks suitable for API processing
3. **Smart Translation**: Calls DeepSeek API for translation while preserving code blocks and formatting
4. **Caching Mechanism**: Uses SHA256 hash to cache translation results, avoiding duplicate translations
5. **Document Reconstruction**: Replaces original document content with translated content

## Important Notes

- Ensure you have set the correct `DEEPSEEK_API_KEY` environment variable
- Translation process requires network connection (users in mainland China may need to configure HTTP proxy)
- First translation may take longer, subsequent builds will use cache for acceleration
- Code blocks and special formatting will be preserved and not translated
- It's recommended to backup original documents before translation

## Dependencies

- `mdbook`: mdBook core library
- `reqwest`: HTTP client for API calls
- `serde_json`: JSON serialization/deserialization
- `sha2`: Hash calculation for cache key generation
- `anyhow`: Error handling
- `clap`: Command line argument parsing
- `toml`: TOML configuration file parsing

## for Developers

### Build from Source

```bash
# Clone the project
# ssh
git clone git@github.com:BUB97/mdbook-translator.git
# or https
git clone https://github.com/BUB97/mdbook-translator.git

cd mdbook-translator

# Build the project
cargo build --release

# Install to system path
cargo install --path .
```

## Contributing

Welcome to submit Issues and Pull Requests!