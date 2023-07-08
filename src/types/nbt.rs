use std::{collections::BTreeMap, fmt::Display};

use super::prelude::*;

#[macro_export]
macro_rules! nbt {
    ({$($key:ident: $value:expr),*}) => {{
        let mut tree: std::collections::BTreeMap<$crate::types::RStr, $crate::types::Nbt> = std::collections::BTreeMap::new();
        $(
            tree.insert(stringify!($key).into(), nbt!($value));
        )*
        $crate::types::Nbt::Object(tree)
    }};

    ([$($value:expr),*]) => {{
        $crate::types::Nbt::Array(vec![$(nbt!($value)),*])
    }};

    ($obj:expr) => {
        $crate::types::Nbt::from($obj)
    }
}

#[derive(Debug, Clone)]
pub enum Nbt {
    Object(BTreeMap<RStr, Nbt>),
    Array(Vec<Nbt>),
    String(RStr),
    Integer(i32),
    Float(f32),
    Boolean(bool),
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
            Self::Object(data) => {
                let mut data_buf = f.debug_map();
                for (ident, value) in data {
                    data_buf.entry(&format_args!("{ident}"), &format_args!("{value}"));
                }
                data_buf.finish()
            }
            Self::Array(data) => {
                let mut data_buf = f.debug_list();
                for value in data {
                    data_buf.entry(&format_args!("{value}"));
                }
                data_buf.finish()
            }
            Self::String(str) => write!(f, "\"{str}\""),
            Self::Integer(num) => write!(f, "{num}"),
            Self::Float(float) => write!(f, "{float}"),
            Self::Boolean(bool) => write!(f, "{bool}"),
            Self::Unit => write!(f, "{{}}"),
        }
    }
}

impl Nbt {
    pub const TRUE: Self = Self::Boolean(true);
    // const FALSE: Self = Self::Boolean(false);

    pub fn to_json(&self) -> String {
        match self {
            Self::Object(obj) => {
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
            Self::Array(arr) => {
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
            Self::String(str) => format!("{str:?}"),
            Self::Integer(num) => format!("{num}"),
            Self::Float(float) => format!("{float}"),
            Self::Boolean(bool) => format!("{bool}"),
            Self::Unit => String::from("{}"),
        }
    }

    pub fn get_obj(&self) -> SResult<&BTreeMap<RStr, Self>> {
        match self {
            Self::Object(obj) => Ok(obj),
            _ => Err(format!("Expected an object; got `{self:?}`")),
        }
    }
}

impl TryFrom<&Syntax> for Nbt {
    type Error = String;

    fn try_from(value: &Syntax) -> SResult<Self> {
        match value {
            Syntax::Object(items) => {
                if items.is_empty() {
                    Ok(Self::default())
                } else {
                    Ok(Self::Object(
                        items
                            .iter()
                            .map(|(k, v)| Self::try_from(v).map(|nbt| (k.clone(), nbt)))
                            .collect::<SResult<BTreeMap<RStr, Self>>>()?,
                    ))
                }
            }
            Syntax::Array(items) => Ok(Self::Array(
                items
                    .iter()
                    .map(Self::try_from)
                    .collect::<SResult<Vec<Self>>>()?,
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

impl From<RStr> for Nbt {
    fn from(value: RStr) -> Self {
        Self::String(value)
    }
}

impl From<&RStr> for Nbt {
    fn from(value: &RStr) -> Self {
        Self::String(value.clone())
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

impl From<bool> for Nbt {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl<T> From<BTreeMap<RStr, T>> for Nbt
where
    Self: From<T>,
{
    fn from(value: BTreeMap<RStr, T>) -> Self {
        Self::Object(value.into_iter().map(|(k, v)| (k, Self::from(v))).collect())
    }
}

impl TryFrom<Syntax> for Nbt {
    type Error = String;
    fn try_from(value: Syntax) -> SResult<Self> {
        match value {
            Syntax::Object(obj) => Ok(Self::Object(
                obj.into_iter()
                    .map(|(k, v)| Self::try_from(v).map(|v| (k, v)))
                    .collect::<SResult<BTreeMap<RStr, Self>>>()?,
            )),
            Syntax::Array(arr) => Ok(Self::Array(
                arr.iter()
                    .map(Self::try_from)
                    .collect::<SResult<Vec<Self>>>()?,
            )),
            Syntax::String(str) | Syntax::Identifier(str) => Ok(Self::String(str)),
            Syntax::Integer(num) => Ok(Self::Integer(num)),
            Syntax::Unit => Ok(Self::Unit),
            _ => Err(format!("Can't turn `{value:?}` into Nbt")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NbtPathPart {
    Ident(RStr),
    Index(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NbtLocation {
    Entity(Selector<String>, NbtPath),
    Storage(NbtPath),
}

impl NbtLocation {
    pub fn stringify(&self, namespace: &str) -> String {
        match self {
            Self::Entity(sel, nbt) => format!("entity {sel} {}", fmt_nbt_path(nbt)),
            Self::Storage(nbt) => format!("storage {namespace} {}", fmt_nbt_path(nbt)),
        }
    }
}
