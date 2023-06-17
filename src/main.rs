#![warn(clippy::nursery, clippy::pedantic)]
use std::fs::{self, File};
use std::io::Write;
use std::rc::Rc;

use clap::Parser;

use crate::{interpreter::InterRep, parser::Syntax};

mod command;
mod compiler;
mod interpreter;
mod lexer;
mod parser;

type RStr = Rc<str>;

#[derive(Parser)]
struct Args {
    path: String,
    namespace: String,
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();
    let file = format!(
        "[{}]",
        fs::read_to_string(args.path).unwrap_or_else(|err| {
            println!("{err}");
            String::new()
        })
    );
    let tokens = lexer::tokenize(&file).unwrap_or_else(|err| {
        println!("{err}");
        Vec::new()
    });
    println!("{tokens:?}");
    let syntax = parser::parse(&mut tokens.into_iter().peekable()).unwrap_or_else(|err| {
        println!("{err}");
        Syntax::Unit
    });
    println!("{syntax:?}");
    let state = interpreter::interpret(&syntax).unwrap_or_else(|err| {
        println!("{err}");
        InterRep::new()
    });
    println!("{state:#?}");
    let compiled = compiler::compile(&state, &args.namespace).unwrap();
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
