use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Hash)]
pub struct Versioned<T> {
    base: T,
    mods: BTreeMap<u8, T>,
}

impl<T: Clone> Versioned<Vec<T>> {
    pub fn extend<I: IntoIterator<Item = T> + Clone>(&mut self, other: Versioned<I>) {
        self.map_with(std::iter::Extend::extend, other);
    }

    pub fn push(&mut self, other: Versioned<T>) {
        self.extend(other.map(Some));
    }

    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }
}

impl<T> Versioned<T> {
    pub fn map<F: FnMut(T) -> O, O>(self, mut func: F) -> Versioned<O> {
        Versioned {
            base: func(self.base),
            mods: self.mods.into_iter().map(|(k, v)| (k, func(v))).collect(),
        }
    }

    pub fn map_ref<F: FnMut(&T) -> O, O>(&self, mut func: F) -> Versioned<O> {
        Versioned {
            base: func(&self.base),
            mods: self.mods.iter().map(|(k, v)| (*k, func(v))).collect(),
        }
    }

    pub fn into_vec(self) -> Versioned<Vec<T>> {
        self.map(|t| vec![t])
    }

    pub fn get(&self, version: u8) -> &T {
        self.mods.get(&version).unwrap_or_else(|| {
            self.mods
                .range(..version)
                .next_back()
                .map_or(&self.base, |(_, v)| v)
        })
    }

    pub const fn base(&self) -> &T {
        &self.base
    }

    // pub fn get_mut(&mut self, version: u8) -> &mut T {
    //     if self.mods.contains_key(&version) {
    //         self.mods.get_mut(&version).unwrap()
    //     } else if let Some((_, v)) = self.mods.range_mut(..version).next_back() {
    //         v
    //     } else {
    //         &mut self.base
    //     }
    // }

    pub fn add_version(&mut self, version: u8, item: T) -> Option<T> {
        self.mods.insert(version, item)
    }

    pub const fn versions(&self) -> &BTreeMap<u8, T> {
        &self.mods
    }
}

impl<T: Clone> Versioned<T> {
    pub fn map_with<O: Clone, F>(&mut self, func: F, mut other: Versioned<O>)
    where
        F: Fn(&mut T, O),
    {
        let all_mods: Vec<u8> = self
            .mods
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            .union(&other.mods.keys().copied().collect())
            .copied()
            .collect();
        for version in all_mods {
            match (
                self.mods.contains_key(&version),
                other.mods.remove(&version),
            ) {
                (true, Some(other)) => func(self.mods.get_mut(&version).unwrap(), other),
                (true, None) => func(
                    self.mods.get_mut(&version).unwrap(),
                    other.get(version).clone(),
                ),
                (false, Some(other)) => {
                    let mut new = self.get(version).clone();
                    func(&mut new, other);
                    self.mods.insert(version, new);
                }
                (false, None) => {
                    let mut new = self.get(version).clone();
                    func(&mut new, other.get(version).clone());
                    self.mods.insert(version, new);
                }
            }
        }
        func(&mut self.base, other.base);
    }
}

impl<A, B> Versioned<(A, B)> {
    pub fn unzip(self) -> (Versioned<A>, Versioned<B>) {
        let mut out_a = Versioned::from(self.base.0);
        let mut out_b = Versioned::from(self.base.1);
        for (version, (item_a, item_b)) in self.mods {
            out_a.add_version(version, item_a);
            out_b.add_version(version, item_b);
        }
        (out_a, out_b)
    }
}

impl Versioned<String> {
    pub fn push(&mut self, ch: char) {
        self.base.push(ch);
        for version in self.mods.values_mut() {
            version.push(ch);
        }
    }

    pub fn push_str(&mut self, string: &str) {
        self.base.push_str(string);
        for version in self.mods.values_mut() {
            version.push_str(string);
        }
    }

    pub fn push_str_v(&mut self, strs: Self) {
        self.map_with(|mine, theirs| mine.push_str(&theirs), strs);
    }

    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }
}

// impl<T, E> Versioned<Result<T, E>> {
//     pub fn all(self) -> Result<Versioned<T>, E> {
//         Ok(Versioned {
//             base: self.base?,
//             mods: self
//                 .mods
//                 .into_iter()
//                 .map(|(v, res)| match res {
//                     Ok(t) => Ok((v, t)),
//                     Err(e) => Err(e),
//                 })
//                 .collect::<Result<BTreeMap<u8, T>, E>>()?,
//         })
//     }
// }

impl<T> From<T> for Versioned<T> {
    fn from(value: T) -> Self {
        Self {
            base: value,
            mods: BTreeMap::new(),
        }
    }
}

impl<T: Default> Default for Versioned<T> {
    fn default() -> Self {
        Self {
            base: T::default(),
            mods: BTreeMap::new(),
        }
    }
}
