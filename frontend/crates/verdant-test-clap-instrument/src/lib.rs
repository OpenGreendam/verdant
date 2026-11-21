//! Test fixture: the smallest useful CLAP instrument. One stereo output, no
//! audio input, CLAP note events in, a sine per held key out. It exists so
//! the plug-in host's note path can be exercised end to end without any
//! third-party plug-in.

use clack_extensions::audio_ports::{AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, AudioPortType, PluginAudioPorts, PluginAudioPortsImpl};
use clack_plugin::events::event_types::{NoteOffEvent, NoteOnEvent};
use clack_plugin::prelude::*;

pub struct SineInstrument;

impl Plugin for SineInstrument {
    type AudioProcessor<'a> = Voices;
    type Shared<'a> = ();
    type MainThread<'a> = Ports;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: Option<&()>) {
        builder.register::<PluginAudioPorts>();
    }
}

impl DefaultPluginFactory for SineInstrument {
    fn get_descriptor() -> PluginDescriptor {
        use clack_plugin::plugin::features::{INSTRUMENT, SYNTHESIZER};
        PluginDescriptor::new("org.verdant.test.sine-instrument", "Verdant Test Sine").with_features([INSTRUMENT, SYNTHESIZER])
    }
    fn new_shared(_host: HostSharedHandle<'_>) -> Result<(), PluginError> { Ok(()) }
    fn new_main_thread<'a>(_host: HostMainThreadHandle<'a>, _shared: &'a ()) -> Result<Ports, PluginError> { Ok(Ports) }
}

pub struct Ports;

impl PluginMainThread<'_, ()> for Ports {}

impl PluginAudioPortsImpl for Ports {
    fn count(&self, is_input: bool) -> u32 { if is_input { 0 } else { 1 } }
    fn get(&self, index: u32, is_input: bool, writer: &mut AudioPortInfoWriter) {
        if index == 0 && !is_input {
            writer.set(&AudioPortInfo {
                id: ClapId::new(0), name: b"main", channel_count: 2, flags: AudioPortFlags::IS_MAIN,
                port_type: Some(AudioPortType::STEREO), in_place_pair: None,
            });
        }
    }
}

pub struct Voices { rate: f64, keys: [Option<(f64, f64)>; 128] }

impl<'a> PluginAudioProcessor<'a, (), Ports> for Voices {
    fn activate(_host: HostAudioProcessorHandle<'a>, _main: &Ports, _shared: &'a (), config: PluginAudioConfiguration) -> Result<Self, PluginError> {
        Ok(Self { rate: config.sample_rate, keys: [None; 128] })
    }

    fn process(&mut self, _process: Process, mut audio: Audio, events: Events) -> Result<ProcessStatus, PluginError> {
        let mut port = audio.output_port(0).ok_or(PluginError::Message("no output port"))?;
        let frames = port.frames_count() as usize;
        let mut channels = port.channels()?.into_f32().ok_or(PluginError::Message("expected f32 output"))?;
        let mut pending = events.input.iter().peekable();
        let mut mono = vec![0.0f32; frames];
        for (i, out) in mono.iter_mut().enumerate() {
            while let Some(event) = pending.next_if(|e| e.header().time() as usize <= i) {
                if let Some(on) = event.as_event::<NoteOnEvent>() {
                    if let Some(key) = on.pckn().key.into_specific() { self.keys[key as usize % 128] = Some((0.0, on.velocity())); }
                } else if let Some(off) = event.as_event::<NoteOffEvent>() {
                    if let Some(key) = off.pckn().key.into_specific() { self.keys[key as usize % 128] = None; }
                }
            }
            let mut sum = 0.0;
            for (key, voice) in self.keys.iter_mut().enumerate() {
                if let Some((phase, velocity)) = voice {
                    sum += phase.sin() * *velocity * 0.25;
                    *phase += std::f64::consts::TAU * 440.0 * 2f64.powf((key as f64 - 69.0) / 12.0) / self.rate;
                }
            }
            *out = sum as f32;
        }
        for channel in channels.iter_mut() { channel.copy_from_slice(&mono); }
        Ok(ProcessStatus::Continue)
    }
}

clack_export_entry!(SinglePluginEntry<SineInstrument>);
