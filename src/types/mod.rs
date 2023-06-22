mod command;
mod nbt;
mod repr;
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
    pub use super::repr::{CompiledRepr, InterRepr, Item};
    pub use super::selector::{Selector, SelectorType};
    pub use super::syntax::{
        BlockType, IdentBlockType, OpLeft, Operation, SelectorBlockType, Syntax,
    };
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
    #[allow(clippy::cast_possible_truncation)]
    pub fn farey_approximation(target: f32, max_denominator: i32) -> (i32, i32) {
        let mut a = (target.floor() as i32, 1);
        let mut b = (target.ceil() as i32, 1);

        while b.1 < max_denominator && a.1 < max_denominator {
            let mediant_numerator = a.0 + b.0;
            let mediant_denominator = a.1 + b.1;

            let mediant_value = mediant_numerator as f32 / mediant_denominator as f32;

            // println!("{a:?} < {mediant_value} < {b:?}");

            if (mediant_value - target).abs() < f32::EPSILON {
                return simplify_fraction((mediant_numerator, mediant_denominator));
            } else if mediant_value < target {
                a = (mediant_numerator, mediant_denominator);
            } else {
                b = (mediant_numerator, mediant_denominator);
            }
            // println!("{a:?} << {b:?}");
        }
        // println!("Returning on time");
        // println!("{a:?} << {b:?}");
        // println!("{} > {max_denominator} || {} > {max_denominator}", a.1, b.1);
        // make sure final answer is the closer one and inside the range
        if (target - b.0 as f32 / b.1 as f32).abs() < (target - a.0 as f32 / a.1 as f32).abs()
            && b.1 <= max_denominator
        {
            simplify_fraction(b)
        } else if a.1 <= max_denominator {
            simplify_fraction(a)
        } else {
            simplify_fraction(b)
        }
    }

    fn simplify_fraction((numerator, denominator): (i32, i32)) -> (i32, i32) {
        let gcd = euclidean_algorithm(numerator, denominator);
        (numerator / gcd, denominator / gcd)
    }

    fn euclidean_algorithm(a: i32, b: i32) -> i32 {
        if b == 0 {
            a
        } else {
            euclidean_algorithm(b, a % b)
        }
    }
}
