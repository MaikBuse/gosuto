use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures::StreamExt;
use livekit::track::RemoteAudioTrack;
use livekit::webrtc::audio_source::AudioSourceOptions;
use livekit::webrtc::audio_source::native::NativeAudioSource;
use livekit::webrtc::audio_stream::native::NativeAudioStream;
use livekit::webrtc::prelude::AudioFrame;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::config::AudioConfig;
use crate::event::{AppEvent, EventSender};

pub const SAMPLE_RATE: u32 = 48000;
pub const FRAME_SIZE: usize = 960; // 20ms at 48kHz

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
    playback_streams: Vec<SendStream>,
    playback_tasks: Vec<tokio::task::JoinHandle<()>>,
}

// ── Device enumeration ──────────────────────────────

fn find_input_device(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.input_devices().ok()?.find(|d| {
        d.description()
            .ok()
            .map(|desc| desc.name().to_string())
            .as_deref()
            == Some(name)
    })
}

fn find_output_device(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.output_devices().ok()?.find(|d| {
        d.description()
            .ok()
            .map(|desc| desc.name().to_string())
            .as_deref()
            == Some(name)
    })
}

fn resolve_input_device(config: &AudioConfig) -> Result<cpal::Device> {
    if let Some(ref name) = config.input_device {
        if let Some(dev) = find_input_device(name) {
            return Ok(dev);
        }
        warn!(
            "Configured input device '{}' not found, using default",
            name
        );
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
        warn!(
            "Configured output device '{}' not found, using default",
            name
        );
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
            playback_streams: Vec::new(),
            playback_tasks: Vec::new(),
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

    /// Start capture and return the NativeAudioSource for LiveKit publishing.
    pub fn start_capture(
        &mut self,
        config: &AudioConfig,
        transmitting: Arc<AtomicBool>,
    ) -> Result<NativeAudioSource> {
        self.running.store(true, Ordering::Relaxed);

        let source = NativeAudioSource::new(
            AudioSourceOptions::default(),
            SAMPLE_RATE,
            1, // mono
            FRAME_SIZE as u32,
        );

        match Self::build_capture_stream(source.clone(), self.running.clone(), config, transmitting)
        {
            Ok(stream) => {
                self.capture_stream = Some(SendStream(stream));
            }
            Err(e) => {
                self.running.store(false, Ordering::Relaxed);
                return Err(e);
            }
        }

        Ok(source)
    }

    /// Add playback for a remote audio track from LiveKit.
    pub fn add_playback(
        &mut self,
        remote_track: RemoteAudioTrack,
        config: &AudioConfig,
    ) -> Result<()> {
        let (stream, task) =
            Self::build_playback_stream(remote_track, self.running.clone(), config)?;
        self.playback_streams.push(SendStream(stream));
        self.playback_tasks.push(task);
        Ok(())
    }

    /// Stop all audio
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.capture_stream = None;
        self.playback_streams.clear();
        for task in self.playback_tasks.drain(..) {
            task.abort();
        }
        info!("Audio pipeline stopped");
    }

    /// Start a mic test stream that captures audio and sends RMS levels.
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
                let sum_sq: f32 = data
                    .iter()
                    .map(|&s| {
                        let v = s * vol;
                        v * v
                    })
                    .sum();
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

    /// Build the capture stream: cpal input → NativeAudioSource for LiveKit.
    fn build_capture_stream(
        source: NativeAudioSource,
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

        let running_flag = running.clone();
        let rt_handle = tokio::runtime::Handle::current();
        let input_volume = audio_config.input_volume;
        let voice_activity = audio_config.voice_activity;
        let sensitivity = audio_config.sensitivity;

        // Buffer for accumulating samples to form complete frames
        let sample_buffer = Arc::new(std::sync::Mutex::new(Vec::<i16>::with_capacity(
            FRAME_SIZE * 2,
        )));

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

                // Voice activity gate: compute RMS and skip if below threshold
                if voice_activity {
                    let sum_sq: f32 = data
                        .iter()
                        .map(|&s| {
                            let v = s * input_volume;
                            v * v
                        })
                        .sum();
                    let rms = (sum_sq / data.len().max(1) as f32).sqrt();
                    if rms < sensitivity {
                        return;
                    }
                }

                // Convert f32 to i16 with volume
                let samples: Vec<i16> = data
                    .iter()
                    .map(|&s| ((s * input_volume).clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                    .collect();

                let mut buf = sample_buffer.lock().unwrap();
                buf.extend_from_slice(&samples);

                // Process complete frames
                while buf.len() >= FRAME_SIZE {
                    let frame: Vec<i16> = buf.drain(..FRAME_SIZE).collect();
                    let source = source.clone();
                    rt_handle.spawn(async move {
                        let audio_frame = AudioFrame {
                            data: frame.into(),
                            sample_rate: SAMPLE_RATE,
                            num_channels: 1,
                            samples_per_channel: FRAME_SIZE as u32,
                        };
                        let _ = source.capture_frame(&audio_frame).await;
                    });
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

    /// Build the playback stream for a remote LiveKit audio track.
    fn build_playback_stream(
        remote_track: RemoteAudioTrack,
        running: Arc<AtomicBool>,
        audio_config: &AudioConfig,
    ) -> Result<(cpal::Stream, tokio::task::JoinHandle<()>)> {
        let device = resolve_output_device(audio_config)?;

        info!("Using output device: {:?}", device.description());

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            buffer_size: cpal::BufferSize::Default,
        };

        let output_volume = audio_config.output_volume;

        // Channel for decoded audio samples
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

        // Spawn a task to read from NativeAudioStream and forward to cpal
        let running_decode = running.clone();
        let task = tokio::spawn(async move {
            let mut audio_stream =
                NativeAudioStream::new(remote_track.rtc_track(), SAMPLE_RATE as i32, 1);

            loop {
                if !running_decode.load(Ordering::Relaxed) {
                    break;
                }

                match audio_stream.next().await {
                    Some(frame) => {
                        let f32_samples: Vec<f32> = frame
                            .data
                            .iter()
                            .map(|&s| s as f32 / i16::MAX as f32)
                            .collect();
                        let _ = audio_tx.send(f32_samples);
                    }
                    None => {
                        info!("Audio stream ended");
                        break;
                    }
                }
            }

            info!("Audio decode loop ended");
        });

        Ok((stream, task))
    }
}

impl Drop for AudioPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}
