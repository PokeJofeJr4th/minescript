use std::{collections::BTreeMap, fmt::Display, rc::Rc};

use crate::parser::Syntax;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorType {
    S,
    P,
    E,
    A,
    R,
}

#[derive(Debug, Clone)]
pub struct Selector {
    pub selector_type: SelectorType,
    pub args: BTreeMap<Rc<str>, String>,
}

impl Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.args.is_empty() {
            write!(
                f,
                "@{}",
                match self.selector_type {
                    SelectorType::S => 's',
                    SelectorType::P => 'p',
                    SelectorType::E => 'e',
                    SelectorType::A => 'a',
                    SelectorType::R => 'r',
                },
            )
        } else {
            write!(
                f,
                "@{}",
                match self.selector_type {
                    SelectorType::S => 's',
                    SelectorType::P => 'p',
                    SelectorType::E => 'e',
                    SelectorType::A => 'a',
                    SelectorType::R => 'r',
                }
            )?;
            let mut args_buf = f.debug_list();
            for (k, v) in self.args.iter() {
                args_buf.entry(&format_args!("{k}={v}"));
            }
            args_buf.finish()
        }
    }
}

#[macro_export]
macro_rules! nbt {
    ({$($key:ident: $value:expr),*}) => {{
        let mut tree: BTreeMap<std::rc::Rc<str>, Nbt> = BTreeMap::new();
        $(
            tree.insert(stringify!($key).into(), nbt!($value));
        )*
        Nbt::Object(tree)
    }};

    ([$($value:expr),*]) => {{
        Nbt::Array(vec![$(nbt!($value)),*])
    }};

    ($obj:expr) => {
        Nbt::from($obj)
    }
}

#[derive(Debug, Clone)]
pub enum Nbt {
    Object(BTreeMap<Rc<str>, Nbt>),
    Array(Vec<Nbt>),
    String(Rc<str>),
    Integer(i32),
    Float(f32),
    Unit,
}

impl Default for Nbt {
    fn default() -> Self {
        Self::Unit
    }
}

impl Display for Nbt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Nbt::Object(data) => {
                let mut data_buf = f.debug_map();
                for (ident, value) in data {
                    data_buf.entry(&format_args!("{ident}"), &format_args!("{value}"));
                }
                data_buf.finish()
            }
            Nbt::Array(data) => {
                let mut data_buf = f.debug_list();
                for value in data {
                    data_buf.entry(&format_args!("{value}"));
                }
                data_buf.finish()
            }
            Nbt::String(str) => write!(f, "\"{str}\""),
            Nbt::Integer(num) => write!(f, "{num}"),
            Nbt::Float(float) => write!(f, "{float}"),
            Nbt::Unit => write!(f, "{{}}"),
        }
    }
}

impl Nbt {
    pub fn to_json(&self) -> String {
        match self {
            Nbt::Object(obj) => {
                let mut buf = String::from("{");
                for (key, value) in obj {
                    buf.push('"');
                    buf.push_str(key);
                    buf.push('"');
                    buf.push(':');
                    buf.push_str(&value.to_json());
                    buf.push(',');
                }
                // remove the last comma
                buf.pop();
                buf.push('}');
                buf
            }
            Nbt::Array(arr) => {
                let mut buf = String::from('[');
                for item in arr {
                    buf.push_str(&item.to_json());
                    buf.push(',');
                }
                // remove the last comma
                buf.pop();
                buf.push(']');
                buf
            }
            Nbt::String(str) => format!("\"{str}\""),
            Nbt::Integer(num) => format!("{num}"),
            Nbt::Float(float) => format!("{float}"),
            Nbt::Unit => String::from("{}"),
        }
    }
}

impl TryFrom<&Syntax> for Nbt {
    type Error = String;

    fn try_from(value: &Syntax) -> Result<Self, Self::Error> {
        match value {
            Syntax::Object(items) => {
                if items.is_empty() {
                    Ok(Self::default())
                } else {
                    Ok(Self::Object(
                        items
                            .iter()
                            .map(|(k, v)| Self::try_from(v).map(|nbt| (k.clone(), nbt)))
                            .collect::<Result<BTreeMap<Rc<str>, Self>, Self::Error>>()?,
                    ))
                }
            }
            Syntax::Array(items) => Ok(Self::Array(
                items
                    .iter()
                    .map(Self::try_from)
                    .collect::<Result<Vec<Self>, Self::Error>>()?,
            )),
            Syntax::String(str) => Ok(Self::String(str.clone())),
            Syntax::Integer(num) => Ok(Self::Integer(*num)),
            Syntax::Float(float) => Ok(Self::Float(*float)),
            Syntax::Unit => Ok(Self::default()),
            other => Err(format!("Can't make nbt from {other:?}")),
        }
    }
}

impl From<&str> for Nbt {
    fn from(value: &str) -> Self {
        Self::String(String::from(value).into())
    }
}

impl From<String> for Nbt {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<Rc<str>> for Nbt {
    fn from(value: Rc<str>) -> Self {
        Self::String(value)
    }
}

impl From<i32> for Nbt {
    fn from(value: i32) -> Self {
        Self::Integer(value)
    }
}

impl From<f32> for Nbt {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl<T> From<BTreeMap<Rc<str>, T>> for Nbt
where
    Nbt: From<T>,
{
    fn from(value: BTreeMap<Rc<str>, T>) -> Self {
        Self::Object(value.into_iter().map(|(k, v)| (k, Nbt::from(v))).collect())
    }
}

impl TryFrom<Syntax> for Nbt {
    type Error = String;
    fn try_from(value: Syntax) -> Result<Self, Self::Error> {
        match value {
            Syntax::Object(obj) => Ok(Self::Object(
                obj.into_iter()
                    .map(|(k, v)| Nbt::try_from(v).map(|v| (k, v)))
                    .collect::<Result<BTreeMap<Rc<str>, Nbt>, String>>()?,
            )),
            Syntax::Array(arr) => Ok(Self::Array(
                arr.iter()
                    .map(Nbt::try_from)
                    .collect::<Result<Vec<Nbt>, String>>()?,
            )),
            Syntax::String(str) | Syntax::Identifier(str) => Ok(Nbt::String(str)),
            Syntax::Integer(num) => Ok(Nbt::Integer(num)),
            Syntax::Unit => Ok(Nbt::Unit),
            _ => Err(format!("Can't turn `{value:?}` into Nbt")),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    EffectGive {
        target: Selector,
        effect: String,
        duration: Option<i32>,
        level: Option<i32>,
    },
    Kill {
        target: Selector,
    },
    Function {
        func: String,
    },
}

impl Command {
    pub fn stringify(&self, namespace: &str) -> String {
        match self {
            Command::EffectGive {
                target,
                effect,
                duration,
                level,
            } => {
                format!(
                    "effect give {target} {effect} {} {}",
                    match duration {
                        Some(num) => format!("{num}"),
                        None => String::from("infinite"),
                    },
                    level.unwrap_or(0)
                )
            }
            Command::Kill { target } => format!("kill {target}"),
            Command::Function { func } => format!("function {namespace}:{func}"),
        }
    }
}
