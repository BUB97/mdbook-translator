# 从零开始开发 mdbook-translator：一个基于 DeepSeek API 的 mdBook 翻译插件

## 项目背景

在技术文档的国际化过程中，翻译一直是一个耗时且容易出错的工作。特别是对于使用 mdBook 构建的技术文档，手动翻译不仅效率低下，还难以保持格式的一致性。为了解决这个问题，我决定开发一个自动化的翻译预处理器插件 —— mdbook-translator。

## 技术选型与架构设计

### 核心技术栈

- **Rust**: 选择 Rust 作为主要开发语言，充分利用其内存安全和高性能特性
- **mdBook**: 基于 mdBook 的预处理器 API 进行开发
- **DeepSeek API**: 使用 DeepSeek 的 Chat API 进行高质量翻译
- **reqwest**: 处理 HTTP 请求
- **serde_json**: JSON 序列化和反序列化
- **sha2**: 生成缓存键的哈希值

### 架构设计

项目采用模块化设计，主要包含以下几个核心模块：

```
src/
├── main.rs                    # 程序入口
├── lib.rs                     # 库文件，导出公共接口
├── command_handler.rs         # 命令行参数处理
└── translate_preprocessor.rs  # 核心翻译逻辑
```

## 开发过程详解

### 第一步：项目初始化

首先创建 Rust 项目并配置 `Cargo.toml`：

```toml
[package]
name = "mdbook-translator"
version = "0.1.2"
edition = "2024"
description = "A translation preprocessor plugin for mdBook"
license = "MIT"

[dependencies]
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.143"
anyhow = "1.0.99"
reqwest = { version = "0.12", features = ["json", "blocking"] }
mdbook = "0.4.52"
sha2 = "0.10.9"
log = "0.4.27"
semver = "1.0.26"
clap = "4.5.47"
toml = "0.5"
```

### 第二步：理解 mdBook 预处理器设计规范

在开始实现具体功能之前，首先需要理解 mdBook 预处理器的设计规范和工作原理。

#### 预处理器接口规范

mdBook 预处理器必须实现 `Preprocessor` trait，这是插件系统的核心接口：

```rust
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use mdbook::book::Book;
use mdbook::errors::Error;

impl Preprocessor for DeepSeekTranslator {
    fn name(&self) -> &str {
        "mdbook-translator"  // 预处理器的唯一标识符
    }

    fn run(&self, ctx: &PreprocessorContext, book: Book) -> Result<Book, Error> {
        // 核心处理逻辑
        // ctx: 包含配置信息和构建上下文
        // book: 包含所有章节内容的数据结构
        // 返回: 处理后的书籍数据
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"  // 指定支持的渲染器
    }
}
```

#### 命令行接口规范

mdBook 通过标准输入/输出与预处理器通信，传递上下文信息和书籍内容：

```rust
pub fn handle_preprocessing(pre: &mut DeepSeekTranslator) -> Result<(), Error> {
    // 从标准输入解析 mdBook 传递的上下文和书籍数据
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    // 检查版本兼容性
    let book_version = Version::parse(&ctx.mdbook_version)?;
    let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

    if !version_req.matches(&book_version) {
        eprintln!("Warning: Version mismatch detected");
    }

    // 从配置中读取用户设置
    let language = ctx.config.get("preprocessor")
        .and_then(|p| p.get("translator"))
        .and_then(|t| t.get("language"));
    
    let ext_prompt = ctx.config.get("preprocessor")
        .and_then(|p| p.get("translator"))
        .and_then(|t| t.get("prompt"));

    // 应用配置到预处理器实例
    if let Some(Value::String(language_config)) = language {
        pre.set_language(language_config);
    }
    
    if let Some(Value::String(prompt_config)) = ext_prompt {
        pre.set_prompt(prompt_config);
    }

    // 执行预处理逻辑
    let processed_book = pre.run(&ctx, book)?;
    
    // 将处理后的书籍数据输出到标准输出
    serde_json::to_writer(io::stdout(), &processed_book)?;
    
    Ok(())
}
```

关键要点：
- 通过 `CmdPreprocessor::parse_input()` 获取 mdBook 传递的上下文和书籍数据
- 从 `PreprocessorContext` 中提取用户配置
- 处理完成后将结果输出到标准输出供 mdBook 使用

### 第三步：实现核心翻译器结构

设计 `DeepSeekTranslator` 结构体，包含缓存机制和配置管理：

```rust
pub struct DeepSeekTranslator {
    cache_file: String,
    pub target_lang: String,
    pub prompt: String,
}
```

### 第四步：实现智能缓存机制

为了避免重复翻译相同内容，实现了基于 SHA256 哈希的缓存系统：

```rust
fn hash_key(&self, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update(self.target_lang.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

设计要点：
- 将原文和目标语言都纳入哈希计算，支持多语言缓存
- 使用 JSON 文件存储缓存，便于调试和管理
- 缓存命中时直接返回，大幅提升构建速度

### 第五步：实现 API 调用逻辑

与 DeepSeek API 的交互是项目的核心功能：

```rust
pub fn translate_text(
    &self,
    client: &Client,
    api_key: &str,
    text: &str,
    cache: &mut Value,
) -> String {
    // 检查缓存
    let key = self.hash_key(text);
    if let Some(cached) = cache.get(&key) {
        return cached.as_str().unwrap_or("").to_string();
    }

    // 构建请求消息
    let mut messages = Vec::from([
        Message {
            role: "system".to_string(),
            content: "你是专业技术文档翻译助手...".to_string(),
        },
        Message {
            role: "user".to_string(),
            content: format!("Translate the following text into {}:\n\n{}", self.target_lang, text),
        }
    ]);
    
    // 发送请求并处理响应
    // ...
}
```

### 第六步：实现 Preprocessor trait

实现 `Preprocessor` trait，使插件能够集成到 mdBook 的构建流程中：

```rust
impl Preprocessor for DeepSeekTranslator {
    fn name(&self) -> &str {
        "mdbook-translator"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        // 获取 API 密钥
        let api_key = env::var("DEEPSEEK_API_KEY")
            .map_err(|_| Error::msg("DEEPSEEK_API_KEY environment variable not set"))?;
        
        // 创建 HTTP 客户端
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| Error::msg(format!("Failed to create HTTP client: {}", e)))?;

        // 加载缓存并遍历处理所有章节
        let mut cache = self.load_cache();
        self.walk_items(&client, &api_key, &mut book.sections, &mut cache);
        self.save_cache(&cache);

        Ok(book)
    }
}
```

### 第七步：处理长文本分块

由于 API 对输入长度有限制，实现了文本分块功能：

```rust
fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    
    for line in text.lines() {
        if current_chunk.len() + line.len() + 1 > max_chars && !current_chunk.is_empty() {
            chunks.push(current_chunk.clone());
            current_chunk.clear();
        }
        if !current_chunk.is_empty() {
            current_chunk.push('\n');
        }
        current_chunk.push_str(line);
    }
    
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }
    
    chunks
}
```

### 第八步：命令行参数处理

使用 `clap` 实现命令行接口，支持 mdBook 的标准预处理器协议：

```rust
pub fn make_app() -> Command {
    Command::new("mdbook-translator")
        .about("A mdbook preprocessor for translating content")
        .subcommand(
            Command::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}
```

## 遇到的挑战与解决方案

### 1. 配置管理

**挑战**: 如何从 `book.toml` 中读取用户配置？

**解决方案**: 通过 `PreprocessorContext` 获取配置，支持 `language` 和 `prompt` 参数：

```rust
let language = ctx.config.get("preprocessor")
    .and_then(|p| p.get("translator"))
    .and_then(|t| t.get("language"));
```

### 2. 错误处理

**挑战**: 网络请求可能失败，如何优雅处理错误？

**解决方案**: 使用 `anyhow` 进行错误处理，在翻译失败时返回原文：

```rust
match response {
    Ok(translated) => {
        cache[&key] = Value::String(translated.clone());
        translated
    }
    Err(e) => {
        eprintln!("Translation failed: {}, using original text", e);
        text.to_string()
    }
}
```

### 3. 性能优化

**挑战**: 大型文档翻译耗时较长。

**解决方案**: 
- 实现智能缓存机制
- 文本分块处理
- 设置合理的超时时间
- 提供详细的进度信息

## 项目特色功能

### 1. 智能缓存
- 基于内容哈希的缓存机制
- 支持多语言缓存
- 增量翻译，避免重复工作

### 2. 灵活配置
- 支持自定义翻译提示词
- 可配置目标语言
- 可指定输出目录

### 3. 格式保持
- 保留代码块格式
- 维护 Markdown 结构
- 保持链接和图片引用

### 4. 错误恢复
- 网络错误时使用原文
- 详细的错误日志
- 降级处理机制

### 5. 网络代理支持

考虑到部分用户的网络环境，插件支持通过配置文件设置 HTTP 代理：

```toml
[preprocessor.translator]
proxy = "http://127.0.0.1:8099" # 示例，根据情况配置代理
```

## 使用示例

在 `book.toml` 中配置：

```toml
[build]
build-dir = "book-zh"

[preprocessor.translator]
command = "mdbook-translator"
language = "Chinese" # 示例
prompt = "请保持Send、Future、Futures等rust中的专业术语不要翻译" # 示例
# 中国大陆用户可能需要配置代理
proxy = "http://127.0.0.1:8099"  # 可选：HTTP 代理 URL
```

设置环境变量并构建：

```bash
export DEEPSEEK_API_KEY="your-api-key"
mdbook build
```

## 项目成果

经过几周的开发和测试，mdbook-translator 已经能够：

- ✅ 自动翻译整本 mdBook 文档
- ✅ 保持代码块和格式不变
- ✅ 智能缓存，提升构建速度
- ✅ 支持自定义翻译提示
- ✅ 提供详细的调试信息
- ✅ 优雅处理网络错误

## 未来规划

1. **多 API 支持**: 支持更多翻译服务提供商
2. **并发优化**: 实现并发翻译提升速度
3. **增量更新**: 只翻译修改过的内容
4. **术语词典**: 支持自定义术语翻译规则
5. **质量评估**: 添加翻译质量检查机制
6. **进度条**: 添加翻译过程进度提示

## 总结

开发 mdbook-translator 是一次很有价值的经历。通过这个项目，我深入学习了：

- Rust 生态系统中的各种 crate 使用
- mdBook 插件开发的实践方法
- API 集成和错误处理策略
- 缓存机制的设计与实现
- 命令行工具的开发规范

这个项目不仅解决了实际的翻译需求，也为 Rust 社区贡献了一个实用的工具。希望它能帮助更多开发者轻松地将技术文档国际化。


## 项目链接

- GitHub: [mdbook-translator](https://github.com/BUB97/mdbook-translator)
- Crates.io: [mdbook-translator](https://crates.io/crates/mdbook-translator)

---

*如果你对这个项目感兴趣，欢迎 Star、Fork 或提交 Issue！*