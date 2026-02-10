use bevy::prelude::Resource;
use std::fmt::Debug;

#[derive(Resource, Debug)]
pub enum LoginState {
    Input,
    WaitServerList,
    ServerSelect,
    JoiningServer,
}
