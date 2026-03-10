use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures::StreamExt;
use gosuto_livekit::track::RemoteAudioTrack;
use gosuto_livekit::webrtc::audio_source::AudioSourceOptions;
use gosuto_livekit::webrtc::audio_source::native::NativeAudioSource;
use gosuto_livekit::webrtc::audio_stream::native::NativeAudioStream;
use gosuto_livekit::webrtc::prelude::AudioFrame;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::AudioConfig;
use crate::event::{AppEvent, EventSender, WarnClosed};

pub const SAMPLE_RATE: u32 = 48000;
pub const FRAME_SIZE: usize = 960; // 20ms at 48kHz

/// Wrapper to make cpal::Stream Send.
/// cpal::Stream is !Send because the audio backend uses raw pointers internally,
/// but the stream itself is safe to move between threads (we only stop/drop it).
struct SendStream {
    _stream: cpal::Stream,
}

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
/// Returns (StreamConfig, device_channels, actual_sample_rate, sample_format).
fn resolve_output_stream_config(
    device: &cpal::Device,
) -> Result<(cpal::StreamConfig, u16, u32, cpal::SampleFormat)> {
    let default_config = device.default_output_config()?;
    let device_channels = default_config.channels();
    let sample_format = default_config.sample_format();

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
        "Output device config: channels={}, sample_rate={}, format={:?}",
        device_channels, sample_rate, sample_format
    );

    Ok((config, device_channels, sample_rate, sample_format))
}

/// Resolve the input stream config, preferring 48kHz if the device supports it.
/// Returns (StreamConfig, device_channels, actual_sample_rate, sample_format).
fn resolve_input_stream_config(
    device: &cpal::Device,
) -> Result<(cpal::StreamConfig, u16, u32, cpal::SampleFormat)> {
    let default_config = device.default_input_config()?;
    let device_channels = default_config.channels();
    let sample_format = default_config.sample_format();

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
        "Input device config: channels={}, sample_rate={}, format={:?}",
        device_channels, sample_rate, sample_format
    );

    Ok((config, device_channels, sample_rate, sample_format))
}

/// Create a sinc resampler for high-quality sample rate conversion.
/// Returns None when rates match (no overhead in the common case).
fn create_resampler(from_rate: u32, to_rate: u32, chunk_size: usize) -> Option<Async<f32>> {
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
    match Async::<f32>::new_sinc(
        to_rate as f64 / from_rate as f64,
        2.0,
        &params,
        chunk_size,
        1, // mono
        FixedAsync::Input,
    ) {
        Ok(resampler) => Some(resampler),
        Err(e) => {
            error!("Failed to create resampler ({from_rate}→{to_rate}Hz): {e}");
            None
        }
    }
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
        ptt_transmitting: Arc<AtomicBool>,
        mic_active: Arc<AtomicBool>,
    ) -> Result<NativeAudioSource> {
        self.running.store(true, Ordering::Relaxed);

        let source = NativeAudioSource::new(
            AudioSourceOptions::default(),
            SAMPLE_RATE,
            1, // mono
            FRAME_SIZE as u32,
        );

        match Self::build_capture_stream(
            source.clone(),
            self.running.clone(),
            config,
            ptt_transmitting,
            mic_active,
        ) {
            Ok((stream, task)) => {
                self.capture_stream = Some(SendStream { _stream: stream });
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
        self.playback_streams.push(SendStream { _stream: stream });
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

        let (config, device_channels, _device_rate, sample_format) =
            resolve_input_stream_config(&device)?;

        let running_flag = running.clone();
        let vol = volume;

        macro_rules! build_mic_stream {
            ($T:ty, $convert:expr) => {
                device.build_input_stream(
                    &config,
                    {
                        let running = running_flag.clone();
                        let level_tx = level_tx.clone();
                        move |data: &[$T], _: &cpal::InputCallbackInfo| {
                            if !running.load(Ordering::Relaxed) {
                                return;
                            }
                            let f32_data: Vec<f32> = data.iter().map($convert).collect();
                            let mono_data: Vec<f32>;
                            let samples: &[f32] = if device_channels > 1 {
                                mono_data = downmix_to_mono(&f32_data, device_channels);
                                &mono_data
                            } else {
                                &f32_data
                            };
                            let sum_sq: f32 = samples
                                .iter()
                                .map(|&s| {
                                    let v = s * vol;
                                    v * v
                                })
                                .sum();
                            let rms = (sum_sq / samples.len().max(1) as f32).sqrt();
                            level_tx
                                .send(AppEvent::MicLevel(rms.sqrt().clamp(0.0, 1.0)))
                                .warn_closed("MicLevel");
                        }
                    },
                    |err: cpal::StreamError| error!("Mic test stream error: {err}"),
                    None,
                )?
            };
        }

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_mic_stream!(f32, |&s: &f32| s),
            cpal::SampleFormat::I16 => {
                build_mic_stream!(i16, |&s: &i16| s as f32 / i16::MAX as f32)
            }
            cpal::SampleFormat::I32 => {
                build_mic_stream!(i32, |&s: &i32| s as f32 / i32::MAX as f32)
            }
            format => anyhow::bail!("Unsupported input sample format: {format:?}"),
        };

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
        ptt_transmitting: Arc<AtomicBool>,
        mic_active: Arc<AtomicBool>,
    ) -> Result<(cpal::Stream, tokio::task::JoinHandle<()>)> {
        let device = resolve_input_device(audio_config)?;

        debug!("Using input device: {:?}", device.description());

        let (config, device_channels, device_rate, sample_format) =
            resolve_input_stream_config(&device)?;

        let running_flag = running.clone();
        let input_volume = audio_config.input_volume;
        let voice_activity = audio_config.voice_activity;
        let push_to_talk = audio_config.push_to_talk;
        let sensitivity = audio_config.sensitivity;
        let vad_hold_duration = Duration::from_millis(audio_config.vad_hold_ms);

        // Channel for raw audio from cpal callback → processing task
        let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<Vec<f32>>();

        macro_rules! build_capture {
            ($T:ty, $convert:expr) => {
                device.build_input_stream(
                    &config,
                    {
                        let running = running_flag.clone();
                        let raw_tx = raw_tx.clone();
                        move |data: &[$T], _: &cpal::InputCallbackInfo| {
                            if !running.load(Ordering::Relaxed) {
                                return;
                            }
                            let f32_data: Vec<f32> = data.iter().map($convert).collect();
                            raw_tx.send(f32_data).warn_closed("raw audio capture");
                        }
                    },
                    |err: cpal::StreamError| error!("Audio input stream error: {err}"),
                    None,
                )?
            };
        }

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_capture!(f32, |&s: &f32| s),
            cpal::SampleFormat::I16 => {
                build_capture!(i16, |&s: &i16| s as f32 / i16::MAX as f32)
            }
            cpal::SampleFormat::I32 => {
                build_capture!(i32, |&s: &i32| s as f32 / i32::MAX as f32)
            }
            format => anyhow::bail!("Unsupported input sample format: {format:?}"),
        };

        stream.play()?;
        info!("Audio capture started");

        // Spawn processing task — all DSP runs here, off the real-time thread
        let capture_task = tokio::spawn(async move {
            let mut resampler = create_resampler(device_rate, SAMPLE_RATE, FRAME_SIZE);
            let mut mono_buffer: Vec<f32> = Vec::with_capacity(FRAME_SIZE * 2);
            let mut sample_buffer: Vec<i16> = Vec::with_capacity(FRAME_SIZE * 2);
            let mut last_voice_at: Option<std::time::Instant> = None;

            while let Some(raw_data) = raw_rx.recv().await {
                // 1. PTT gate
                if push_to_talk && !ptt_transmitting.load(Ordering::Relaxed) {
                    mic_active.store(false, Ordering::Relaxed);
                    continue;
                }

                // 2. Downmix to mono
                let mono_samples = if device_channels > 1 {
                    downmix_to_mono(&raw_data, device_channels)
                } else {
                    raw_data
                };

                // 3. Resample to 48kHz via rubato (if needed)
                let resampled = if let Some(ref mut resampler) = resampler {
                    mono_buffer.extend_from_slice(&mono_samples);
                    let mut output = Vec::new();
                    while mono_buffer.len() >= FRAME_SIZE {
                        let chunk: Vec<f32> = mono_buffer.drain(..FRAME_SIZE).collect();
                        let channels = [chunk];
                        let input = audioadapter_buffers::direct::SequentialSliceOfVecs::new(
                            &channels[..],
                            1,
                            FRAME_SIZE,
                        );
                        match input {
                            Ok(adapter) => match resampler.process(&adapter, 0, None) {
                                Ok(result) => output.extend(result.take_data()),
                                Err(e) => error!("Capture resampler error: {}", e),
                            },
                            Err(e) => error!("Capture resampler adapter error: {}", e),
                        }
                    }
                    if output.is_empty() {
                        continue;
                    }
                    output
                } else {
                    mono_samples
                };

                // 4. Voice activity gate with hold/decay (on 48kHz data)
                if voice_activity {
                    let sum_sq: f32 = resampled
                        .iter()
                        .map(|&s| {
                            let v = s * input_volume;
                            v * v
                        })
                        .sum();
                    let rms = (sum_sq / resampled.len().max(1) as f32).sqrt();
                    if rms >= sensitivity {
                        last_voice_at = Some(std::time::Instant::now());
                    } else if let Some(last) = last_voice_at {
                        if last.elapsed() >= vad_hold_duration {
                            mic_active.store(false, Ordering::Relaxed);
                            continue;
                        }
                        // Within hold period — keep sending
                    } else {
                        // No voice detected yet
                        mic_active.store(false, Ordering::Relaxed);
                        continue;
                    }
                }

                mic_active.store(true, Ordering::Relaxed);

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
                    if let Err(e) = source.capture_frame(&audio_frame).await {
                        tracing::warn!("capture_frame failed: {e}");
                    }
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

        let (config, device_channels, device_rate, sample_format) =
            resolve_output_stream_config(&device)?;

        let output_volume = audio_config.output_volume;

        // Channel for decoded audio samples
        let (audio_tx, audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();
        let audio_rx = Arc::new(parking_lot::Mutex::new(audio_rx));

        let playback_buffer = Arc::new(parking_lot::Mutex::new(Vec::<f32>::new()));
        let playback_buf_clone = playback_buffer.clone();
        let running_flag = running.clone();

        macro_rules! build_output {
            ($T:ty, $zero:expr, $from_f32:expr) => {
                device.build_output_stream(
                    &config,
                    {
                        let running = running_flag.clone();
                        let audio_rx = audio_rx.clone();
                        let playback_buf = playback_buf_clone.clone();
                        move |data: &mut [$T], _: &cpal::OutputCallbackInfo| {
                            if !running.load(Ordering::Relaxed) {
                                data.fill($zero);
                                return;
                            }
                            let mut rx = audio_rx.lock();
                            while let Ok(samples) = rx.try_recv() {
                                playback_buf.lock().extend(samples);
                            }
                            drop(rx);
                            let mut buf = playback_buf.lock();
                            let available = buf.len().min(data.len());
                            if available > 0 {
                                for (i, sample) in buf.drain(..available).enumerate() {
                                    data[i] = $from_f32((sample * output_volume).clamp(-1.0, 1.0));
                                }
                            }
                            data[available..].fill($zero);
                        }
                    },
                    |err: cpal::StreamError| error!("Audio output stream error: {err}"),
                    None,
                )?
            };
        }

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_output!(f32, 0.0f32, |s: f32| s),
            cpal::SampleFormat::I16 => {
                build_output!(i16, 0i16, |s: f32| (s * i16::MAX as f32) as i16)
            }
            cpal::SampleFormat::I32 => {
                build_output!(i32, 0i32, |s: f32| (s * i32::MAX as f32) as i32)
            }
            format => anyhow::bail!("Unsupported output sample format: {format:?}"),
        };

        stream.play()?;
        info!("Audio playback started");

        // Spawn a task to read from NativeAudioStream and forward to cpal
        let running_decode = running.clone();
        let task = tokio::spawn(async move {
            let mut audio_stream =
                NativeAudioStream::new(remote_track.rtc_track(), SAMPLE_RATE as i32, 1);
            let mut resampler = create_resampler(SAMPLE_RATE, device_rate, FRAME_SIZE);
            let mut mono_buffer: Vec<f32> = Vec::with_capacity(FRAME_SIZE * 2);
            let mut frame_count: u64 = 0;

            loop {
                if !running_decode.load(Ordering::Relaxed) {
                    break;
                }

                match audio_stream.next().await {
                    Some(frame) => {
                        if frame_count == 0 {
                            let min = frame.data.iter().copied().min().unwrap_or(0);
                            let max = frame.data.iter().copied().max().unwrap_or(0);
                            let mean: f64 = frame.data.iter().map(|&s| s as f64).sum::<f64>()
                                / frame.data.len().max(1) as f64;
                            let rms: f64 = (frame
                                .data
                                .iter()
                                .map(|&s| (s as f64) * (s as f64))
                                .sum::<f64>()
                                / frame.data.len().max(1) as f64)
                                .sqrt();
                            info!(
                                "First remote audio frame: sample_rate={}, num_channels={}, \
                                 samples_per_channel={}, data_len={}, min={}, max={}, \
                                 mean={:.1}, rms={:.1}",
                                frame.sample_rate,
                                frame.num_channels,
                                frame.samples_per_channel,
                                frame.data.len(),
                                min,
                                max,
                                mean,
                                rms
                            );
                        }
                        frame_count += 1;
                        // Convert i16 to f32
                        let f32_samples: Vec<f32> = frame
                            .data
                            .iter()
                            .map(|&s| s as f32 / i16::MAX as f32)
                            .collect();

                        // Resample from 48kHz to device rate if different
                        if let Some(ref mut resampler) = resampler {
                            // Buffer samples and process in FRAME_SIZE chunks
                            // (SincFixedIn requires exactly chunk_size samples)
                            mono_buffer.extend_from_slice(&f32_samples);
                            while mono_buffer.len() >= FRAME_SIZE {
                                let chunk: Vec<f32> = mono_buffer.drain(..FRAME_SIZE).collect();
                                let channels = [chunk];
                                let input =
                                    audioadapter_buffers::direct::SequentialSliceOfVecs::new(
                                        &channels[..],
                                        1,
                                        FRAME_SIZE,
                                    );
                                match input {
                                    Ok(adapter) => match resampler.process(&adapter, 0, None) {
                                        Ok(result) => {
                                            let mut resampled = result.take_data();
                                            if device_channels > 1 {
                                                resampled =
                                                    expand_channels(&resampled, device_channels);
                                            }
                                            audio_tx.send(resampled).warn_closed("audio playback");
                                        }
                                        Err(e) => {
                                            error!("Playback resampler error: {}", e);
                                        }
                                    },
                                    Err(e) => error!("Playback resampler adapter error: {}", e),
                                }
                            }
                        } else {
                            // No resampling needed — expand channels and send directly
                            let output = if device_channels > 1 {
                                expand_channels(&f32_samples, device_channels)
                            } else {
                                f32_samples
                            };
                            audio_tx.send(output).warn_closed("audio playback");
                        }
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
