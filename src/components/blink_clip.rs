use bevy::prelude::{Component, Handle, Mesh};

/// Eye-blink variants of a character face mesh, added to the face part entity by
/// `character_model_blink_system` once the face mesh and its material split have loaded.
///
/// A face ZMS holds the closed eyelids as its first material and the open eyes as its last
/// (same face count). The original client clipped one of them from the index range at draw
/// time; here each variant is a copy of the face mesh without one of them, and the blink
/// swaps the part's `Mesh3d` between the two.
#[derive(Component)]
pub struct BlinkClipMeshes {
    /// The loaded face mesh with every face. Kept alive so faces spawned later reuse the
    /// loaded asset and the variants already derived from it.
    pub source: Handle<Mesh>,
    /// Every face except the first material's (closed eyelids).
    pub eyes_open: Handle<Mesh>,
    /// Every face except the last material's (open eyes).
    pub eyes_closed: Handle<Mesh>,
}

impl BlinkClipMeshes {
    pub fn get(&self, eyes_open: bool) -> &Handle<Mesh> {
        if eyes_open {
            &self.eyes_open
        } else {
            &self.eyes_closed
        }
    }
}
