use bevy::prelude::{Entity, Message};

use rose_data::{EffectId, SkillId};

use super::blood_effect_event::BloodImpactProfile;

#[derive(Message)]
pub struct HitEvent {
    pub attacker: Entity,
    pub defender: Entity,
    pub effect_id: Option<EffectId>,
    pub skill_id: Option<SkillId>,
    pub blood_profile: BloodImpactProfile,
    pub apply_damage: bool,
    pub ignore_miss: bool,
}

impl HitEvent {
    pub fn with_weapon(attacker: Entity, defender: Entity, effect_id: Option<EffectId>) -> Self {
        Self {
            attacker,
            defender,
            effect_id,
            skill_id: None,
            blood_profile: BloodImpactProfile::Slash,
            apply_damage: true,
            ignore_miss: false,
        }
    }

    pub fn with_skill_damage(attacker: Entity, defender: Entity, skill_id: SkillId) -> Self {
        Self {
            attacker,
            defender,
            effect_id: None,
            skill_id: Some(skill_id),
            blood_profile: BloodImpactProfile::SkillMagic,
            apply_damage: true,
            ignore_miss: false,
        }
    }

    pub fn with_skill_effect(attacker: Entity, defender: Entity, skill_id: SkillId) -> Self {
        Self {
            attacker,
            defender,
            effect_id: None,
            skill_id: Some(skill_id),
            blood_profile: BloodImpactProfile::SkillMagic,
            apply_damage: true,
            ignore_miss: true,
        }
    }

    pub fn with_blood_profile(mut self, blood_profile: BloodImpactProfile) -> Self {
        self.blood_profile = blood_profile;
        self
    }

    pub fn apply_damage(mut self, apply_damage: bool) -> Self {
        self.apply_damage = apply_damage;
        self
    }
}
