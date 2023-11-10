use crate::{
    types::{Command, DataLocation, ExecuteOption, InterRepr, NbtLocation, SResult, VecCmd},
    Config,
};

pub(super) fn storage_op(
    lhs: DataLocation,
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
            DataLocation::SelectorDoubleColon(_, _) => {
                return Err(String::from("Can't assign a command result to an xp level"))
            }
            DataLocation::SelectorNbt(sel, nbt) => vec![ExecuteOption::StoreNBT {
                location: NbtLocation::Entity(sel.stringify()?, nbt),
                is_success,
                scale: 1.0,
            }],
            DataLocation::NbtStorage(nbt) => vec![ExecuteOption::StoreNBT {
                location: NbtLocation::Storage(nbt),
                is_success,
                scale: 1.0,
            }],
            _ => unreachable!(),
        }
    };
    Ok(Command::execute(&execute_options, cmd, hash, state).into_vec())
}
