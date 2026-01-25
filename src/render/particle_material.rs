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
pub struct ParticleMaterial {
    pub texture: Handle<Image>,
}

pub struct ParticleMaterialPlugin;

impl Plugin for ParticleMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ParticleMaterial>()
            .add_plugins(RenderAssetPlugin::<ParticleMaterial>::default());
    }
}

#[derive(Debug, Clone, TypePath)]
pub struct GpuParticleMaterial {
    pub texture: Handle<Image>,
}

impl RenderAsset for ParticleMaterial {
    type PreparedAsset = GpuParticleMaterial;
    type Param = ();

    fn asset_usage(&self) -> RenderAssetUsages {
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    }

    fn prepare_asset(
        self,
        _param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>> {
        Ok(GpuParticleMaterial {
            texture: self.texture,
        })
    }
}
