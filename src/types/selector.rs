use std::{collections::BTreeMap, fmt::Display};

use super::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectorType {
    S,
    P,
    E,
    A,
    R,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Selector<T> {
    pub selector_type: SelectorType,
    pub args: BTreeMap<RStr, T>,
}

impl Selector<Syntax> {
    pub fn stringify(&self) -> Result<Selector<String>, String> {
        Ok(Selector {
            selector_type: self.selector_type,
            args: self
                .args
                .iter()
                .map(|(k, v)| String::try_from(v).map(|v| (k.clone(), v)))
                .collect::<Result<BTreeMap<RStr, String>, _>>()
                .map_err(|_| String::from("Couldn't convert to string in selector"))?,
        })
    }
}

impl<T: Display> Display for Selector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.args.is_empty() {
            self.selector_type.fmt(f)
        } else {
            write!(f, "{}", self.selector_type)?;
            let mut args_buf = f.debug_list();
            for (k, v) in &self.args {
                args_buf.entry(&format_args!("{k}={v}"));
            }
            args_buf.finish()
        }
    }
}

impl Display for SelectorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@{}",
            match self {
                Self::S => 's',
                Self::P => 'p',
                Self::E => 'e',
                Self::A => 'a',
                Self::R => 'r',
            }
        )
    }
}
