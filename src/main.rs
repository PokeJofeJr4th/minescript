#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::cast_precision_loss)]

use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs, thread};

use clap::Parser;
use dotenvy::dotenv;

/// transforms an `InterRepr` into a set of files that need to be written to a datapack
mod compiler;
/// transforms a syntax tree into an `InterRepr` containing the datapack's items, functions, and recipes
mod interpreter;
/// transforms a string into a stream of `Token`s
mod lexer;
/// turns text into json
mod md_to_json;
/// transforms a stream of `Token`s into a syntax tree
mod parser;
/// defines all relevant types
mod types;

#[cfg(test)]
mod tests;

macro_rules! input {
    ($msg: expr) => {{
        println!($msg);
        let mut response: String = String::new();
        std::io::stdin().read_line(&mut response).unwrap();
        response.trim().to_owned()
    }};
}

#[derive(Parser)]
struct Args {
    /// path to the source file
    path: String,
    /// namespace for the finished program
    namespace: String,
    /// Print debug data for intermediate representations
    #[clap(short, long)]
    verbose: bool,
    /// Enable hot reloading; when you change source file, the project will rebuild
    #[clap(short, long)]
    reload: bool,
    /// Save the datapack to a world's `datapacks` folder
    #[clap(short, long)]
    world: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let path = PathBuf::from(args.path);
    // load environment variables from `.env` file
    dotenv().ok();
    // either get "DOTMINECRAFT" from env or ask for it
    let dotminecraft = env::var("DOTMINECRAFT").map_or_else(
        |e| {
            println!("{e}");
            let dm = input!("Provide the location of your `.minecraft` folder:");
            env::set_var("DOTMINECRAFT", &dm);
            dm
        },
        |val| val,
    );
    // set the parent folder for the compiled output
    let parent = args.world.map_or_else(
        || format!("{dotminecraft}/datapacks/"),
        |world| format!("{dotminecraft}/saves/{world}/datapacks/"),
    );
    // start the list of dependent files
    let mut src_files = BTreeSet::new();
    build(
        &path,
        &parent,
        &args.namespace,
        args.verbose,
        &mut src_files,
    )?;
    println!("Successfully built {}", args.namespace);
    if args.reload {
        let dur = Duration::new(1, 0);
        loop {
            thread::sleep(dur);
            let mut need_change = false;
            for src_file in &src_files {
                let Ok(metadata) = fs::metadata(src_file) else { need_change = true; break };
                let Ok(modified) = metadata.modified() else { need_change = true; break };
                let Ok(elapsed) = modified.elapsed() else { need_change = true; break };
                if elapsed < dur {
                    need_change = true;
                    if args.verbose {
                        println!("{} has changed", src_file.to_string_lossy());
                    }
                    break;
                }
            }
            if need_change {
                println!("Rebuilding...");
                src_files = BTreeSet::new();
                match build(
                    &path,
                    &parent,
                    &args.namespace,
                    args.verbose,
                    &mut src_files,
                ) {
                    Ok(()) => println!(
                        "{} Successfully rebuilt {}",
                        chrono::Local::now().format("%H:%M:%S"),
                        args.namespace
                    ),
                    Err(err) => eprintln!("Error rebuilding {}: {err}", args.namespace),
                }
            }
        }
    }
    Ok(())
}

fn build(
    path: &Path,
    parent: &str,
    namespace: &str,
    verbose: bool,
    src_files: &mut BTreeSet<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    // read the contents of the primary source file
    let file = format!("[{}]", fs::read_to_string(path)?);
    // tokenize the raw source
    let tokens = lexer::tokenize(&file)?;
    if verbose {
        println!("{tokens:?}");
    }
    // parse the tokens to syntax
    let syntax = parser::parse(&mut tokens.into_iter().peekable())?;
    if verbose {
        println!("{syntax:?}");
    }
    // get the current folder so that imports work
    let folder = path
        .parent()
        .ok_or_else(|| String::from("Bad source path"))?;
    src_files.insert(PathBuf::from(path));
    // interpret the syntax
    let mut state = interpreter::interpret(&syntax, folder, src_files)?;
    if verbose {
        println!("{state:#?}");
    }
    // compile the InterRepr
    let compiled = compiler::compile(&mut state, namespace)?;
    if verbose {
        println!("{compiled:#?}");
    }
    compiled.write(parent, namespace)?;
    Ok(())
}
