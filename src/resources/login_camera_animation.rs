use bevy::{
    prelude::{Handle, Resource},
    reflect::Reflect,
};

use crate::animation::ZmoAsset;

/// Resource holding the preloaded login screen camera animation.
/// This is loaded during PostStartup to ensure the animation is ready
/// before the login screen is entered, preventing the camera angle
/// race condition on initial load.
#[derive(Resource, Reflect)]
pub struct LoginCameraAnimation {
    pub handle: Handle<ZmoAsset>,
}
