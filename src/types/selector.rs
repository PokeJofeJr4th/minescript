use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
};

use super::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectorType {
    /// self
    S,
    /// nearest player
    P,
    /// all entities
    E,
    /// all players
    A,
    /// random player
    R,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Selector<T> {
    pub selector_type: SelectorType,
    pub args: BTreeMap<RStr, T>,
}

impl<T: Debug> Debug for Selector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{:?}", self.selector_type)?;
        f.debug_list()
            .entries(
                self.args
                    .iter()
                    .map(|(name, value)| format!("{name}={value:?}")),
            )
            .finish()
    }
}

impl<T> Selector<T> {
    pub const fn s() -> Self {
        Self {
            selector_type: SelectorType::S,
            args: BTreeMap::new(),
        }
    }
    pub const fn p() -> Self {
        Self {
            selector_type: SelectorType::P,
            args: BTreeMap::new(),
        }
    }
    pub const fn e() -> Self {
        Self {
            selector_type: SelectorType::E,
            args: BTreeMap::new(),
        }
    }
    pub const fn a() -> Self {
        Self {
            selector_type: SelectorType::A,
            args: BTreeMap::new(),
        }
    }
    pub const fn r() -> Self {
        Self {
            selector_type: SelectorType::R,
            args: BTreeMap::new(),
        }
    }
    pub fn with_property<K: Into<RStr>>(mut self, k: K, v: T) -> Self {
        self.args.insert(k.into(), v);
        self
    }
}

impl Selector<Syntax> {
    pub fn stringify(&self) -> SResult<Selector<String>> {
        Ok(Selector {
            selector_type: self.selector_type,
            args: self
                .args
                .iter()
                .map(|(k, v)| {
                    if let ("nbt", Ok(nbt)) = (&**k, Nbt::try_from(v)) {
                        Ok(format!("{nbt}"))
                    } else {
                        v.to_selector_body()
                    }
                    .map(|v| (k.clone(), v))
                })
                .collect::<Result<BTreeMap<RStr, String>, _>>()
                .map_err(|err| format!("Couldn't convert to string in selector: {err}"))?,
        })
    }
}

impl<T: Display> Display for Selector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.args.is_empty() {
            write!(f, "{}", self.selector_type)
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
