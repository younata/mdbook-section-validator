extern crate tokio;

use clap::{App, Arg, ArgMatches, SubCommand};
use mdbook_section_validator::ValidatorProcessor;
use mdbook_section_validator::issue_validator::DefaultIssueValidator;
use mdbook::preprocess::{Preprocessor, CmdPreprocessor};
use std::{io, process};
use mdbook::errors::Error;

pub fn make_app() -> App<'static, 'static> {
    App::new("mdbook-section-validator")
        .about("A preprocessor that validates sections that could change in the future.")
        .subcommand(
            SubCommand::with_name("supports")
                .arg(Arg::with_name("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

#[tokio::main]
async fn main() {
    let matches = make_app().get_matches();
    let preprocessor = ValidatorProcessor {
        validator: Box::new(DefaultIssueValidator)
    };
    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    }
    if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{}", e);
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    if ctx.mdbook_version != mdbook::MDBOOK_VERSION {
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
    let renderer = sub_args.value_of("renderer").expect("Required argument");
    let supported = pre.supports_renderer(&renderer);
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}