use bevy::prelude::Message;

use crate::scripting::lua4::Lua4Value;

#[derive(Message, Clone)]
pub enum SystemFuncEvent {
    CallFunction(String, Vec<Lua4Value>),
}
