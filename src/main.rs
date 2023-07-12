#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::cast_precision_loss)]

use std::error::Error;
use std::fs::{self, File};

use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

/// transforms an `InterRepr` into a set of files that need to be written to a datapack
mod compiler;
/// transforms a syntax tree into an `InterRepr` containing the datapack's items, functions, and recipes
mod interpreter;
/// transforms a string into a stream of `Token`s
mod lexer;
/// transforms a stream of `Token`s into a syntax tree
mod parser;
/// defines all relevant types
mod types;

#[cfg(test)]
mod tests;

#[derive(Parser)]
struct Args {
    /// path to the source file
    path: String,
    /// namespace for the finished program
    namespace: String,
    /// Print debug data for intermediate representations
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let path = PathBuf::from(args.path);
    let file = format!("[{}]", fs::read_to_string(&path)?);
    let tokens = lexer::tokenize(&file)?;
    if args.verbose {
        println!("{tokens:?}");
    }
    let syntax = parser::parse(&mut tokens.into_iter().peekable())?;
    if args.verbose {
        println!("{syntax:?}");
    }
    let folder = path
        .parent()
        .ok_or_else(|| String::from("Bad source path"))?;
    let mut state = interpreter::interpret(&syntax, folder)?;
    if args.verbose {
        println!("{state:#?}");
    }
    let compiled = compiler::compile(&mut state, &args.namespace)?;
    if args.verbose {
        println!("{compiled:#?}");
    }
    match fs::remove_dir_all(&args.namespace) {
        Ok(_) => println!("Deleted existing directory"),
        Err(err) => println!("Didn't delete directory: {err}"),
    }
    for (path, contents) in compiled.functions {
        let mut file = create_file_with_parent_dirs(&format!(
            "{nmsp}/data/{nmsp}/functions/{path}.mcfunction",
            nmsp = args.namespace
        ))?;
        write!(file, "{contents}")?;
        if &*path == "tick" {
            let mut tick = create_file_with_parent_dirs(&format!(
                "{nmsp}/data/minecraft/tags/functions/tick.json",
                nmsp = args.namespace
            ))?;
            write!(
                tick,
                "{{\"values\":[\"{nmsp}:tick\"]}}",
                nmsp = args.namespace
            )?;
        }
        if &*path == "load" {
            let mut load = create_file_with_parent_dirs(&format!(
                "{nmsp}/data/minecraft/tags/functions/load.json",
                nmsp = args.namespace
            ))?;
            write!(
                load,
                "{{\"values\":[\"{nmsp}:load\"]}}",
                nmsp = args.namespace
            )?;
        }
    }
    for (path, contents) in compiled.advancements {
        let mut file = create_file_with_parent_dirs(&format!(
            "{nmsp}/data/{nmsp}/advancements/{path}.json",
            nmsp = args.namespace
        ))?;
        write!(file, "{contents}")?;
    }
    for (path, contents) in compiled.recipes {
        let mut file = create_file_with_parent_dirs(&format!(
            "{nmsp}/data/{nmsp}/recipes/{path}.json",
            nmsp = args.namespace
        ))?;
        write!(file, "{contents}")?;
    }
    for (path, contents) in compiled.loot_tables {
        let mut file = create_file_with_parent_dirs(&format!(
            "{nmsp}/data/{nmsp}/loot_tables/{path}.json",
            nmsp = args.namespace
        ))?;
        write!(file, "{contents}")?;
    }
    let mut mcmeta =
        create_file_with_parent_dirs(&format!("{nmsp}/pack.mcmeta", nmsp = args.namespace))?;
    write!(mcmeta, "{}", compiled.mcmeta)?;
    Ok(())
}

fn create_file_with_parent_dirs(filename: &str) -> Result<File, std::io::Error> {
    let parent_dir = std::path::Path::new(filename).parent().unwrap();
    fs::create_dir_all(parent_dir)?;

    File::create(filename)
}
