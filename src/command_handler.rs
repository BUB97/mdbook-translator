use mdbook::preprocess::{Preprocessor, CmdPreprocessor};
use clap::{Arg, ArgMatches, Command};
use semver::{Version, VersionReq};
use std::io;
use mdbook::errors::Error;
use anyhow::Result;
use std::process;
use toml::value::Value;
use crate::translate_preprocessor::DeepSeekTranslator;

pub fn handle_preprocessing(pre: &mut DeepSeekTranslator) -> Result<(), Error> {
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

    let language = 
        ctx.config.get("preprocessor")
            .and_then(|p| p.get("translator"))
            .and_then(|t| t.get("language"));
    let ext_prompt = 
        ctx.config.get("preprocessor")
            .and_then(|p| p.get("translator"))
            .and_then(|t| t.get("prompt"));
    let proxy = 
        ctx.config.get("preprocessor")
            .and_then(|p| p.get("translator"))
            .and_then(|t| t.get("proxy"));

    if let Some(Value::String(language_config)) = language {
        if !language_config.is_empty() {
            pre.set_language(language_config);
        }
    }

    if let Some(Value::String(prompt_config)) = ext_prompt {
        if !prompt_config.is_empty() {
            pre.set_prompt(prompt_config);
        }
    }

    if let Some(Value::String(proxy_config)) = proxy {
        if !proxy_config.is_empty() {
            pre.set_proxy(proxy_config);
        }
    }

    eprintln!("target_lang: {:?}", pre.target_lang);
    eprintln!("prompt: {:?}", pre.prompt);

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

pub fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
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

pub fn make_app() -> Command {
    Command::new("mdbook-translator")
        .about("A translation preprocessor plugin for mdBook that automatically translates Markdown documents using the DeepSeek API.")
        .subcommand(
            Command::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}