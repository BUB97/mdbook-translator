use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::path::Path;
use sha2::{Sha256, Digest};
use std::{fs, env};
use std::time::Duration;

pub struct DeepSeekTranslator {
    cache_file: String,
    pub target_lang: String,
    pub prompt: String,
    pub proxy: String,
}

impl DeepSeekTranslator {
    pub fn new() -> Self {
        Self {
            cache_file: "deepseek_cache.json".to_string(),
            target_lang: String::new(),
            prompt: String::new(),
            proxy: String::new(),
        }
    }

    pub fn set_language(&mut self, lang: &str) {
        self.target_lang = lang.to_string();
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = prompt.to_string();
    }

    pub fn set_proxy(&mut self, proxy: &str) {
        self.proxy = proxy.to_string();
    }
    
    // 读取缓存
    fn load_cache(&self) -> Value {
        if Path::new(&self.cache_file).exists() {
            let data = fs::read_to_string(&self.cache_file).expect("failed to read cache file");
            serde_json::from_str(&data).unwrap_or(json!({}))
        } else {
            json!({})
        }
    }

    // 写入缓存
    fn save_cache(&self, cache: &Value) {
        let data = serde_json::to_string_pretty(cache).expect("failed to serialize cache");
        fs::write(&self.cache_file, data).expect("failed to write cache file");
    }

    fn hash_key(&self, text: &str) -> String {
        let mut hasher = Sha256::new();
        // 可以把目标语言也加进 hash，支持多语言缓存
        hasher.update(text.as_bytes());
        hasher.update(self.target_lang.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

struct Message {
    role: String,
    content: String,
}

impl DeepSeekTranslator {
    pub fn translate_text(
        &self,
        client: &Client,
        api_key: &str,
        text: &str,
        cache: &mut Value,
    ) -> String {
        let key = self.hash_key(text);
        // 使用原文作为 key，简单去重
        if let Some(cached) = cache.get(&key) {
            let mut print_cached = String::new();
            if let Value::String(cached_str) = cached {
                if cached_str.chars().count() > 100 {
                    print_cached.push_str(&cached_str.chars().take(100).collect::<String>());
                    print_cached.push_str("...");
                } else {
                    print_cached.push_str(cached_str);
                }
            }
            eprintln!("\x1b[38;2;38;188;213;1mCache hit:\x1b[0m {:?}", print_cached);
            return cached.as_str().unwrap_or("").to_string();
        }

        let url = "https://api.deepseek.com/v1/chat/completions";
        let mut messages = Vec::from([
            Message {
                role: "system".to_string(),
                content: "你是专业技术文档翻译助手，保留代码、命令，术语翻译尽量遵循社区的常见用法。如果有不理解的术语，保持原文。".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: format!("Translate the following text into {}:\n\n{}", self.target_lang, text).to_string(),
            }
        ]);
        if !self.prompt.is_empty() {
            messages.push(Message {
                role: "user".to_string(),
                content: self.prompt.to_string(),
            });
        }

        let body = json!({
            "model": "deepseek-chat",
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
        });

        eprintln!("\x1b[38;2;214;200;75;1mRequesting Deepseek API, please wait patiently\x1b[0m");
        let resp = client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .expect("failed to send request to deepseek api");

        let json_resp: serde_json::Value =
            resp.json().expect("failed to parse response from deepseek api");

        let translated = json_resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if !translated.is_empty() {
            // 写入缓存
            cache[&key] = json!(translated);
        }

        let mut print_translated = String::new();
            if translated.chars().count() > 100 {
                print_translated.push_str(&translated.chars().take(100).collect::<String>());
                print_translated.push_str("...");
            } else {
                print_translated.push_str(&translated);
            }

        eprintln!("\x1b[38;2;214;200;75;1mRequest succeed, translated:\x1b[0m {:?}", print_translated);

        translated
    }

    fn walk_items(&self, client: &Client, api_key: &str, items: &mut Vec<BookItem>, cache: &mut Value) {
        for item in items.iter_mut() {
            match item {
                BookItem::Chapter(chapter) => {
                    let chapter_num = match &chapter.number {
                        Some(num) => num.to_string(),
                        None => "".to_string(),
                    };

                    eprintln!();
                    eprintln!("\x1b[32;1mProcessing chapter:\x1b[0m  \x1b[1m{}{}\x1b[0m", &chapter_num, &chapter.name);

                    let chunks = split_into_chunks(&chapter.content, 4000);
                    chapter.content = "".to_string();
                    chunks.into_iter().for_each(|chunk| {
                        let translated = self.translate_text(client, api_key, &chunk, cache);
                        chapter.content.push_str(&translated);
                        // 如果是以```结尾，则加上一个换行符
                        if translated.ends_with("```") {
                            chapter.content.push_str("\n\n");
                        }
                    });
                    self.walk_items(client, api_key, &mut chapter.sub_items, cache);
                }
                _ => {}
            }
        }
    }
}

impl Preprocessor for DeepSeekTranslator {
    fn name(&self) -> &str {
        "translator"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        let api_key = env::var("DEEPSEEK_API_KEY")
            .expect("请在环境变量中设置 DEEPSEEK_API_KEY");

        // eprintln!("api_key: {:?}", api_key);

        let proxy = &self.proxy;
        let mut client_builder = Client::builder()
                    .timeout(Duration::from_secs(600)); // 显式设置超时
        
        if !proxy.is_empty() {
            client_builder = client_builder.proxy(reqwest::Proxy::all(proxy)?);
        }
        
        let client = client_builder.build()?;
        let mut cache = self.load_cache();

        self.walk_items(&client, &api_key, &mut book.sections, &mut cache);

        // 保存缓存
        self.save_cache(&cache);

        Ok(book)
    }
}

fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut buffer = String::new();
    let mut is_in_code = false;

    text.lines().into_iter().for_each(|line| {
        if line.is_empty() {
            buffer.push_str("\n\n");
            return;
        }
        if line.starts_with("```") {
            buffer.push_str(line);
            buffer.push_str("\n");
            is_in_code = !is_in_code;
            return;
        }
        if is_in_code || (buffer.len() + line.len() < max_chars){
            buffer.push_str(&line);
            buffer.push_str("\n");
        } else {
            chunks.push(buffer.clone());
            buffer.clear();
            buffer.push_str(&line);
            buffer.push_str("\n");
        }
    });
    if !buffer.is_empty() {
        chunks.push(buffer.clone());
        buffer.clear();
    }
    chunks
}
