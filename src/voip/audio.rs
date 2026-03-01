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
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

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
    capture_task: Option<tokio::task::JoinHandle<()>>,
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

// ── Audio format helpers ──────────────────────────────

/// Check if an output device supports a specific sample rate.
fn can_device_support_output_rate(device: &cpal::Device, rate: u32) -> bool {
    device.supported_output_configs().is_ok_and(|configs| {
        configs
            .into_iter()
            .any(|c| c.min_sample_rate() <= rate && rate <= c.max_sample_rate())
    })
}

/// Check if an input device supports a specific sample rate.
fn can_device_support_input_rate(device: &cpal::Device, rate: u32) -> bool {
    device.supported_input_configs().is_ok_and(|configs| {
        configs
            .into_iter()
            .any(|c| c.min_sample_rate() <= rate && rate <= c.max_sample_rate())
    })
}

/// Resolve the output stream config, preferring 48kHz if the device supports it.
/// Returns (StreamConfig, device_channels, actual_sample_rate).
fn resolve_output_stream_config(device: &cpal::Device) -> Result<(cpal::StreamConfig, u16, u32)> {
    let default_config = device.default_output_config()?;
    let device_channels = default_config.channels();

    let sample_rate = if can_device_support_output_rate(device, SAMPLE_RATE) {
        SAMPLE_RATE
    } else {
        default_config.sample_rate()
    };

    let config = cpal::StreamConfig {
        channels: device_channels,
        sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    info!(
        "Output device config: channels={}, sample_rate={}",
        device_channels, sample_rate
    );

    Ok((config, device_channels, sample_rate))
}

/// Resolve the input stream config, preferring 48kHz if the device supports it.
/// Returns (StreamConfig, device_channels, actual_sample_rate).
fn resolve_input_stream_config(device: &cpal::Device) -> Result<(cpal::StreamConfig, u16, u32)> {
    let default_config = device.default_input_config()?;
    let device_channels = default_config.channels();

    let sample_rate = if can_device_support_input_rate(device, SAMPLE_RATE) {
        SAMPLE_RATE
    } else {
        default_config.sample_rate()
    };

    let config = cpal::StreamConfig {
        channels: device_channels,
        sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    info!(
        "Input device config: channels={}, sample_rate={}",
        device_channels, sample_rate
    );

    Ok((config, device_channels, sample_rate))
}

/// Create a sinc resampler for high-quality sample rate conversion.
/// Returns None when rates match (no overhead in the common case).
fn create_resampler(from_rate: u32, to_rate: u32, chunk_size: usize) -> Option<SincFixedIn<f32>> {
    if from_rate == to_rate {
        return None;
    }
    let params = SincInterpolationParameters {
        sinc_len: 128,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };
    Some(
        SincFixedIn::<f32>::new(
            to_rate as f64 / from_rate as f64,
            2.0,
            params,
            chunk_size,
            1, // mono
        )
        .expect("Failed to create resampler"),
    )
}

/// Expand mono samples to multiple channels by duplicating each sample.
fn expand_channels(mono_samples: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return mono_samples.to_vec();
    }

    let mut output = Vec::with_capacity(mono_samples.len() * channels as usize);
    for &sample in mono_samples {
        for _ in 0..channels {
            output.push(sample);
        }
    }
    output
}

/// Downmix interleaved multi-channel samples to mono by averaging.
fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }

    let ch = channels as usize;
    interleaved
        .chunks_exact(ch)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

impl AudioPipeline {
    /// Create a new AudioPipeline (not yet started)
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            capture_stream: None,
            capture_task: None,
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
            Ok((stream, task)) => {
                self.capture_stream = Some(SendStream(stream));
                self.capture_task = Some(task);
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
        if let Some(task) = self.capture_task.take() {
            task.abort();
        }
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

        let (config, device_channels, _device_rate) = resolve_input_stream_config(&device)?;

        let running_flag = running.clone();
        let vol = volume;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !running_flag.load(Ordering::Relaxed) {
                    return;
                }

                // Downmix to mono if device is multi-channel
                let mono_data: Vec<f32>;
                let samples: &[f32] = if device_channels > 1 {
                    mono_data = downmix_to_mono(data, device_channels);
                    &mono_data
                } else {
                    data
                };

                // Compute RMS
                let sum_sq: f32 = samples
                    .iter()
                    .map(|&s| {
                        let v = s * vol;
                        v * v
                    })
                    .sum();
                let rms = (sum_sq / samples.len().max(1) as f32).sqrt();
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

    /// Build the capture stream: cpal input -> NativeAudioSource for LiveKit.
    /// The cpal callback only forwards raw samples over a channel; all DSP
    /// (downmix, resample, convert, frame) runs in a separate tokio task.
    fn build_capture_stream(
        source: NativeAudioSource,
        running: Arc<AtomicBool>,
        audio_config: &AudioConfig,
        transmitting: Arc<AtomicBool>,
    ) -> Result<(cpal::Stream, tokio::task::JoinHandle<()>)> {
        let device = resolve_input_device(audio_config)?;

        debug!("Using input device: {:?}", device.description());

        let (config, device_channels, device_rate) = resolve_input_stream_config(&device)?;

        let running_flag = running.clone();
        let input_volume = audio_config.input_volume;
        let voice_activity = audio_config.voice_activity;
        let sensitivity = audio_config.sensitivity;

        // Channel for raw audio from cpal callback → processing task
        let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<Vec<f32>>();

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !running_flag.load(Ordering::Relaxed) {
                    return;
                }
                if !transmitting.load(Ordering::Relaxed) {
                    return;
                }
                let _ = raw_tx.send(data.to_vec());
            },
            move |err| {
                error!("Audio input stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        info!("Audio capture started");

        // Spawn processing task — all DSP runs here, off the real-time thread
        let capture_task = tokio::spawn(async move {
            let mut resampler = create_resampler(device_rate, SAMPLE_RATE, FRAME_SIZE);
            let mut mono_buffer: Vec<f32> = Vec::with_capacity(FRAME_SIZE * 2);
            let mut sample_buffer: Vec<i16> = Vec::with_capacity(FRAME_SIZE * 2);

            while let Some(raw_data) = raw_rx.recv().await {
                // 1. Downmix to mono
                let mono_samples = if device_channels > 1 {
                    downmix_to_mono(&raw_data, device_channels)
                } else {
                    raw_data
                };

                // 2. Resample to 48kHz via rubato (if needed)
                let resampled = if let Some(ref mut resampler) = resampler {
                    mono_buffer.extend_from_slice(&mono_samples);
                    let mut output = Vec::new();
                    while mono_buffer.len() >= FRAME_SIZE {
                        let chunk: Vec<f32> = mono_buffer.drain(..FRAME_SIZE).collect();
                        match resampler.process(&[chunk], None) {
                            Ok(mut result) => output.extend(result.pop().unwrap_or_default()),
                            Err(e) => error!("Capture resampler error: {}", e),
                        }
                    }
                    if output.is_empty() {
                        continue;
                    }
                    output
                } else {
                    mono_samples
                };

                // 3. Voice activity gate (on 48kHz data)
                if voice_activity {
                    let sum_sq: f32 = resampled
                        .iter()
                        .map(|&s| {
                            let v = s * input_volume;
                            v * v
                        })
                        .sum();
                    let rms = (sum_sq / resampled.len().max(1) as f32).sqrt();
                    if rms < sensitivity {
                        continue;
                    }
                }

                // 4. Convert f32 → i16 with volume
                let samples: Vec<i16> = resampled
                    .iter()
                    .map(|&s| ((s * input_volume).clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                    .collect();

                // 5. Accumulate and drain FRAME_SIZE chunks → NativeAudioSource
                sample_buffer.extend_from_slice(&samples);
                while sample_buffer.len() >= FRAME_SIZE {
                    let frame: Vec<i16> = sample_buffer.drain(..FRAME_SIZE).collect();
                    let audio_frame = AudioFrame {
                        data: frame.into(),
                        sample_rate: SAMPLE_RATE,
                        num_channels: 1,
                        samples_per_channel: FRAME_SIZE as u32,
                    };
                    let _ = source.capture_frame(&audio_frame).await;
                }
            }
        });

        Ok((stream, capture_task))
    }

    /// Build the playback stream for a remote LiveKit audio track.
    fn build_playback_stream(
        remote_track: RemoteAudioTrack,
        running: Arc<AtomicBool>,
        audio_config: &AudioConfig,
    ) -> Result<(cpal::Stream, tokio::task::JoinHandle<()>)> {
        let device = resolve_output_device(audio_config)?;

        debug!("Using output device: {:?}", device.description());

        let (config, device_channels, device_rate) = resolve_output_stream_config(&device)?;

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
            let mut resampler = create_resampler(SAMPLE_RATE, device_rate, FRAME_SIZE);

            loop {
                if !running_decode.load(Ordering::Relaxed) {
                    break;
                }

                match audio_stream.next().await {
                    Some(frame) => {
                        // Convert i16 to f32
                        let mut f32_samples: Vec<f32> = frame
                            .data
                            .iter()
                            .map(|&s| s as f32 / i16::MAX as f32)
                            .collect();

                        // Resample from 48kHz to device rate if different
                        if let Some(ref mut resampler) = resampler {
                            let waves_in = vec![f32_samples];
                            match resampler.process(&waves_in, None) {
                                Ok(mut result) => {
                                    f32_samples = result.pop().unwrap_or_default();
                                }
                                Err(e) => {
                                    error!("Playback resampler error: {}", e);
                                    continue;
                                }
                            }
                        }

                        // Expand mono to device channels if needed
                        if device_channels > 1 {
                            f32_samples = expand_channels(&f32_samples, device_channels);
                        }

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
