use bevy::{
    app::{App, Plugin},
    asset::{Asset, AssetApp, Handle},
    ecs::system::SystemParamItem,
    reflect::TypePath,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssetUsages},
        texture::Image,
    },
};

#[derive(Debug, Clone, TypePath, Asset)]
pub struct DamageDigitMaterial {
    pub texture: Handle<Image>,
}

pub struct DamageDigitMaterialPlugin;

impl Plugin for DamageDigitMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<DamageDigitMaterial>()
            .add_plugins(RenderAssetPlugin::<DamageDigitMaterial>::default());
    }
}

#[derive(Debug, Clone)]
pub struct GpuDamageDigitMaterial {
    pub texture: Handle<Image>,
}

impl TypePath for GpuDamageDigitMaterial {
    fn type_path() -> &'static str {
        "rose_offline_client::damage_digit_material::GpuDamageDigitMaterial"
    }

    fn short_type_path() -> &'static str {
        "GpuDamageDigitMaterial"
    }
}

impl RenderAsset for DamageDigitMaterial {
    type PreparedAsset = GpuDamageDigitMaterial;
    type Param = ();

    fn asset_usage(&self) -> RenderAssetUsages {
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    }

    fn prepare_asset(
        self,
        _param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>> {
        Ok(GpuDamageDigitMaterial {
            texture: self.texture.clone(),
        })
    }
}
