use std::sync::Arc;

use super::audio_source::{AudioSource, AudioSourceDecoded, StreamingAudioSource};

pub enum StreamingSound {
    Streaming {
        source: Box<dyn StreamingAudioSource + Send + Sync>,
        buffer: Vec<f32>,
    },
    Buffered {
        decoded: Arc<AudioSourceDecoded>,
        position: usize,
    },
}

impl StreamingSound {
    pub fn new(audio_source: &AudioSource) -> Self {
        if let Some(decoded) = audio_source.decoded.clone() {
            Self::Buffered {
                decoded,
                position: 0,
            }
        } else {
            Self::Streaming {
                source: audio_source.create_streaming_source().unwrap(),
                buffer: Vec::with_capacity(1024),
            }
        }
    }

    pub fn channel_count(&self) -> u32 {
        match self {
            StreamingSound::Streaming { source, .. } => source.channel_count(),
            StreamingSound::Buffered { decoded, .. } => decoded.channel_count,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        match self {
            StreamingSound::Streaming { source, .. } => source.sample_rate(),
            StreamingSound::Buffered { decoded, .. } => decoded.sample_rate,
        }
    }

    pub fn fill_mono(&mut self, stream: &mut oddio::StreamControl<f32>, repeating: bool) -> bool {
        self.fill(|samples| stream.write(samples), repeating)
    }

    pub fn fill_stereo(
        &mut self,
        stream: &mut oddio::StreamControl<[f32; 2]>,
        repeating: bool,
    ) -> bool {
        self.fill(|samples| stream.write(stereo_frames(samples)) * 2, repeating)
    }

    fn fill(&mut self, mut write: impl FnMut(&[f32]) -> usize, repeating: bool) -> bool {
        match self {
            StreamingSound::Streaming { source, buffer } => {
                if !buffer.is_empty() {
                    let samples_read = write(buffer);
                    buffer.drain(0..samples_read);
                }

                if buffer.is_empty() {
                    let mut did_repeat = false;

                    loop {
                        let packet = source.read_packet();
                        if packet.is_empty() {
                            if repeating {
                                if !did_repeat {
                                    source.rewind();
                                    did_repeat = true;
                                    continue;
                                } else {
                                    return false; // Encountered an error
                                }
                            } else {
                                return false; // Reached end of stream
                            }
                        }

                        let samples_read = write(&packet);
                        if samples_read == packet.len() {
                            continue;
                        } else {
                            buffer.extend_from_slice(&packet[samples_read..]);
                            break;
                        }
                    }
                }

                true
            }
            StreamingSound::Buffered { decoded, position } => {
                loop {
                    if *position == decoded.samples.len() {
                        if repeating {
                            *position = 0;
                        } else {
                            // Reached end of stream
                            break false;
                        }
                    }

                    let samples_read = write(&decoded.samples[*position..]);
                    *position += samples_read;

                    if *position < decoded.samples.len() {
                        // stream internal buffer full, read more later
                        break true;
                    }
                }
            }
        }
    }
}

fn stereo_frames(samples: &[f32]) -> &[[f32; 2]] {
    unsafe { core::slice::from_raw_parts(samples.as_ptr() as *const [f32; 2], samples.len() / 2) }
}
