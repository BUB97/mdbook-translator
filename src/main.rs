use mdbook_translator::DeepSeekTranslator;
use mdbook_translator::{make_app, handle_supports, handle_preprocessing};
use std::process;

fn main() {
    let matches = make_app().get_matches();

    // Users will want to construct their own preprocessor here
    let preprocessor = DeepSeekTranslator::new("Chinese");
    // let preprocessor = nop_lib::Nop::new();

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{e:?}");
        process::exit(1);
    }
}
