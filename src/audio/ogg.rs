use bevy::asset::{AssetLoader, io::Reader, LoadContext};
use bevy::tasks::futures_lite::AsyncReadExt;
use lewton::{
    audio::AudioReadError, inside_ogg::OggStreamReader, samples::InterleavedSamples, VorbisError,
};

use crate::audio::audio_source::AudioSource;

use super::audio_source::StreamingAudioSource;

#[derive(Default)]
pub struct OggLoader;

impl AssetLoader for OggLoader {
    type Asset = AudioSource;
    type Settings = ();
    type Error = anyhow::Error;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(AudioSource {
            bytes: bytes.into(),
            decoded: None,
            create_streaming_source_fn: |audio_source| {
                OggAudioSource::new(audio_source)
                    .map(|source| Box::new(source) as Box<dyn StreamingAudioSource + Send + Sync>)
            },
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ogg"]
    }
}

struct OggAudioSource {
    reader: OggStreamReader<std::io::Cursor<AudioSource>>,
}

impl OggAudioSource {
    pub fn new(audio_source: &AudioSource) -> Result<Self, anyhow::Error> {
        Ok(Self {
            reader: OggStreamReader::new(std::io::Cursor::new(audio_source.clone()))?,
        })
    }
}

impl StreamingAudioSource for OggAudioSource {
    fn channel_count(&self) -> u32 {
        self.reader.ident_hdr.audio_channels as u32
    }

    fn sample_rate(&self) -> u32 {
        self.reader.ident_hdr.audio_sample_rate
    }

    fn rewind(&mut self) {
        // Seek back to start
        self.reader.seek_absgp_pg(0).ok();
    }

    fn read_packet(&mut self) -> Vec<f32> {
        loop {
            match self
                .reader
                .read_dec_packet_generic::<InterleavedSamples<f32>>()
            {
                Ok(Some(packet)) => {
                    if !packet.samples.is_empty() {
                        return packet.samples;
                    }
                }
                Err(VorbisError::BadAudio(AudioReadError::AudioIsHeader)) => {
                    continue;
                }
                Ok(_) | Err(_) => break,
            }
        }

        Vec::default()
    }
}
