# mdbook-translator

**Languages:** [English](README.md) | [中文](README_CN.md)

一个用于 mdBook 的翻译预处理器插件，使用 DeepSeek API 自动翻译 Markdown 文档。

## 功能特性

- 🌐 自动翻译 mdBook 文档内容
- 🔄 智能缓存机制，避免重复翻译
- 🎯 保留代码块和专业术语
- 🚀 基于 DeepSeek API 的高质量翻译
- ⚙️ 可配置prompt
- 📚 支持多语言翻译

## 安装

### 从源码构建

```bash
# 克隆项目
git clone <repository-url>
cd mdbook-translator

# 构建项目
cargo build --release

# 安装到系统路径
cargo install --path .
```

### 使用 cargo install

```bash
cargo install mdbook-translator
```

## 配置

### 1. 获取 DeepSeek API 密钥

访问 [DeepSeek 官网](https://platform.deepseek.com/) 获取 API 密钥，并设置环境变量：

```bash
export DEEPSEEK_API_KEY="your-api-key-here"
```

### 2. 配置 book.toml

在你的 mdBook 项目的 `book.toml` 文件中添加以下配置：

```toml
[book]
title = "你的书籍标题"
authors = ["作者名"]

[build]
build-dir = "book-zh"  # 可选：指定输出目录

[preprocessor.translator]
command = "mdbook-translator"
language = "Chinese"  # 目标翻译语言
prompt = "请保持Send、Future、Futures等rust中的专业术语不要翻译"  # 可选：自定义翻译提示
# 支持配置代理
proxy = "http://127.0.0.1:8099"  # 可选：HTTP 代理 URL
```

### 配置选项说明

- `language`: 目标翻译语言（如 "Chinese"、"Japanese"、"Korean" 等）
- `prompt`: 可选的自定义翻译提示，用于指导翻译行为
- `proxy`: 可选的 HTTP 代理 URL
- `build-dir`: 可选的输出目录，默认为 "book"

## 使用方法

### 基本使用

```bash
# 在你的 mdBook 项目目录中运行
mdbook build
```

插件会自动：
1. 读取源文档
2. 调用 DeepSeek API 进行翻译
3. 缓存翻译结果
4. 生成翻译后的文档

### 清理缓存

如果需要重新翻译，可以删除缓存文件：

```bash
rm deepseek_cache.json
```

### 调试模式

插件会输出调试信息到标准错误输出，包括缓存命中情况等。

## 工作原理

1. **文档解析**: 插件遍历 mdBook 的所有章节和页面
2. **内容分块**: 将长文本分割成适合 API 处理的块
3. **智能翻译**: 调用 DeepSeek API 进行翻译，保留代码块和格式
4. **缓存机制**: 使用 SHA256 哈希缓存翻译结果，避免重复翻译
5. **文档重建**: 用翻译后的内容替换原文档内容

## 注意事项

- 确保设置了正确的 `DEEPSEEK_API_KEY` 环境变量
- 翻译过程需要网络连接（中国大陆用户可能需要配置http代理）
- 首次翻译可能需要较长时间，后续构建会使用缓存加速
- 代码块和特殊格式会被保留，不会被翻译
- 建议在翻译前备份原始文档

## 依赖项

- `mdbook`: mdBook 核心库
- `reqwest`: HTTP 客户端，用于 API 调用
- `serde_json`: JSON 序列化/反序列化
- `sha2`: 哈希计算，用于缓存键生成
- `anyhow`: 错误处理
- `clap`: 命令行参数解析
- `toml`: TOML 配置文件解析

## 示例项目

参考 `async-book` 项目的配置：

```toml
[book]
title = "Asynchronous Programming in Rust"
authors = ["Taylor Cramer", "Nicholas Cameron", "Open source contributors"]

[build]
preprocessor = ["mdbook-translator"]
build-dir = "book-zh"

[preprocessor.translator]
command = "mdbook-translator"
language = "Chinese"
prompt = "请保持Send、Future、Futures等rust中的专业术语不要翻译"
```

## 贡献

欢迎提交 Issue 和 Pull Request！