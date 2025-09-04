use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext, CmdPreprocessor};
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::path::Path;
use sha2::{Sha256, Digest};
use std::{process, fs, env};
use semver::{Version, VersionReq};
use std::time::Duration;
use pulldown_cmark::{Parser, Event, Tag, TagEnd, CodeBlockKind};

pub struct DeepSeekTranslator {
    cache_file: String,
    target_lang: String,
}

impl DeepSeekTranslator {
    fn new(target_lang: &str) -> Self {
        Self {
            cache_file: "deepseek_cache.json".to_string(),
            target_lang: target_lang.to_string(),
        }
    }

    // 读取缓存
    fn load_cache(&self) -> Value {
        if Path::new(&self.cache_file).exists() {
            let data = fs::read_to_string(&self.cache_file).expect("读取缓存文件失败");
            serde_json::from_str(&data).unwrap_or(json!({}))
        } else {
            json!({})
        }
    }

    // 写入缓存
    fn save_cache(&self, cache: &Value) {
        let data = serde_json::to_string_pretty(cache).expect("序列化缓存失败");
        fs::write(&self.cache_file, data).expect("写入缓存失败");
    }

    fn hash_key(&self, text: &str) -> String {
        let mut hasher = Sha256::new();
        // 可以把目标语言也加进 hash，支持多语言缓存
        hasher.update(text.as_bytes());
        hasher.update(self.target_lang.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Preprocessor for DeepSeekTranslator {
    fn name(&self) -> &str {
        "deepseek-translator"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        eprintln!("here-----");
        let api_key = env::var("DEEPSEEK_API_KEY")
            .expect("请在环境变量中设置 DEEPSEEK_API_KEY");

        eprintln!("api_key: {:?}", api_key);

        let client = Client::builder()
                    .proxy(reqwest::Proxy::all("http://127.0.0.1:8099")?)
                    .timeout(Duration::from_secs(600)) // 显式设置超时
                    .build()?;
        let mut cache = self.load_cache();

        fn translate_text(
            client: &Client,
            api_key: &str,
            text: &str,
            cache: &mut Value,
            translator: &DeepSeekTranslator,
        ) -> String {
            let key = translator.hash_key(text);
            // 使用原文作为 key，简单去重
            if let Some(cached) = cache.get(&key) {
                eprintln!("cached: {:?}", cached);
                return cached.as_str().unwrap_or("").to_string();
            }

            let url = "https://api.deepseek.com/v1/chat/completions";
            let body = json!({
                "model": "deepseek-chat",
                "messages": [
                    {
                        "role": "system",
                        "content": "你是专业技术文档翻译助手，保留代码、命令，术语翻译尽量遵循 Rust 中文社区的常见用法。如果有不理解的术语，保持英文原文。"
                    },
                    {
                        "role": "user",
                        "content": format!("Translate the following text into Chinese:\n\n{}", text)
                    }
                ]
            });

            let resp = client
                .post(url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .expect("请求 DeepSeek API 失败");

            let json_resp: serde_json::Value =
                resp.json().expect("解析 DeepSeek API 返回失败");

            eprintln!("json_resp: {:?}", json_resp);
            let translated = json_resp["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

            if !translated.is_empty() {
                // 写入缓存
                cache[&key] = json!(translated);
            }

            translated
        }

        fn walk_items(client: &Client, api_key: &str, items: &mut Vec<BookItem>, cache: &mut Value, translator: &DeepSeekTranslator,) {
            for item in items.iter_mut() {
                match item {
                    BookItem::Chapter(chapter) => {
                        // let translated_name = translate_text(client, api_key, &chapter.name, cache, translator);
                        // chapter.name = chapter.name.clone() + " " + &translated_name;
                        // eprintln!("chapter.name: {:?}", chapter.name);
                        let chunks = split_into_chunks(&chapter.content, 4000);
                        chapter.content = "".to_string();
                        eprintln!("chunks: {:?}", chunks);
                        chunks.into_iter().for_each(|chunk| {
                            eprintln!("chunk: {:?}, {:?}", chunk, chunk.len());
                            let translated = translate_text(client, api_key, &chunk, cache, translator);
                            chapter.content.push_str(&translated);
                            // 如果是以```结尾，则加上一个换行符
                            if translated.ends_with("```") {
                                chapter.content.push_str("\n\n");
                            }
                        });
                        walk_items(client, api_key, &mut chapter.sub_items, cache, translator);
                    }
                    _ => {}
                }
            }
        }

        walk_items(&client, &api_key, &mut book.sections, &mut cache, &self);

        // 保存缓存
        self.save_cache(&cache);

        Ok(book)
    }
}

fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut buffer = String::new();
    let mut is_in_code = false;
    eprintln!("text: {:?}", text);

    text.lines().into_iter().for_each(|line| {
        if line.is_empty() {
            buffer.push_str("\n\n");
            return;
        }
        if(line.starts_with("```")) {
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

// fn main() -> Result<()> {
//     eprintln!("translator main");
//     let preprocessor = DeepSeekTranslator::new("Chinese"); // 可修改目标语言

//     // 直接读取原始输入
//     let mut raw_input = String::new();
//     io::stdin().read_to_string(&mut raw_input)?;
//     eprintln!("Raw input:\n{}", raw_input); // 打印原始数据


//     if let Err(e) = handle_preprocessing(&preprocessor) {
//         eprintln!("{e:?}");
//         process::exit(1);
//     }
//     Ok(())
// }

// fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
//     eprintln!("handle_preprocessing");
    
//     match CmdPreprocessor::parse_input(io::stdin()) {
//         Ok((ctx, book)) => {
//             // 处理成功的情况
//             let processed_book = pre.run(&ctx, book)?;
//             serde_json::to_writer(io::stdout(), &processed_book)?;
//             Ok(())
//         },
//         Err(e) => {
//             eprintln!("解析输入失败: {:?}", e);
//             Err(e)
//         }
//     }

//     // let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

//     // let book_version = Version::parse(&ctx.mdbook_version)?;
//     // let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

//     // if !version_req.matches(&book_version) {
//     //     eprintln!(
//     //         "Warning: The {} plugin was built against version {} of mdbook, \
//     //          but we're being called from version {}",
//     //         pre.name(),
//     //         mdbook::MDBOOK_VERSION,
//     //         ctx.mdbook_version
//     //     );
//     // }

//     // let processed_book = pre.run(&ctx, book)?;
//     // serde_json::to_writer(io::stdout(), &processed_book)?;

//     // Ok(())
// }

// nop-preprocessors.rs
use clap::{Arg, ArgMatches, Command};
use std::io::{self, Read};

pub fn make_app() -> Command {
    Command::new("nop-preprocessor")
        .about("A mdbook preprocessor which does precisely nothing")
        .subcommand(
            Command::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

fn main() {
    let matches = make_app().get_matches();

    // Users will want to construct their own preprocessor here
    let preprocessor = DeepSeekTranslator::new("Chinese");
    // let preprocessor = nop_lib::Nop::new();

    // eprintln!("here---- {:?}", matches);

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{e:?}");
        process::exit(1);
    }
}

mod nop_lib {
    use super::*;

    /// A no-op preprocessor.
    pub struct Nop;

    impl Nop {
        pub fn new() -> Nop {
            Nop
        }
    }

    impl Preprocessor for Nop {
        fn name(&self) -> &str {
            "nop-preprocessor"
        }

        fn run(&self, ctx: &PreprocessorContext, book: Book) -> Result<Book, Error> {
            // In testing we want to tell the preprocessor to blow up by setting a
            // particular config value
            if let Some(nop_cfg) = ctx.config.get_preprocessor(self.name()) {
                if nop_cfg.contains_key("blow-up") {
                    anyhow::bail!("Boom!!1!");
                }
            }

            // we *are* a no-op preprocessor after all
            Ok(book)
        }

        fn supports_renderer(&self, renderer: &str) -> bool {
            renderer != "not-supported"
        }
    }
}


fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {

    // 直接读取原始输入
    // let mut raw_input = String::new();
    // io::stdin().read_to_string(&mut raw_input)?;
    // eprintln!("Raw input:\n{}", raw_input); // 打印原始数据


    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    let book_version = Version::parse(&ctx.mdbook_version)?;
    let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

    if !version_req.matches(&book_version) {
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args
        .get_one::<String>("renderer")
        .expect("Required argument");
    let supported = pre.supports_renderer(renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}