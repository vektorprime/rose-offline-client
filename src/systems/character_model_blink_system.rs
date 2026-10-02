use std::{collections::HashMap, ops::Range};

use bevy::{
    asset::{AssetId, LoadState},
    mesh::Indices,
    prelude::{
        AssetServer, Assets, Commands, Handle, Has, Local, Mesh, Mesh3d, Query, Res, ResMut, Time,
        Without,
    },
};
use rand::Rng;

use crate::{
    components::{
        BlinkClipMeshes, CharacterBlinkTimer, CharacterModel, CharacterModelPart, Dead,
        SkinningTarget,
    },
    zms_asset_loader::{ZmsMaterialNumFaces, ZmsMaterialNumFacesHandle},
};

/// Eyes-open / eyes-closed meshes derived from each source face mesh. Face meshes are
/// shared by every character with that face, so each is derived once. Stores asset ids
/// (reused through `Assets::get_strong_handle`), so the cache keeps no mesh alive.
#[derive(Default)]
pub struct BlinkMeshCache(HashMap<AssetId<Mesh>, (AssetId<Mesh>, AssetId<Mesh>)>);

pub fn character_model_blink_system(
    mut commands: Commands,
    mut query_characters: Query<(&CharacterModel, &mut CharacterBlinkTimer, Option<&Dead>)>,
    mut query_blink_faces: Query<(&mut Mesh3d, &BlinkClipMeshes)>,
    query_pending_faces: Query<
        (&Mesh3d, &ZmsMaterialNumFacesHandle, Has<SkinningTarget>),
        Without<BlinkClipMeshes>,
    >,
    num_faces_assets: Res<Assets<ZmsMaterialNumFaces>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut blink_mesh_cache: Local<BlinkMeshCache>,
    time: Res<Time>,
) {
    for (character_model, mut blink_timer, dead) in query_characters.iter_mut() {
        if dead.is_none() {
            blink_timer.timer += time.delta().as_secs_f32();

            if blink_timer.is_open {
                if blink_timer.timer >= blink_timer.open_duration {
                    blink_timer.is_open = false;
                    blink_timer.timer -= blink_timer.open_duration;
                    blink_timer.closed_duration =
                        rand::thread_rng().gen_range(CharacterBlinkTimer::BLINK_CLOSED_DURATION);
                }
            } else if blink_timer.timer >= blink_timer.closed_duration {
                blink_timer.is_open = true;
                blink_timer.timer -= blink_timer.closed_duration;
                blink_timer.open_duration =
                    rand::thread_rng().gen_range(CharacterBlinkTimer::BLINK_OPEN_DURATION);
            }
        } else if blink_timer.is_open {
            blink_timer.is_open = false;

            // Set timer so the eyes open as soon as resurrected
            blink_timer.closed_duration = 0.0;
            blink_timer.timer = 0.0;
        }

        let eyes_open = blink_timer.is_open;

        // Checked every frame rather than on open/closed transitions only, so respawned
        // face parts and faces whose mesh finished loading late get the current state.
        for &face_entity in character_model.model_parts[CharacterModelPart::CharacterFace]
            .1
            .iter()
        {
            if let Ok((mut mesh, blink_meshes)) = query_blink_faces.get_mut(face_entity) {
                // Compare first: writing Mesh3d re-extracts and re-specializes the entity.
                let target = blink_meshes.get(eyes_open);
                if mesh.0.id() != target.id() {
                    mesh.0 = target.clone();
                }
                continue;
            }

            let Ok((mesh, num_faces_handle, awaiting_skinning)) =
                query_pending_faces.get(face_entity)
            else {
                continue;
            };

            // SkinnedMeshFixPlugin decides on SkinnedMesh from the face's Mesh3d once the
            // asset server reports it loaded, which a derived mesh never is: swap only
            // after it has processed the face (its SkinnedMesh, if any, stays valid as the
            // derived meshes keep every vertex attribute).
            if awaiting_skinning || !meshes.contains(mesh.0.id()) {
                continue;
            }

            let Some(num_faces) = num_faces_assets.get(&num_faces_handle.0) else {
                if matches!(
                    asset_server.get_load_state(num_faces_handle.0.id()),
                    Some(LoadState::Failed(_))
                ) {
                    // No material split (the label is only written for meshes that have
                    // one): this face has no blink geometry and keeps every face.
                    commands
                        .entity(face_entity)
                        .try_remove::<ZmsMaterialNumFacesHandle>();
                }
                continue;
            };

            let Some(blink_meshes) = get_or_create_blink_meshes(
                &mut blink_mesh_cache,
                &mut meshes,
                &mesh.0,
                &num_faces.material_num_faces,
            ) else {
                log::warn!(
                    "[BLINK] Cannot split face mesh {:?} by its material face counts {:?}, eyes will not blink",
                    mesh.0.path(),
                    num_faces.material_num_faces
                );
                // Stop retrying: this face keeps every face.
                commands
                    .entity(face_entity)
                    .try_remove::<ZmsMaterialNumFacesHandle>();
                continue;
            };

            // The face switches to its variant from the next frame on (branch above). New
            // meshes only reach the render world once their Added event is flushed
            // (AssetEventSystems, unordered with this system); switching now could draw the
            // face with a mesh that is not prepared yet, i.e. not at all for a frame.
            // The num-faces handle stays on the face (as before) so the asset stays loaded
            // for the next spawn of this face.
            commands.entity(face_entity).try_insert(blink_meshes);
        }
    }
}

fn get_or_create_blink_meshes(
    cache: &mut BlinkMeshCache,
    meshes: &mut Assets<Mesh>,
    source: &Handle<Mesh>,
    material_num_faces: &[u16],
) -> Option<BlinkClipMeshes> {
    let source_id = source.id();
    if let Some(&(eyes_open_id, eyes_closed_id)) = cache.0.get(&source_id) {
        if let (Some(eyes_open), Some(eyes_closed)) = (
            meshes.get_strong_handle(eyes_open_id),
            meshes.get_strong_handle(eyes_closed_id),
        ) {
            return Some(BlinkClipMeshes {
                source: source.clone(),
                eyes_open,
                eyes_closed,
            });
        }
    }

    let source_mesh = meshes.get(source_id)?;
    let (eyes_open, eyes_closed) = create_blink_meshes(source_mesh, material_num_faces)?;
    let eyes_open = meshes.add(eyes_open);
    let eyes_closed = meshes.add(eyes_closed);

    // Forget variants that were freed, so the map does not grow each time a face mesh is
    // loaded again after every character using it was gone.
    cache.0.retain(|_, &mut (open_id, closed_id)| {
        meshes.contains(open_id) && meshes.contains(closed_id)
    });
    cache
        .0
        .insert(source_id, (eyes_open.id(), eyes_closed.id()));

    Some(BlinkClipMeshes {
        source: source.clone(),
        eyes_open,
        eyes_closed,
    })
}

/// Copies of `source` without the first material's faces (eyes open) and without the last
/// material's faces (eyes closed). The clip count is the last material's face count, as in
/// the original client (ZZ_CLIP_FACE_FIRST while open, ZZ_CLIP_FACE_LAST while closed).
/// Only the index buffer differs, so vertex attributes (joints included) and bounds match.
fn create_blink_meshes(source: &Mesh, material_num_faces: &[u16]) -> Option<(Mesh, Mesh)> {
    let num_clip_faces = *material_num_faces.last()? as usize;
    let indices = source.try_indices().ok()?;
    let num_indices = indices.len();
    let num_clip_indices = num_clip_faces * 3;
    let num_material_faces: usize = material_num_faces.iter().map(|&n| n as usize).sum();

    // The split must describe this mesh's triangles, and the two eye variants must not
    // overlap (a single-material mesh would otherwise lose every face).
    if num_clip_faces == 0
        || num_material_faces * 3 != num_indices
        || num_clip_indices * 2 > num_indices
    {
        return None;
    }

    let mut eyes_open = source.clone();
    eyes_open
        .try_insert_indices(slice_indices(indices, num_clip_indices..num_indices))
        .ok()?;
    let mut eyes_closed = source.clone();
    eyes_closed
        .try_insert_indices(slice_indices(indices, 0..num_indices - num_clip_indices))
        .ok()?;
    Some((eyes_open, eyes_closed))
}

fn slice_indices(indices: &Indices, range: Range<usize>) -> Indices {
    match indices {
        Indices::U16(indices) => Indices::U16(indices[range].to_vec()),
        Indices::U32(indices) => Indices::U32(indices[range].to_vec()),
    }
}
