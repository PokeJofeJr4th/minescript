use std::collections::BTreeMap;

use crate::types::prelude::*;

pub(super) fn effect(src: &Syntax) -> SResult<Vec<Command>> {
    let mut selector: Option<Selector<String>> = None;
    let mut effect = None;
    let mut duration = None;
    let mut level = 1;
    if let Syntax::Object(src) = src {
        for (prop, value) in src.iter() {
            match prop.as_ref() {
                "selector" | "target" => match value {
                    Syntax::Selector(sel) => {
                        selector = Some(sel.stringify()?);
                    }
                    other => {
                        return Err(format!(
                            "Unexpected element: `{other:?}`; expected a selector"
                        ))
                    }
                },
                "effect" => {
                    let Ok(eff) = RStr::try_from(value) else {
                        return Err(String::from("Potion effect must be a string"))
                    };
                    effect = Some(eff);
                }
                "duration" => match value {
                    Syntax::Identifier(str) | Syntax::String(str) => {
                        if *str != "infinite".into() {
                            return Err(format!(
                                "Potion duration should be an integer or infinite, not `{str}`"
                            ));
                        }
                    }
                    Syntax::Integer(num) => duration = Some(*num),
                    other => {
                        return Err(format!(
                            "Potion duration should be an integer or infinite, not `{other:?}`"
                        ))
                    }
                },
                "level" => match value {
                    Syntax::Integer(num) => level = *num,
                    other => {
                        return Err(format!(
                            "Potion level should be an integer, not `{other:?}`"
                        ))
                    }
                },
                other => return Err(format!("Unexpected potion property: `{other}`")),
            }
        }
    } else if let Ok(str) = RStr::try_from(src) {
        effect = Some(str);
    } else {
        return Err(format!("Expected an object for item macro; got `{src:?}`"));
    };

    let Some(effect) = effect else {
        return Err(String::from("Effect must include the effect id; {... effect: \"...\"}"))
    };

    Ok(vec![Command::EffectGive {
        target: selector.unwrap_or(Selector {
            selector_type: SelectorType::S,
            args: BTreeMap::new(),
        }),
        effect,
        duration,
        level,
    }])
}
