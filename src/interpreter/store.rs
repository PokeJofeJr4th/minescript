use crate::{
    types::{Command, ExecuteOption, InterRepr, NbtLocation, OpLeft, SResult, VecCmd},
    Config,
};

pub(super) fn storage_op(
    lhs: &OpLeft,
    is_success: bool,
    cmd: VecCmd,
    hash: &str,
    state: &mut InterRepr,
    config: &Config,
) -> SResult<VecCmd> {
    let execute_options = if let (Ok(target), Ok(objective)) = (
        lhs.stringify_scoreboard_target(),
        lhs.stringify_scoreboard_objective(config),
    ) {
        vec![ExecuteOption::StoreScore {
            target,
            objective,
            is_success,
        }]
    } else {
        match lhs {
            OpLeft::SelectorDoubleColon(_, _) => {
                return Err(String::from("Can't assign a command result to an xp level"))
            }
            OpLeft::SelectorNbt(sel, nbt) => vec![ExecuteOption::StoreNBT {
                location: NbtLocation::Entity(sel.stringify()?, nbt.clone()),
                is_success,
            }],
            OpLeft::NbtStorage(nbt) => vec![ExecuteOption::StoreNBT {
                location: NbtLocation::Storage(nbt.clone()),
                is_success,
            }],
            _ => unreachable!(),
        }
    };
    Ok(Command::execute(&execute_options, cmd, hash, state).into_vec())
}
