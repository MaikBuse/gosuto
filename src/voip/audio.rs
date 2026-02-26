use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use audiopus::coder::{Decoder as OpusDecoder, Encoder as OpusEncoder};
use audiopus::{Application, Channels, MutSignals, SampleRate, packet::Packet};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use webrtc::media::Sample;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_remote::TrackRemote;

use crate::config::AudioConfig;
use crate::event::{AppEvent, EventSender};

const SAMPLE_RATE: u32 = 48000;
const FRAME_SIZE: usize = 960; // 20ms at 48kHz

/// Wrapper to make cpal::Stream Send.
/// cpal::Stream is !Send because the audio backend uses raw pointers internally,
/// but the stream itself is safe to move between threads (we only stop/drop it).
#[allow(dead_code)]
struct SendStream(cpal::Stream);

// SAFETY: cpal::Stream is !Send due to raw pointers in backend implementations,
// but the stream handle itself is safe to transfer between threads. We only
// call play/pause/drop on it, which are thread-safe operations.
unsafe impl Send for SendStream {}

pub struct AudioPipeline {
    running: Arc<AtomicBool>,
    capture_stream: Option<SendStream>,
    playback_stream: Option<SendStream>,
}

// ── Device enumeration ──────────────────────────────

fn find_input_device(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.input_devices()
        .ok()?
        .find(|d| d.description().ok().map(|desc| desc.name().to_string()).as_deref() == Some(name))
}

fn find_output_device(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.output_devices()
        .ok()?
        .find(|d| d.description().ok().map(|desc| desc.name().to_string()).as_deref() == Some(name))
}

fn resolve_input_device(config: &AudioConfig) -> Result<cpal::Device> {
    if let Some(ref name) = config.input_device {
        if let Some(dev) = find_input_device(name) {
            return Ok(dev);
        }
        warn!("Configured input device '{}' not found, using default", name);
    }
    cpal::default_host()
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No microphone found"))
}

fn resolve_output_device(config: &AudioConfig) -> Result<cpal::Device> {
    if let Some(ref name) = config.output_device {
        if let Some(dev) = find_output_device(name) {
            return Ok(dev);
        }
        warn!("Configured output device '{}' not found, using default", name);
    }
    cpal::default_host()
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No audio output device found"))
}

impl AudioPipeline {
    /// Create a new AudioPipeline (not yet started)
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            capture_stream: None,
            playback_stream: None,
        }
    }

    /// Enumerate available input device names
    pub fn enumerate_input_devices() -> Vec<String> {
        let host = cpal::default_host();
        host.input_devices()
            .map(|devs| {
                devs.filter_map(|d| d.description().ok().map(|desc| desc.name().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Enumerate available output device names
    pub fn enumerate_output_devices() -> Vec<String> {
        let host = cpal::default_host();
        host.output_devices()
            .map(|devs| {
                devs.filter_map(|d| d.description().ok().map(|desc| desc.name().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Start capture and return the running flag
    pub fn start(
        &mut self,
        local_track: Arc<TrackLocalStaticSample>,
        config: &AudioConfig,
        transmitting: Arc<AtomicBool>,
    ) -> Result<Arc<AtomicBool>> {
        self.running.store(true, Ordering::Relaxed);

        match Self::start_capture(local_track, self.running.clone(), config, transmitting) {
            Ok(stream) => {
                self.capture_stream = Some(SendStream(stream));
            }
            Err(e) => {
                self.running.store(false, Ordering::Relaxed);
                return Err(e);
            }
        }

        Ok(self.running.clone())
    }

    /// Add playback for a remote track
    pub fn add_playback(
        &mut self,
        remote_track: Arc<TrackRemote>,
        config: &AudioConfig,
    ) -> Result<()> {
        let stream = Self::start_playback(remote_track, self.running.clone(), config)?;
        self.playback_stream = Some(SendStream(stream));
        Ok(())
    }

    /// Stop all audio
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.capture_stream = None;
        self.playback_stream = None;
        info!("Audio pipeline stopped");
    }

    /// Start a mic test stream that captures audio and sends RMS levels.
    /// Runs on the calling thread's audio context. Returns when `running` is set to false.
    pub fn start_mic_test(
        device_name: Option<&str>,
        volume: f32,
        level_tx: EventSender,
        running: Arc<AtomicBool>,
    ) -> Result<()> {
        let host = cpal::default_host();
        let device = if let Some(name) = device_name {
            find_input_device(name)
                .or_else(|| host.default_input_device())
                .ok_or_else(|| anyhow::anyhow!("No microphone found"))?
        } else {
            host.default_input_device()
                .ok_or_else(|| anyhow::anyhow!("No microphone found"))?
        };

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            buffer_size: cpal::BufferSize::Default,
        };

        let running_flag = running.clone();
        let vol = volume;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !running_flag.load(Ordering::Relaxed) {
                    return;
                }
                // Compute RMS
                let sum_sq: f32 = data.iter().map(|&s| {
                    let v = s * vol;
                    v * v
                }).sum();
                let rms = (sum_sq / data.len().max(1) as f32).sqrt();
                let _ = level_tx.send(AppEvent::MicLevel(rms.clamp(0.0, 1.0)));
            },
            move |err| {
                error!("Mic test stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        info!("Mic test started");

        // Block until running is set to false
        while running.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
        }

        drop(stream);
        info!("Mic test stopped");
        Ok(())
    }

    /// Start the audio capture pipeline.
    /// Captures from configured input device, Opus-encodes, and writes to the WebRTC track.
    fn start_capture(
        local_track: Arc<TrackLocalStaticSample>,
        running: Arc<AtomicBool>,
        audio_config: &AudioConfig,
        transmitting: Arc<AtomicBool>,
    ) -> Result<cpal::Stream> {
        let device = resolve_input_device(audio_config)?;

        info!("Using input device: {:?}", device.description());

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            buffer_size: cpal::BufferSize::Default,
        };

        let encoder = OpusEncoder::new(
            SampleRate::Hz48000,
            Channels::Mono,
            Application::Voip,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create Opus encoder: {:?}", e))?;
        let encoder = Arc::new(std::sync::Mutex::new(encoder));

        let running_flag = running.clone();
        let rt_handle = tokio::runtime::Handle::current();
        let input_volume = audio_config.input_volume;
        let voice_activity = audio_config.voice_activity;
        let sensitivity = audio_config.sensitivity;

        // Buffer for accumulating samples to form complete frames
        let sample_buffer = Arc::new(std::sync::Mutex::new(Vec::<i16>::with_capacity(FRAME_SIZE * 2)));

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !running_flag.load(Ordering::Relaxed) {
                    return;
                }

                // PTT gate: if not transmitting, skip
                if !transmitting.load(Ordering::Relaxed) {
                    return;
                }

                // Convert f32 to i16 with volume
                let samples: Vec<i16> = data
                    .iter()
                    .map(|&s| ((s * input_volume).clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                    .collect();

                // Voice activity gate: compute RMS and skip if below threshold
                if voice_activity {
                    let sum_sq: f32 = data.iter().map(|&s| {
                        let v = s * input_volume;
                        v * v
                    }).sum();
                    let rms = (sum_sq / data.len().max(1) as f32).sqrt();
                    if rms < sensitivity {
                        return;
                    }
                }

                let mut buf = sample_buffer.lock().unwrap();
                buf.extend_from_slice(&samples);

                // Process complete frames
                while buf.len() >= FRAME_SIZE {
                    let frame: Vec<i16> = buf.drain(..FRAME_SIZE).collect();
                    let mut opus_buf = vec![0u8; 4000];
                    let enc = encoder.lock().unwrap();
                    match enc.encode(&frame, &mut opus_buf) {
                        Ok(len) => {
                            opus_buf.truncate(len);
                            let track = local_track.clone();
                            let data = opus_buf;
                            rt_handle.spawn(async move {
                                let sample = Sample {
                                    data: data.into(),
                                    duration: Duration::from_millis(20),
                                    ..Default::default()
                                };
                                if let Err(e) = track.write_sample(&sample).await {
                                    error!("Failed to write audio sample: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Opus encode error: {:?}", e);
                        }
                    }
                }
            },
            move |err| {
                error!("Audio input stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        info!("Audio capture started");

        Ok(stream)
    }

    /// Start the audio playback pipeline.
    /// Reads RTP from remote track, Opus-decodes, and plays through configured output device.
    fn start_playback(
        remote_track: Arc<TrackRemote>,
        running: Arc<AtomicBool>,
        audio_config: &AudioConfig,
    ) -> Result<cpal::Stream> {
        let device = resolve_output_device(audio_config)?;

        info!("Using output device: {:?}", device.description());

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            buffer_size: cpal::BufferSize::Default,
        };

        let output_volume = audio_config.output_volume;

        // Ring buffer for decoded audio samples
        let (audio_tx, audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();
        let audio_rx = Arc::new(std::sync::Mutex::new(audio_rx));

        let playback_buffer = Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
        let playback_buf_clone = playback_buffer.clone();
        let running_flag = running.clone();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if !running_flag.load(Ordering::Relaxed) {
                    data.fill(0.0);
                    return;
                }

                // Drain any new decoded samples into the buffer
                let mut rx = audio_rx.lock().unwrap();
                while let Ok(samples) = rx.try_recv() {
                    playback_buf_clone.lock().unwrap().extend(samples);
                }
                drop(rx);

                let mut buf = playback_buf_clone.lock().unwrap();
                let available = buf.len().min(data.len());
                if available > 0 {
                    // Apply output volume
                    for (i, sample) in buf.drain(..available).enumerate() {
                        data[i] = (sample * output_volume).clamp(-1.0, 1.0);
                    }
                }
                // Fill remaining with silence
                data[available..].fill(0.0);
            },
            move |err| {
                error!("Audio output stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        info!("Audio playback started");

        // Spawn a task to read RTP from remote track and decode
        let running_decode = running.clone();
        tokio::spawn(async move {
            let mut decoder = match OpusDecoder::new(SampleRate::Hz48000, Channels::Mono) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to create Opus decoder: {:?}", e);
                    return;
                }
            };

            let mut decode_buf = vec![0i16; FRAME_SIZE * 2];

            loop {
                if !running_decode.load(Ordering::Relaxed) {
                    break;
                }

                match remote_track.read_rtp().await {
                    Ok((rtp_packet, _)) => {
                        let payload = &rtp_packet.payload;
                        if payload.is_empty() {
                            continue;
                        }

                        let packet = match Packet::try_from(payload.as_ref()) {
                            Ok(p) => p,
                            Err(e) => {
                                warn!("Invalid Opus packet: {:?}", e);
                                continue;
                            }
                        };

                        let output = match MutSignals::try_from(decode_buf.as_mut_slice()) {
                            Ok(s) => s,
                            Err(e) => {
                                warn!("MutSignals error: {:?}", e);
                                continue;
                            }
                        };

                        match decoder.decode(Some(packet), output, false) {
                            Ok(decoded_samples) => {
                                let f32_samples: Vec<f32> = decode_buf[..decoded_samples]
                                    .iter()
                                    .map(|&s| s as f32 / i16::MAX as f32)
                                    .collect();
                                let _ = audio_tx.send(f32_samples);
                            }
                            Err(e) => {
                                warn!("Opus decode error: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        if running_decode.load(Ordering::Relaxed) {
                            error!("RTP read error: {}", e);
                        }
                        break;
                    }
                }
            }

            info!("Audio decode loop ended");
        });

        Ok(stream)
    }
}

impl Drop for AudioPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}
