use std::sync::Arc;

use bevy::asset::{AssetLoader, BoxedFuture, io::Reader, LoadContext};
use bevy::tasks::futures_lite::AsyncReadExt;
use hound::WavReader;

use crate::audio::audio_source::AudioSource;

use super::audio_source::AudioSourceDecoded;

#[derive(Default)]
pub struct WavLoader;

impl AssetLoader for WavLoader {
    type Asset = AudioSource;
    type Settings = ();
    type Error = anyhow::Error;

    fn load<'a, 'b>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'b>,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let mut reader = WavReader::new(std::io::Cursor::new(bytes))?;
            let hound::WavSpec {
                bits_per_sample,
                sample_format,
                sample_rate,
                channels,
            } = reader.spec();

            let samples: Result<Vec<f32>, _> = match sample_format {
                hound::SampleFormat::Int => {
                    let max_value = 2_u32.pow(bits_per_sample as u32 - 1) - 1;
                    reader
                        .samples::<i32>()
                        .map(|sample| sample.map(|sample| sample as f32 / max_value as f32))
                        .collect()
                }
                hound::SampleFormat::Float => reader.samples::<f32>().collect(),
            };

            let samples = samples?;

            Ok(AudioSource {
                bytes: Arc::new([]),
                decoded: Some(Arc::new(AudioSourceDecoded {
                    samples,
                    channel_count: channels as u32,
                    sample_rate,
                })),
                create_streaming_source_fn: |_| Err(anyhow::anyhow!("Unsupported")),
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wav"]
    }
}
