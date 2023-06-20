mod command;
mod nbt;
mod selector;
mod syntax;
mod token;

pub use prelude::*;

pub mod prelude {
    use core::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    use std::rc::Rc;

    pub use super::command::{Command, Coordinate, ExecuteOption};
    pub use super::nbt::Nbt;
    pub use super::selector::{Selector, SelectorType};
    pub use super::syntax::{BlockSelectorType, BlockType, OpLeft, Operation, Syntax};
    pub use super::token::Token;
    pub use crate::nbt;

    pub type SResult<T> = Result<T, String>;
    pub type RStr = Rc<str>;

    pub fn get_hash<T: Hash>(obj: T) -> u64 {
        let mut hasher = DefaultHasher::new();
        obj.hash(&mut hasher);
        hasher.finish()
    }

    /// Given a float, find the fraction with the closest value with a denominator less than a certain amount
    pub fn farey_approximation(target: f32, max_denominator: i32) -> (i32, i32) {
        let mut a = (0, 1);
        let mut b = (1, 1);

        while b.1 <= max_denominator {
            let mediant_numerator = a.0 + b.0;
            let mediant_denominator = a.1 + b.1;

            let mediant_value = mediant_numerator as f32 / mediant_denominator as f32;

            if (mediant_value - target).abs() < f32::EPSILON {
                return (mediant_numerator, mediant_denominator);
            } else if mediant_value < target {
                a = (mediant_numerator, mediant_denominator);
            } else {
                b = (mediant_numerator, mediant_denominator);
            }
        }

        if (target - a.0 as f32 / a.1 as f32).abs() < (target - b.0 as f32 / b.1 as f32).abs() {
            a
        } else {
            b
        }
    }
}
