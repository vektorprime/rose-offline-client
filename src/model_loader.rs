use std::sync::Arc;

use arrayvec::ArrayVec;
use bevy::{
    math::{Mat4, Quat, Vec2, Vec3, Vec4},
    pbr::{ExtendedMaterial, MeshMaterial3d, StandardMaterial},
    prelude::{
        AssetServer, Assets, BuildChildren, Color, Commands, DespawnRecursiveExt, Entity,
        GlobalTransform, Handle, Image, Mesh, Mesh3d, Resource, Transform, Visibility,
    },
    render::{
        alpha::AlphaMode,
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes}, view::InheritedVisibility, view::NoFrustumCulling, view::ViewVisibility, primitives::Aabb,
        render_resource::Face,
        storage::ShaderStorageBuffer,
    },
};

use enum_map::{enum_map, EnumMap};

use rose_data::{
    CharacterMotionAction, CharacterMotionDatabase, EffectDatabase, ItemClass, ItemDatabase, NpcId,
    VehicleMotionAction, VehiclePartIndex, VehicleType,
};
use rose_data::{EquipmentIndex, ItemType, NpcDatabase};
use rose_file_readers::{ChrFile, VirtualFilesystem, ZmdFile, ZscFile};
use rose_game_common::components::{
    CharacterGender, CharacterInfo, DroppedItem, Equipment, EquipmentItemDatabase,
};

use crate::{
    animation::ZmoAsset,
    components::{
        CharacterModel, CharacterModelPart, CharacterModelPartIndex, DummyBoneOffset,
        ItemDropModel, NpcModel, PersonalStoreModel, VehicleModel,
    },
    // diagnostics::render_diagnostics::{log_alpha_blend_mesh_setup_simple},
    effect_loader::spawn_effect,
    render::{
        ParticleMaterial, TrailEffect, RoseEffectExtension,
        object_material_extension::RoseObjectExtension,
    },
};

const TRAIL_COLOURS: [Color; 9] = [
    Color::srgba(1.0, 0.0, 0.0, 1.0),
    Color::srgba(0.0, 1.0, 0.0, 1.0),
    Color::srgba(0.0, 0.0, 1.0, 1.0),
    Color::srgba(0.0, 0.0, 0.0, 1.0),
    Color::srgba(1.0, 1.0, 1.0, 1.0),
    Color::srgba(1.0, 1.0, 0.0, 1.0),
    Color::srgba(0.6, 0.6, 0.6, 1.0),
    Color::srgba(1.0, 0.0, 1.0, 1.0),
    Color::srgba(1.0, 0.5, 0.0, 1.0),
];

/// Helper function to create ExtendedMaterial for ROSE objects
pub fn create_rose_object_material(
    materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
    base_texture: Handle<Image>,
    lightmap_texture: Option<Handle<Image>>,
    specular_texture: Option<Handle<Image>>,
    lightmap_offset: Vec2,
    lightmap_scale: f32,
    alpha_mode: AlphaMode,
    two_sided: bool,
) -> Handle<ExtendedMaterial<StandardMaterial, RoseObjectExtension>> {
    materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(base_texture),
            alpha_mode,
            double_sided: two_sided,
            cull_mode: if two_sided { None } else { Some(Face::Back) },
            unlit: false,
            ..StandardMaterial::default()
        },
        extension: RoseObjectExtension {
            lightmap_params: Vec4::new(lightmap_offset.x, lightmap_offset.y, lightmap_scale, 0.0),
            lightmap_texture,
            specular_texture,
        },
    })
}

#[derive(Resource)]
pub struct ModelLoader {
    vfs: Arc<VirtualFilesystem>,
    character_motion_database: Arc<CharacterMotionDatabase>,
    effect_database: Arc<EffectDatabase>,
    item_database: Arc<ItemDatabase>,
    npc_database: Arc<NpcDatabase>,
    trail_effect_image: Handle<Image>,
    specular_image: Handle<Image>,

    // Male
    skeleton_male: ZmdFile,
    face_male: ZscFile,
    hair_male: ZscFile,
    head_male: ZscFile,
    body_male: ZscFile,
    arms_male: ZscFile,
    feet_male: ZscFile,

    // Female
    skeleton_female: ZmdFile,
    face_female: ZscFile,
    hair_female: ZscFile,
    head_female: ZscFile,
    body_female: ZscFile,
    arms_female: ZscFile,
    feet_female: ZscFile,

    // Gender neutral
    face_item: ZscFile,
    back: ZscFile,
    weapon: ZscFile,
    sub_weapon: ZscFile,

    // Vehicle
    skeleton_cart: ZmdFile,
    skeleton_castle_gear: ZmdFile,
    vehicle: ZscFile,

    // Npc
    npc_chr: ChrFile,
    npc_zsc: ZscFile,

    // Field Item
    field_item: ZscFile,
    field_item_motion_path: String,
}

impl ModelLoader {
    pub fn new(
        vfs: Arc<VirtualFilesystem>,
        character_motion_database: Arc<CharacterMotionDatabase>,
        effect_database: Arc<EffectDatabase>,
        item_database: Arc<ItemDatabase>,
        npc_database: Arc<NpcDatabase>,
        trail_effect_image: Handle<Image>,
        specular_image: Handle<Image>,
    ) -> Result<ModelLoader, anyhow::Error> {
        Ok(ModelLoader {
            // Male
            skeleton_male: vfs.read_file::<ZmdFile, _>("3DDATA/AVATAR/MALE.ZMD")?,
            face_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MFACE.ZSC")?,
            hair_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MHAIR.ZSC")?,
            head_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MCAP.ZSC")?,
            body_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MBODY.ZSC")?,
            arms_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MARMS.ZSC")?,
            feet_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MFOOT.ZSC")?,

            // Female
            skeleton_female: vfs.read_file::<ZmdFile, _>("3DDATA/AVATAR/FEMALE.ZMD")?,
            face_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WFACE.ZSC")?,
            hair_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WHAIR.ZSC")?,
            head_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WCAP.ZSC")?,
            body_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WBODY.ZSC")?,
            arms_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WARMS.ZSC")?,
            feet_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WFOOT.ZSC")?,

            // Gender neutral
            face_item: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_FACEIEM.ZSC")?, // Not a typo
            back: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_BACK.ZSC")?,
            weapon: vfs.read_file::<ZscFile, _>("3DDATA/WEAPON/LIST_WEAPON.ZSC")?,
            sub_weapon: vfs.read_file::<ZscFile, _>("3DDATA/WEAPON/LIST_SUBWPN.ZSC")?,

            // Vehicle
            skeleton_cart: vfs.read_file::<ZmdFile, _>("3DDATA/PAT/CART/CART01.ZMD")?,
            skeleton_castle_gear: vfs
                .read_file::<ZmdFile, _>("3DDATA/PAT/CASTLEGEAR/CASTLEGEAR02/CASTLEGEAR02.ZMD")?,
            vehicle: vfs.read_file::<ZscFile, _>("3DDATA/PAT/LIST_PAT.ZSC")?,

            // NPC
            npc_chr: vfs.read_file::<ChrFile, _>("3DDATA/NPC/LIST_NPC.CHR")?,
            npc_zsc: vfs.read_file::<ZscFile, _>("3DDATA/NPC/PART_NPC.ZSC")?,

            // Field items
            field_item: vfs.read_file::<ZscFile, _>("3DDATA/ITEM/LIST_FIELDITEM.ZSC")?,
            field_item_motion_path: "3DDATA/MOTION/ITEM_ANI.ZMO".to_string(),

            vfs,
            character_motion_database,
            effect_database,
            item_database,
            npc_database,
            trail_effect_image,
            specular_image,
        })
    }

    pub fn get_skeleton(&self, gender: CharacterGender) -> &ZmdFile {
        match gender {
            CharacterGender::Male => &self.skeleton_male,
            CharacterGender::Female => &self.skeleton_female,
        }
    }

    pub fn get_model_list(
        &self,
        gender: CharacterGender,
        model_part: CharacterModelPart,
    ) -> &ZscFile {
        match model_part {
            CharacterModelPart::CharacterFace => match gender {
                CharacterGender::Male => &self.face_male,
                CharacterGender::Female => &self.face_female,
            },
            CharacterModelPart::CharacterHair => match gender {
                CharacterGender::Male => &self.hair_male,
                CharacterGender::Female => &self.hair_female,
            },
            CharacterModelPart::FaceItem => &self.face_item,
            CharacterModelPart::Head => match gender {
                CharacterGender::Male => &self.head_male,
                CharacterGender::Female => &self.head_female,
            },
            CharacterModelPart::Body => match gender {
                CharacterGender::Male => &self.body_male,
                CharacterGender::Female => &self.body_female,
            },
            CharacterModelPart::Hands => match gender {
                CharacterGender::Male => &self.arms_male,
                CharacterGender::Female => &self.arms_female,
            },
            CharacterModelPart::Feet => match gender {
                CharacterGender::Male => &self.feet_male,
                CharacterGender::Female => &self.feet_female,
            },
            CharacterModelPart::Back => &self.back,
            CharacterModelPart::Weapon => &self.weapon,
            CharacterModelPart::SubWeapon => &self.sub_weapon,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_npc_model(
        &self,
        commands: &mut Commands,
        asset_server: &AssetServer,
        standard_materials: &mut Assets<bevy::pbr::StandardMaterial>,
        object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        skinned_mesh_inverse_bindposes_assets: &mut Assets<SkinnedMeshInverseBindposes>,
        particle_materials: &mut Assets<ParticleMaterial>,
        effect_mesh_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>,
        meshes: &mut Assets<Mesh>,
        storage_buffers: &mut Assets<ShaderStorageBuffer>,
        model_entity: Entity,
        npc_id: NpcId,
    ) -> Option<(NpcModel, SkinnedMesh, DummyBoneOffset)> {
        let npc_model_data = self.npc_chr.npcs.get(&npc_id.get())?;
        let (skinned_mesh, root_bone_position, dummy_bone_offset) = if let Some(skeleton) = self
            .npc_chr
            .skeleton_files
            .get(npc_model_data.skeleton_index as usize)
            .and_then(|p| self.vfs.read_file::<ZmdFile, _>(p).ok())
        {
            (
                spawn_skeleton(
                    commands,
                    model_entity,
                    &skeleton,
                    skinned_mesh_inverse_bindposes_assets,
                ),
                if let Some(root_bone) = skeleton.bones.first() {
                    Vec3::new(
                        root_bone.position.x,
                        root_bone.position.z,
                        -root_bone.position.y,
                    ) / 100.0
                } else {
                    Vec3::ZERO
                },
                skeleton.bones.len(),
            )
        } else {
            // CRITICAL FIX: NPC has no skeleton - do NOT add SkinnedMesh component
            // This prevents bind group mismatch errors when meshes don't have joint attributes
            log::warn!(
                "[SKINNED_MESH_FIX] NPC {} has no skeleton (skeleton_index: {}), spawning as non-skinned mesh to prevent bind group mismatch",
                npc_id.get(),
                npc_model_data.skeleton_index
            );
            return None;
        };

        let mut model_parts = Vec::with_capacity(16);
        for model_id in npc_model_data.model_ids.iter() {
            let mut parts = spawn_model(
                commands,
                asset_server,
                standard_materials,
                object_materials,
                model_entity,
                &self.npc_zsc,
                *model_id as usize,
                Some(&skinned_mesh),
                None,
                dummy_bone_offset,
                false,
                &self.specular_image,
            );
            model_parts.append(&mut parts);
        }

        for (link_dummy_bone_id, effect_id) in npc_model_data.effect_ids.iter() {
            if let Some(effect_path) = self.npc_chr.effect_files.get(*effect_id as usize) {
                if let Some(dummy_bone_entity) = skinned_mesh
                    .joints
                    .get(dummy_bone_offset + *link_dummy_bone_id as usize)
                {
                if let Some(effect_entity) = spawn_effect(
                        &self.vfs,
                        commands,
                        asset_server,
                        particle_materials,
                        effect_mesh_materials,
                        storage_buffers,
                        meshes,
                        effect_path.into(),
                        false,
                        None,
                    ) {
                        commands.entity(*dummy_bone_entity).add_child(effect_entity);
                        model_parts.push(effect_entity);
                    }
                }
            }
        }

            if let Some(npc_data) = self.npc_database.get_npc(npc_id) {
            if npc_data.right_hand_part_index != 0 {
                let mut parts = spawn_model(
                    commands,
                    asset_server,
                    standard_materials,
                    object_materials,
                    model_entity,
                    &self.weapon,
                    npc_data.right_hand_part_index as usize,
                    Some(&skinned_mesh),
                    None,
                    dummy_bone_offset,
                    false,
                    &self.specular_image,
                );
                model_parts.append(&mut parts);
            }

            if npc_data.left_hand_part_index != 0 {
                let mut parts = spawn_model(
                    commands,
                    asset_server,
                    standard_materials,
                    object_materials,
                    model_entity,
                    &self.sub_weapon,
                    npc_data.left_hand_part_index as usize,
                    Some(&skinned_mesh),
                    None,
                    dummy_bone_offset,
                    false,
                    &self.specular_image,
                );
                model_parts.append(&mut parts);
            }
        }

        let action_motions = enum_map! {
            action => {
                if let Some(motion_data) = self.npc_database.get_npc_action_motion(npc_id, action) {
                    asset_server.load(motion_data.path.path().to_string_lossy().into_owned())
                } else {
                    Handle::default()
                }
            }
        };

        Some((
            NpcModel {
                npc_id,
                model_parts,
                action_motions,
                root_bone_position,
            },
            skinned_mesh,
            DummyBoneOffset::new(dummy_bone_offset),
        ))
    }

    pub fn spawn_personal_store_model(
        &self,
        commands: &mut Commands,
        asset_server: &AssetServer,
        standard_materials: &mut Assets<bevy::pbr::StandardMaterial>,
        object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        model_entity: Entity,
        skin: usize,
    ) -> PersonalStoreModel {
        let root_bone = commands
            .spawn((
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Transform::default(),
                GlobalTransform::default(),
            ))
            .id();
        commands.entity(model_entity).add_child(root_bone);

        let model_parts = spawn_model(
            commands,
            asset_server,
            standard_materials,
            object_materials,
            root_bone,
            &self.field_item,
            260 + skin,
            None,
            None,
            0,
            false,
            &self.specular_image,
        );

        PersonalStoreModel {
            skin,
            model: root_bone,
            model_parts,
        }
    }

    pub fn spawn_item_drop_model(
        &self,
        commands: &mut Commands,
        asset_server: &AssetServer,
        standard_materials: &mut Assets<bevy::pbr::StandardMaterial>,
        object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        model_entity: Entity,
        dropped_item: Option<&DroppedItem>,
    ) -> (ItemDropModel, Handle<ZmoAsset>) {
        let model_id = match dropped_item {
            Some(DroppedItem::Item(item)) => self
                .item_database
                .get_base_item(item.get_item_reference())
                .map(|item_data| item_data.field_model_index)
                .unwrap_or(0) as usize,
            Some(DroppedItem::Money(_)) => 0,
            _ => 0,
        };

        let root_bone = commands
            .spawn((
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Transform::default(),
                GlobalTransform::default(),
            ))
            .id();
        commands.entity(model_entity).add_child(root_bone);

        (
            ItemDropModel {
                dropped_item: dropped_item.cloned(),
                root_bone,
                model_parts: spawn_model(
                    commands,
                    asset_server,
                    standard_materials,
                    object_materials,
                    root_bone,
                    &self.field_item,
                    model_id,
                    None,
                    None,
                    0,
                    false,
                    &self.specular_image,
                ),
            },
            asset_server.load(&self.field_item_motion_path),
        )
    }

    pub fn load_character_action_motions(
        &self,
        asset_server: &AssetServer,
        character_info: &CharacterInfo,
        equipment: &Equipment,
    ) -> EnumMap<CharacterMotionAction, Handle<ZmoAsset>> {
        let weapon_motion_type = self
            .item_database
            .get_equipped_weapon_item_data(equipment, EquipmentIndex::Weapon)
            .map(|item_data| item_data.motion_type)
            .unwrap_or(0) as usize;
        let gender_index = match character_info.gender {
            CharacterGender::Male => 0,
            CharacterGender::Female => 1,
        };
        let get_motion = |action| {
            if let Some(motion_data) = self.character_motion_database.get_character_action_motion(
                action,
                weapon_motion_type,
                gender_index,
            ) {
                return asset_server.load(motion_data.path.path().to_string_lossy().into_owned());
            }

            if gender_index == 1 {
                if let Some(motion_data) = self
                    .character_motion_database
                    .get_character_action_motion(action, weapon_motion_type, 0)
                {
                    return asset_server.load(motion_data.path.path().to_string_lossy().into_owned());
                }
            }

            if let Some(motion_data) =
                self.character_motion_database
                    .get_character_action_motion(action, 0, gender_index)
            {
                return asset_server.load(motion_data.path.path().to_string_lossy().into_owned());
            }

            if gender_index == 1 {
                if let Some(motion_data) = self
                    .character_motion_database
                    .get_character_action_motion(action, 0, 0)
                {
                    return asset_server.load(motion_data.path.path().to_string_lossy().into_owned());
                }
            }

            Handle::default()
        };
        enum_map! {
            action => get_motion(action),
        }
    }

    fn spawn_weapon_trail(
        &self,
        commands: &mut Commands,
        model_list: &ZscFile,
        model_id: usize,
        base_effect_index: usize,
        colour: Color,
        duration: f32,
    ) -> Option<Entity> {
        let object = model_list.objects.get(model_id)?;
        let start_position = object.effects.get(base_effect_index)?.position;
        let end_position = object.effects.get(base_effect_index + 1)?.position;
        Some(
            commands
                .spawn((
                    TrailEffect {
                        colour,
                        duration,
                        start_offset: Vec3::new(
                            start_position.x,
                            start_position.z,
                            -start_position.y,
                        ) / 100.0,
                        end_offset: Vec3::new(end_position.x, end_position.z, -end_position.y)
                            / 100.0,
                        trail_texture: self.trail_effect_image.clone_weak(),
                        distance_per_point: 10.0 / 100.0,
                    },
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ))
                .id(),
        )
    }

    // Gem effects temporarily disabled (use custom materials)
    /*
    fn spawn_character_gem_effect(
        &self,
        commands: &mut Commands,
        asset_server: &AssetServer,
        particle_materials: &mut Assets<ParticleMaterial>,
        effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
        model_list: &ZscFile,
        model_parts: &[Entity],
        item_model_id: usize,
        gem_item_number: usize,
        gem_position: usize,
    ) -> Option<Entity> {
        let gem_item = self.item_database.get_gem_item(gem_item_number)?;
        let gem_effect_id = gem_item.gem_effect_id?;
        let gem_effect = self.effect_database.get_effect(gem_effect_id)?;
        let effect_file_id = gem_effect.point_effects.get(0)?;
        let effect_file = self.effect_database.get_effect_file(*effect_file_id)?;

        let zsc_object = model_list.objects.get(item_model_id)?;
        let gem_effect_point = zsc_object.effects.get(gem_position)?;
        let parent_part_entity = model_parts.get(gem_effect_point.parent.unwrap_or(0) as usize)?;

        let effect_entity = spawn_effect(
            &self.vfs,
            commands,
            asset_server,
            particle_materials,
            effect_mesh_materials,
            effect_file.into(),
            false,
            None,
        )?;

        commands
            .entity(*parent_part_entity)
            .add_child(effect_entity);

        commands.entity(effect_entity).insert(
            Transform::from_translation(
                Vec3::new(
                    gem_effect_point.position.x,
                    gem_effect_point.position.z,
                    -gem_effect_point.position.y,
                ) / 100.0,
            )
            .with_rotation(Quat::from_xyzw(
                gem_effect_point.rotation.x,
                gem_effect_point.rotation.z,
                -gem_effect_point.rotation.y,
                gem_effect_point.rotation.w,
            )),
        );

        Some(effect_entity)
    }
    */

    pub fn spawn_character_weapon_trail(
        &self,
        commands: &mut Commands,
        equipment: &Equipment,
        weapon_bone_entity: Entity,
        subweapon_bone_entity: Entity,
    ) -> ArrayVec<Entity, 2> {
        let weapon_item_number = if let Some(weapon_item_number) = equipment.equipped_items
            [EquipmentIndex::Weapon]
            .as_ref()
            .map(|equipment_item| equipment_item.item.item_number)
        {
            weapon_item_number
        } else {
            return ArrayVec::default();
        };

        let weapon_item_data = if let Some(weapon_item_data) =
            self.item_database.get_weapon_item(weapon_item_number)
        {
            weapon_item_data
        } else {
            return ArrayVec::default();
        };

        let weapon_effect_data = if let Some(weapon_effect_data) = weapon_item_data
            .effect_id
            .and_then(|id| self.effect_database.get_effect(id))
        {
            weapon_effect_data
        } else {
            return ArrayVec::default();
        };

        let trail_colour_index =
            if let Some(trail_colour_index) = weapon_effect_data.trail_colour_index {
                trail_colour_index
            } else {
                return ArrayVec::default();
            };

        let colour = TRAIL_COLOURS
            .get(trail_colour_index.get())
            .cloned()
            .unwrap_or(Color::WHITE);

        let mut parts = ArrayVec::new();

        if let Some(trail_entity) = self.spawn_weapon_trail(
            commands,
            &self.weapon,
            weapon_item_number,
            0,
            colour,
            weapon_effect_data.trail_duration.as_secs_f32(),
        ) {
            commands.entity(weapon_bone_entity).add_child(trail_entity);
            parts.push(trail_entity);
        }

        // If the weapon is dual wield, add trail to the off hand weapon
        if matches!(
            weapon_item_data.item_data.class,
            ItemClass::Katar | ItemClass::DualSwords
        ) {
            if let Some(trail_entity) = self.spawn_weapon_trail(
                commands,
                &self.weapon,
                weapon_item_number,
                2,
                colour,
                weapon_effect_data.trail_duration.as_secs_f32(),
            ) {
                commands
                    .entity(subweapon_bone_entity)
                    .add_child(trail_entity);
                parts.push(trail_entity);
            }
        }

        parts
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_character_model(
        &self,
        commands: &mut Commands,
        asset_server: &AssetServer,
        standard_materials: &mut Assets<bevy::pbr::StandardMaterial>,
        object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        skinned_mesh_inverse_bindposes_assets: &mut Assets<SkinnedMeshInverseBindposes>,
        model_entity: Entity,
        character_info: &CharacterInfo,
        equipment: &Equipment,
    ) -> (CharacterModel, SkinnedMesh, DummyBoneOffset) {
        let skeleton = self.get_skeleton(character_info.gender);
        let dummy_bone_offset = skeleton.bones.len();
        let skinned_mesh = spawn_skeleton(
            commands,
            model_entity,
            skeleton,
            skinned_mesh_inverse_bindposes_assets,
        );
        let mut model_parts = EnumMap::default();

        for model_part in [
            CharacterModelPart::CharacterFace,
            CharacterModelPart::CharacterHair,
            CharacterModelPart::Head,
            CharacterModelPart::FaceItem,
            CharacterModelPart::Body,
            CharacterModelPart::Hands,
            CharacterModelPart::Feet,
            CharacterModelPart::Back,
            CharacterModelPart::Weapon,
            CharacterModelPart::SubWeapon,
        ] {
            if let Some(model_id) =
                get_model_part_index(&self.item_database, character_info, equipment, model_part)
            {
                model_parts[model_part] = (
                    model_id,
                    self.spawn_character_model_part(
                        character_info,
                        model_part,
                        commands,
                        asset_server,
                        standard_materials,
                        object_materials,
                        model_entity,
                        model_id.id,
                        &skinned_mesh,
                        dummy_bone_offset,
                        equipment,
                    ),
                );
            }
        }

        (
            CharacterModel {
                gender: character_info.gender,
                model_parts,
                action_motions: self.load_character_action_motions(
                    asset_server,
                    character_info,
                    equipment,
                ),
            },
            skinned_mesh,
            DummyBoneOffset::new(dummy_bone_offset),
        )
    }

    fn spawn_character_model_part(
        &self,
        character_info: &CharacterInfo,
        model_part: CharacterModelPart,
        commands: &mut Commands,
        asset_server: &AssetServer,
        standard_materials: &mut Assets<bevy::pbr::StandardMaterial>,
        object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        model_entity: Entity,
        model_id: usize,
        skinned_mesh: &SkinnedMesh,
        dummy_bone_offset: usize,
        equipment: &Equipment,
    ) -> Vec<Entity> {
        let model_list = self.get_model_list(character_info.gender, model_part);

        let mut model_parts = spawn_model(
            commands,
            asset_server,
            standard_materials,
            object_materials,
            model_entity,
            model_list,
            model_id,
            Some(skinned_mesh),
            model_part.default_bone_id(dummy_bone_offset),
            dummy_bone_offset,
            matches!(model_part, CharacterModelPart::CharacterFace),
            &self.specular_image,
        );

        if matches!(model_part, CharacterModelPart::Weapon) {
            let weapon_trail_entities = self.spawn_character_weapon_trail(
                commands,
                equipment,
                skinned_mesh.joints[dummy_bone_offset],
                skinned_mesh.joints[dummy_bone_offset + 1],
            );
            model_parts.extend(weapon_trail_entities.into_iter());
        }

        // Gem effects temporarily disabled (use custom materials)
        /*
        if matches!(model_part, CharacterModelPart::Weapon) {
            if let Some(item) = equipment.get_equipment_item(EquipmentIndex::Weapon) {
                if item.has_socket && item.gem > 300 {
                    if let Some(item_data) =
                        self.item_database.get_weapon_item(item.item.item_number)
                    {
                        if let Some(gem_effect_entity) = self.spawn_character_gem_effect(
                            commands,
                            asset_server,
                            particle_materials,
                            effect_mesh_materials,
                            model_list,
                            &model_parts,
                            model_id,
                            item.gem as usize,
                            item_data.gem_position as usize,
                        ) {
                            model_parts.push(gem_effect_entity);
                        }
                    }
                }
            }
        }

        if matches!(model_part, CharacterModelPart::SubWeapon) {
            if let Some(item) = equipment.get_equipment_item(EquipmentIndex::SubWeapon) {
                if item.has_socket && item.gem > 300 {
                    if let Some(item_data) = self
                        .item_database
                        .get_sub_weapon_item(item.item.item_number)
                    {
                        if let Some(gem_effect_entity) = self.spawn_character_gem_effect(
                            commands,
                            asset_server,
                            particle_materials,
                            effect_mesh_materials,
                            model_list,
                            &model_parts,
                            model_id,
                            item.gem as usize,
                            item_data.gem_position as usize,
                        ) {
                            model_parts.push(gem_effect_entity);
                        }
                    }
                }
            }
        }
        */

        model_parts
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_character_equipment(
        &self,
        commands: &mut Commands,
        asset_server: &AssetServer,
        standard_materials: &mut Assets<bevy::pbr::StandardMaterial>,
        object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        model_entity: Entity,
        character_info: &CharacterInfo,
        equipment: &Equipment,
        character_model: &mut CharacterModel,
        dummy_bone_offset: &DummyBoneOffset,
        skinned_mesh: &SkinnedMesh,
    ) {
        let weapon_model_index = get_model_part_index(
            &self.item_database,
            character_info,
            equipment,
            CharacterModelPart::Weapon,
        )
        .unwrap_or(CharacterModelPartIndex {
            id: 0,
            gem: 0,
            grade: 0,
        });
        if weapon_model_index.id != character_model.model_parts[CharacterModelPart::Weapon].0.id {
            character_model.action_motions =
                self.load_character_action_motions(asset_server, character_info, equipment);
        }

        for model_part in [
            CharacterModelPart::CharacterFace,
            CharacterModelPart::CharacterHair,
            CharacterModelPart::Head,
            CharacterModelPart::FaceItem,
            CharacterModelPart::Body,
            CharacterModelPart::Hands,
            CharacterModelPart::Feet,
            CharacterModelPart::Back,
            CharacterModelPart::Weapon,
            CharacterModelPart::SubWeapon,
        ] {
            let model_id =
                get_model_part_index(&self.item_database, character_info, equipment, model_part)
                    .unwrap_or(CharacterModelPartIndex {
                        id: 0,
                        gem: 0,
                        grade: 0,
                    });

            if model_id != character_model.model_parts[model_part].0 {
                // Despawn previous model
                for &entity in character_model.model_parts[model_part].1.iter() {
                    commands.entity(entity).despawn_recursive();
                }

                // Spawn new model
                if model_id.id != 0
                    || matches!(
                        model_part,
                        CharacterModelPart::CharacterHair | CharacterModelPart::CharacterFace
                    )
                {
                    character_model.model_parts[model_part] = (
                        model_id,
                        self.spawn_character_model_part(
                            character_info,
                            model_part,
                            commands,
                            asset_server,
                            standard_materials,
                            object_materials,
                            model_entity,
                            model_id.id,
                            skinned_mesh,
                            dummy_bone_offset.index,
                            equipment,
                        ),
                    );
                } else {
                    character_model.model_parts[model_part].0 = model_id;
                    character_model.model_parts[model_part].1.clear();
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_vehicle_model(
        &self,
        commands: &mut Commands,
        asset_server: &AssetServer,
        standard_materials: &mut Assets<bevy::pbr::StandardMaterial>,
        object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        skinned_mesh_inverse_bindposes_assets: &mut Assets<SkinnedMeshInverseBindposes>,
        particle_materials: &mut Assets<ParticleMaterial>,
        effect_mesh_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>,
        meshes: &mut Assets<Mesh>,
        storage_buffers: &mut Assets<ShaderStorageBuffer>,
        vehicle_model_entity: Entity,
        driver_model_entity: Entity,
        equipment: &Equipment,
    ) -> (VehicleModel, SkinnedMesh, DummyBoneOffset) {
        let body_item_data = equipment.equipped_vehicle[VehiclePartIndex::Body]
            .as_ref()
            .and_then(|equipment_item| {
                self.item_database
                    .get_vehicle_item(equipment_item.item.item_number)
            })
            .unwrap(); // TODO: No panic on invalid vehicle
        let is_cart = matches!(body_item_data.vehicle_type, VehicleType::Cart);
        let skeleton = match body_item_data.vehicle_type {
            VehicleType::Cart => &self.skeleton_cart,
            VehicleType::CastleGear => &self.skeleton_castle_gear,
        };
        let dummy_bone_offset = skeleton.bones.len();
        let skinned_mesh = spawn_skeleton(
            commands,
            vehicle_model_entity,
            skeleton,
            skinned_mesh_inverse_bindposes_assets,
        );
        let mut model_parts = EnumMap::default();

        for vehicle_part_index in [
            VehiclePartIndex::Body,
            VehiclePartIndex::Engine,
            VehiclePartIndex::Leg,
            VehiclePartIndex::Arms,
        ] {
            if let Some(model_id) = equipment.equipped_vehicle[vehicle_part_index]
                .as_ref()
                .map(|equipment_item| equipment_item.item.item_number)
            {
                model_parts[vehicle_part_index] = (
                    model_id,
                    spawn_model(
                        commands,
                        asset_server,
                        standard_materials,
                        object_materials,
                        vehicle_model_entity,
                        &self.vehicle,
                        model_id,
                        Some(&skinned_mesh),
                        None,
                        dummy_bone_offset,
                        false,
                        &self.specular_image,
                    ),
                );

                if let Some(item_data) = self.item_database.get_vehicle_item(model_id) {
                    for (dummy_index, effect_file_id) in
                        item_data.dummy_effect_file_ids.iter().enumerate()
                    {
                        if let Some(effect_path) =
                            effect_file_id.and_then(|id| self.effect_database.get_effect_file(id))
                        {
                            if let Some(dummy_bone_entity) =
                                skinned_mesh.joints.get(dummy_bone_offset + dummy_index)
                            {
                if let Some(effect_entity) = spawn_effect(
                        &self.vfs,
                        commands,
                        asset_server,
                        particle_materials,
                        effect_mesh_materials,
                        storage_buffers,
                        meshes,
                        effect_path.into(),
                        false,
                        None,
                    ) {
                                    commands.entity(*dummy_bone_entity).add_child(effect_entity);
                                    model_parts[vehicle_part_index].1.push(effect_entity);
                                }
                            }
                        }
                    }
                }
            }
        }

        let weapon_motion_type = equipment.equipped_vehicle[VehiclePartIndex::Arms]
            .as_ref()
            .and_then(|equipment_item| {
                self.item_database
                    .get_vehicle_item(equipment_item.item.item_number)
            })
            .map_or(0, |vehicle_item_data| {
                vehicle_item_data.base_motion_index as usize
            });

        (
            VehicleModel {
                driver_model_entity,
                driver_dummy_entity: skinned_mesh.joints[dummy_bone_offset],
                model_parts,
                vehicle_action_motions: enum_map! {
                    action =>  {
                        if let Some(motion_data) = self.character_motion_database.get_vehicle_action_motion(
                            if is_cart && matches!(action, VehicleMotionAction::Attack2 | VehicleMotionAction::Attack3) {
                                // Carts only have a single attack animation
                                VehicleMotionAction::Attack1
                            } else {
                                action
                            },
                            body_item_data.base_motion_index as usize,
                            weapon_motion_type,
                        ) {
                            asset_server.load(motion_data.path.path().to_string_lossy().into_owned())
                        } else {
                            Handle::default()
                        }
                    }
                },
                character_action_motions: enum_map! {
                    action =>  {
                        if let Some(motion_data) = self.character_motion_database.get_vehicle_action_motion(
                            if is_cart && matches!(action, VehicleMotionAction::Attack2 | VehicleMotionAction::Attack3) {
                                // Carts only have a single attack animation
                                VehicleMotionAction::Attack1
                            } else {
                                action
                            },
                            body_item_data.base_avatar_motion_index as usize,
                            0,
                        ) {
                            asset_server.load(motion_data.path.path().to_string_lossy().into_owned())
                        } else {
                            Handle::default()
                        }
                    }
                },
            },
            skinned_mesh,
            DummyBoneOffset::new(dummy_bone_offset),
        )
    }
}

trait DefaultBoneId {
    fn default_bone_id(&self, dummy_bone_offset: usize) -> Option<usize>;
}

impl DefaultBoneId for CharacterModelPart {
    fn default_bone_id(&self, dummy_bone_offset: usize) -> Option<usize> {
        match *self {
            CharacterModelPart::CharacterFace => Some(4),
            CharacterModelPart::CharacterHair => Some(4),
            CharacterModelPart::Head => Some(dummy_bone_offset + 6),
            CharacterModelPart::FaceItem => Some(dummy_bone_offset + 4),
            CharacterModelPart::Back => Some(dummy_bone_offset + 3),
            _ => None,
        }
    }
}

impl From<ItemType> for CharacterModelPart {
    fn from(item_type: ItemType) -> Self {
        match item_type {
            ItemType::Face => CharacterModelPart::FaceItem,
            ItemType::Head => CharacterModelPart::Head,
            ItemType::Body => CharacterModelPart::Body,
            ItemType::Hands => CharacterModelPart::Hands,
            ItemType::Feet => CharacterModelPart::Feet,
            ItemType::Back => CharacterModelPart::Back,
            ItemType::Weapon => CharacterModelPart::Weapon,
            ItemType::SubWeapon => CharacterModelPart::SubWeapon,
            _ => panic!("Invalid ItemType for CharacterModelPart"),
        }
    }
}

fn transform_children(skeleton: &ZmdFile, bone_transforms: &mut Vec<Transform>, bone_index: usize) {
    for (child_id, child_bone) in skeleton.bones.iter().enumerate() {
        if child_id == bone_index || child_bone.parent as usize != bone_index {
            continue;
        }

        bone_transforms[child_id] = bone_transforms[bone_index] * bone_transforms[child_id];
        transform_children(skeleton, bone_transforms, child_id);
    }
}

fn spawn_skeleton(
    commands: &mut Commands,
    model_entity: Entity,
    skeleton: &ZmdFile,
    skinned_mesh_inverse_bindposes_assets: &mut Assets<SkinnedMeshInverseBindposes>,
) -> SkinnedMesh {
    let mut bind_pose = Vec::with_capacity(skeleton.bones.len());
    let mut bone_entities = Vec::with_capacity(skeleton.bones.len());
    let dummy_bone_offset = skeleton.bones.len();

    for bone in skeleton.bones.iter().chain(skeleton.dummy_bones.iter()) {
        let position = Vec3::new(bone.position.x, bone.position.z, -bone.position.y) / 100.0;

        let rotation = Quat::from_xyzw(
            bone.rotation.x,
            bone.rotation.z,
            -bone.rotation.y,
            bone.rotation.w,
        );

        let transform = Transform::default()
            .with_translation(position)
            .with_rotation(rotation);

        bind_pose.push(transform);

        bone_entities.push(
            commands
                .spawn((
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    transform,
                    GlobalTransform::default(),
                ))
                .id(),
        );
    }

    // CRITICAL VALIDATION: Ensure skeleton has bones
    assert!(
        !skeleton.bones.is_empty(),
        "Skeleton has no bones! This will cause rendering errors."
    );

    assert_eq!(
        bone_entities.len(),
        skeleton.bones.len() + skeleton.dummy_bones.len(),
        "Bone entity count mismatch: expected {}, got {}",
        skeleton.bones.len() + skeleton.dummy_bones.len(),
        bone_entities.len()
    );

    assert_eq!(
        bind_pose.len(),
        bone_entities.len(),
        "Bind pose count mismatch: expected {}, got {}",
        bone_entities.len(),
        bind_pose.len()
    );

    // Apply parent-child transform hierarchy to calculate bind pose for each bone
    transform_children(skeleton, &mut bind_pose, 0);
    for (dummy_id, dummy_bone) in skeleton.dummy_bones.iter().enumerate() {
        bind_pose[dummy_id + dummy_bone_offset] =
            bind_pose[dummy_id + dummy_bone_offset] * bind_pose[dummy_bone.parent as usize];
    }

    let inverse_bind_pose: Vec<Mat4> = bind_pose
        .iter()
        .map(|x| x.compute_matrix().inverse())
        .collect();

    assert!(!inverse_bind_pose.is_empty(), "Skeleton has no inverse bind poses!");

    // Validate inverse bind poses are not degenerate
    for (i, matrix) in inverse_bind_pose.iter().enumerate() {
        if matrix.determinant().abs() < 0.0001 {
            log::warn!(
                "Skeleton bone {} has near-zero determinant inverse bind pose",
                i
            );
        }
    }

    for (i, bone) in skeleton
        .bones
        .iter()
        .chain(skeleton.dummy_bones.iter())
        .enumerate()
    {
        if let Some(&bone_entity) = bone_entities.get(i) {
            if bone.parent as usize == i {
                commands.entity(model_entity).add_child(bone_entity);
            } else if let Some(&parent_entity) = bone_entities.get(bone.parent as usize) {
                commands.entity(parent_entity).add_child(bone_entity);
            }
        }
    }

    let inverse_bindposes = SkinnedMeshInverseBindposes::from(inverse_bind_pose);
    let handle = skinned_mesh_inverse_bindposes_assets.add(inverse_bindposes);

    // VERIFY: Asset was actually added (critical for preventing bind group mismatch)
    if skinned_mesh_inverse_bindposes_assets.get(&handle).is_none() {
        log::error!(
           "[SKINNED_MESH_FIX] CRITICAL: Failed to add inverse bind poses asset to assets collection! This will cause a bind group mismatch!"
        );
        panic!("Failed to add inverse bind poses asset");
    }

    debug_assert!(
        skinned_mesh_inverse_bindposes_assets.get(&handle).is_some(),
        "Failed to add inverse bind poses asset"
    );

    // log::info!(
    //     "[SKINNED_MESH_FIX] Created skeleton with {} bones ({} real, {} dummy), inverse_bindposes asset loaded successfully",
    //     bone_entities.len(),
    //     skeleton.bones.len(),
    //     skeleton.dummy_bones.len()
    // );

    SkinnedMesh {
        inverse_bindposes: handle,
        joints: bone_entities,
    }
}

    #[allow(clippy::too_many_arguments)]
    fn spawn_model(
        commands: &mut Commands,
        asset_server: &AssetServer,
        standard_materials: &mut Assets<bevy::pbr::StandardMaterial>,
        object_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>,
        model_entity: Entity,
        model_list: &ZscFile,
        model_id: usize,
        skinned_mesh: Option<&SkinnedMesh>,
        default_bone_index: Option<usize>,
        dummy_bone_offset: usize,
        load_clip_faces: bool,
        specular_image: &Handle<Image>,
    ) -> Vec<Entity> {
    let mut parts = Vec::new();
    let object = if let Some(object) = model_list.objects.get(model_id) {
        object
    } else {
        return parts;
    };

        for object_part in object.parts.iter() {
        let mesh_id = object_part.mesh_id as usize;
        let mesh: Handle<Mesh> = asset_server.load(model_list.meshes[mesh_id].path().to_string_lossy().into_owned());
        let material_id = object_part.material_id as usize;
        let zsc_material = &model_list.materials[material_id];

        // Determine if material is alpha-blended
        // Alpha-blended materials have alpha enabled and z-write disabled
        let alpha_blended = zsc_material.alpha_enabled && !zsc_material.z_write_enabled;

        // Log alpha-blended mesh setup for diagnostic purposes (Crash #2)
        // DISABLED: if alpha_blended {
        //     log_alpha_blend_mesh_setup_simple(
        //         model_id,
        //         material_id,
        //         zsc_material.alpha_enabled,
        //         zsc_material.z_write_enabled,
        //         zsc_material.two_sided,
        //         zsc_material.is_skin,
        //     );
        // }

        // Create material using ExtendedMaterial<StandardMaterial, RoseObjectExtension>
        // Handle NULL texture paths for models
        let texture_path = zsc_material.path.path().to_string_lossy().into_owned();
        let texture_handle = if texture_path.is_empty() || texture_path == "NULL" {
            log::warn!("[SPAWN MODEL] NULL or empty texture path, using fallback");
            asset_server.load::<Image>("ETC/SPECULAR_SPHEREMAP.DDS")
        } else {
            asset_server.load::<Image>(&texture_path)
        };
        let material = create_rose_object_material(
            object_materials,
            texture_handle,
            None, // lightmap_texture
            Some(specular_image.clone()), // specular_texture
            Vec2::new(0.0, 0.0), // lightmap_offset
            1.0, // lightmap_scale
            if zsc_material.alpha_enabled {
                if let Some(threshold) = zsc_material.alpha_test {
                    AlphaMode::Mask(threshold)
                } else {
                    AlphaMode::Blend
                }
            } else {
                AlphaMode::Opaque
            }, // alpha_mode
            zsc_material.two_sided, // two_sided
        );

        let mut entity_commands = commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        // DIAGNOSTIC LOG: Log mesh spawning details
        log::info!(
            "[DIAGNOSTIC] Spawned mesh - model_id: {}, mesh_id: {}, material_id: {}, is_skin: {}, has_skinned_mesh_param: {}",
            model_id,
            mesh_id,
            material_id,
            zsc_material.is_skin,
            skinned_mesh.is_some()
        );

        if load_clip_faces {
            let zms_material_num_faces = crate::zms_asset_loader::ZmsMaterialNumFacesHandle(asset_server.load(format!(
                "{}#material_num_faces",
                model_list.meshes[mesh_id].path().to_string_lossy()
            )));
            entity_commands.insert(zms_material_num_faces);
        }

        if zsc_material.is_skin {
            if let Some(skinned_mesh) = skinned_mesh {
                // CRITICAL VALIDATION: Ensure SkinnedMesh has valid joints
                if skinned_mesh.joints.is_empty() {
                    log::error!(
                        "[SKINNED_MESH_FIX] CRITICAL: Mesh (model_id: {}, mesh_id: {}) is marked as is_skin but SkinnedMesh has no joints! This will cause a bind group mismatch!",
                        model_id,
                        mesh_id
                    );
                    // Do NOT insert SkinnedMesh to prevent crash
                } else {
                    // DIAGNOSTIC LOG: SkinnedMesh component inserted on mesh entity
                    // log::info!(
                    //     "[SKINNED_MESH_FIX] Inserting SkinnedMesh component on mesh entity - model_id: {}, mesh_id: {}, joints_count: {}",
                    //     model_id,
                    //     mesh_id,
                    //     skinned_mesh.joints.len()
                    // );
                    entity_commands.insert(skinned_mesh.clone());
                }
            } else {
                // DIAGNOSTIC LOG: Mesh marked as is_skin but no SkinnedMesh provided
                log::warn!(
                    "[SKINNED_MESH_FIX] Mesh (model_id: {}, mesh_id: {}) is marked as is_skin but no SkinnedMesh component was provided. Spawning as non-skinned mesh to prevent bind group mismatch!",
                    model_id,
                    mesh_id
                );
            }
        }

        let entity = entity_commands.id();

        let link_bone_entity = if let Some(skinned_mesh) = skinned_mesh {
            if let Some(bone_index) = object_part.bone_index {
                skinned_mesh.joints.get(bone_index as usize).cloned()
            } else if let Some(dummy_index) = object_part.dummy_index {
                skinned_mesh
                    .joints
                    .get(dummy_index as usize + dummy_bone_offset)
                    .cloned()
            } else if let Some(default_bone_index) = default_bone_index {
                skinned_mesh.joints.get(default_bone_index).cloned()
            } else {
                None
            }
        } else {
            None
        };

        commands
            .entity(link_bone_entity.unwrap_or(model_entity))
            .add_child(entity);

        parts.push(entity);
    }

    parts
}

fn get_model_part_index(
    item_database: &ItemDatabase,
    character_info: &CharacterInfo,
    equipment: &Equipment,
    model_part: CharacterModelPart,
) -> Option<CharacterModelPartIndex> {
    match model_part {
        CharacterModelPart::CharacterFace => Some(CharacterModelPartIndex {
            id: character_info.face as usize,
            gem: 0,
            grade: 0,
        }),
        CharacterModelPart::CharacterHair => {
            let head_hair_type = equipment.equipped_items[EquipmentIndex::Head]
                .as_ref()
                .map(|equipment_item| equipment_item.item.item_number)
                .and_then(|item_number| item_database.get_head_item(item_number))
                .map_or(0, |head_item_data| head_item_data.hair_type as usize);

            Some(CharacterModelPartIndex {
                id: character_info.hair as usize + head_hair_type,
                gem: 0,
                grade: 0,
            })
        }
        CharacterModelPart::Head => {
            equipment.equipped_items[EquipmentIndex::Head]
                .as_ref()
                .map(|equipment_item| CharacterModelPartIndex {
                    id: equipment_item.item.item_number,
                    gem: equipment_item.gem as usize,
                    grade: equipment_item.grade as usize,
                })
        }
        CharacterModelPart::FaceItem => equipment.equipped_items[EquipmentIndex::Face]
            .as_ref()
            .map(|equipment_item| CharacterModelPartIndex {
                id: equipment_item.item.item_number,
                gem: equipment_item.gem as usize,
                grade: equipment_item.grade as usize,
            }),
        CharacterModelPart::Body => Some(
            equipment.equipped_items[EquipmentIndex::Body]
                .as_ref()
                .map(|equipment_item| CharacterModelPartIndex {
                    id: equipment_item.item.item_number,
                    gem: equipment_item.gem as usize,
                    grade: equipment_item.grade as usize,
                })
                .unwrap_or(CharacterModelPartIndex {
                    id: 1,
                    gem: 0,
                    grade: 0,
                }),
        ),
        CharacterModelPart::Hands => Some(
            equipment.equipped_items[EquipmentIndex::Hands]
                .as_ref()
                .map(|equipment_item| CharacterModelPartIndex {
                    id: equipment_item.item.item_number,
                    gem: equipment_item.gem as usize,
                    grade: equipment_item.grade as usize,
                })
                .unwrap_or(CharacterModelPartIndex {
                    id: 1,
                    gem: 0,
                    grade: 0,
                }),
        ),
        CharacterModelPart::Feet => Some(
            equipment.equipped_items[EquipmentIndex::Feet]
                .as_ref()
                .map(|equipment_item| CharacterModelPartIndex {
                    id: equipment_item.item.item_number,
                    gem: equipment_item.gem as usize,
                    grade: equipment_item.grade as usize,
                })
                .unwrap_or(CharacterModelPartIndex {
                    id: 1,
                    gem: 0,
                    grade: 0,
                }),
        ),
        CharacterModelPart::Back => {
            equipment.equipped_items[EquipmentIndex::Back]
                .as_ref()
                .map(|equipment_item| CharacterModelPartIndex {
                    id: equipment_item.item.item_number,
                    gem: equipment_item.gem as usize,
                    grade: equipment_item.grade as usize,
                })
        }
        CharacterModelPart::Weapon => equipment.equipped_items[EquipmentIndex::Weapon]
            .as_ref()
            .map(|equipment_item| CharacterModelPartIndex {
                id: equipment_item.item.item_number,
                gem: equipment_item.gem as usize,
                grade: equipment_item.grade as usize,
            }),
        CharacterModelPart::SubWeapon => equipment.equipped_items[EquipmentIndex::SubWeapon]
            .as_ref()
            .map(|equipment_item| CharacterModelPartIndex {
                id: equipment_item.item.item_number,
                gem: equipment_item.gem as usize,
                grade: equipment_item.grade as usize,
            }),
    }
}
