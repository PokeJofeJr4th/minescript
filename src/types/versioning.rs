use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Hash)]
pub struct Versioned<T> {
    base: T,
    mods: BTreeMap<u8, T>,
}

impl<T: Clone> Versioned<Vec<T>> {
    pub fn extend<I: IntoIterator<Item = T> + Clone>(&mut self, mut iter: Versioned<I>) {
        let all_mods: Vec<u8> = self
            .mods
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            .union(&iter.mods.keys().copied().collect())
            .copied()
            .collect();
        for version in all_mods {
            match (self.mods.contains_key(&version), iter.mods.remove(&version)) {
                //  if we both do, just append
                (true, Some(theirs)) => self.mods.get_mut(&version).unwrap().extend(theirs),
                //  if I do but they don't, use their last mod?
                (true, None) => self
                    .mods
                    .get_mut(&version)
                    .unwrap()
                    .extend(iter.get(version).clone()),
                //  if I don't have it but they do, clone my base and add as a mod
                (false, Some(theirs)) => {
                    let mut mine = self.get(version).clone();
                    mine.extend(theirs);
                    self.mods.insert(version, mine);
                }
                // if we both don't, it won't be in `all_mods`
                (false, None) => unreachable!(),
            }
        }
        self.base.extend(iter.base);
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

    pub fn get(&self, version: u8) -> &T {
        self.mods.get(&version).unwrap_or_else(|| {
            self.mods
                .range(..version)
                .next_back()
                .map_or(&self.base, |(_, v)| v)
        })
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

    // pub fn add_version(&mut self, version: u8, item: T) -> Option<T> {
    //     self.mods.insert(version, item)
    // }
}

impl<T, E> Versioned<Result<T, E>> {
    pub fn all(self) -> Result<Versioned<T>, E> {
        Ok(Versioned {
            base: self.base?,
            mods: self
                .mods
                .into_iter()
                .map(|(v, res)| match res {
                    Ok(t) => Ok((v, t)),
                    Err(e) => Err(e),
                })
                .collect::<Result<BTreeMap<u8, T>, E>>()?,
        })
    }
}

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
