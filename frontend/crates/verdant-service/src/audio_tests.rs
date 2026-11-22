//! Hardware-free parity tests of the actual exporter and callback. The PCM
//! fixture is synthetic repeated harmonic material, not a voice-model result.
use super::*;
use verdant_audio::{CallbackRenderer, Timeline};
use verdant_backend::VerifiedAudio;
use verdant_backend::post::TimeMap;
use verdant_domain::{Note, Track};
use verdant_domain::model::GainPoint;

fn settings(project:&Project)->RenderSettings{RenderSettings{options:ExportOptions::default(),active_track:project.tracks[0].id.clone(),range_ticks:None,media_policy:verdant_media::AssetPolicy::new(Vec::<PathBuf>::new()).unwrap()}}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("verdant-export-parity-{}", NoteId::generate()));
        std::fs::create_dir_all(&directory).unwrap();
        Self(directory)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

fn fixture() -> (Project, Vec<Source>) {
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 140_000_000;
    project.tracks.push(Track::new("second repeated phrase", "blue"));
    project.tracks[0].gain_millidecibels = -2000;
    project.tracks[0].pan_permille = -250;
    project.tracks[1].gain_millidecibels = -6000;
    project.tracks[1].pan_permille = 300;
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).unwrap();
    let mut sources = Vec::new();
    for (track_index, track) in project.tracks.iter_mut().enumerate() {
        let start_tick = 320 + track_index as i64 * 480;
        let start_seconds = tempo.seconds_at(start_tick as f64);
        for i in 0..6 {
            let mut note = Note::new(start_tick + i * 960, 960, 60 + track_index as i32 * 4);
            note.lyric = ["a", "i", "u"][i as usize % 3].into();
            if i == 1 || i == 4 {
                note.curves.gain_millidecibel_points = vec![
                    GainPoint { id: format!("gain-{track_index}-{i}-a").as_str().into(), tick: 0, millidecibels: -18000, unknown: Default::default() },
                    GainPoint { id: format!("gain-{track_index}-{i}-b").as_str().into(), tick: 900, millidecibels: -3000, unknown: Default::default() },
                ];
            }
            track.notes.push(note);
        }
        let authored: Vec<_> = (0..=6).map(|i| tempo.seconds_at((start_tick + i * 960) as f64) - start_seconds).collect();
        let rendered: Vec<_> = authored.iter().map(|seconds| (seconds * 200.0).round() / 200.0).collect();
        let end = *rendered.last().unwrap();
        let sample_rate = [16_000, 44_100][track_index];
        // Include a source-tail sample beyond the verified map. DYN must
        // silence it identically on both paths, including after resampling.
        let len = (end * sample_rate as f64).round() as usize + 1;
        let samples: Arc<[f32]> = (0..len).map(|i| {
            let seconds = i as f64 / sample_rate as f64;
            let repeated = ((seconds / end * 6.0).floor() as usize) % 3;
            let phase = seconds * std::f64::consts::TAU * (220.0 + track_index as f64 * 55.0);
            let harmonic = [2.0, 5.0, 3.0][repeated];
            (0.22 * phase.sin() + 0.08 * (phase * harmonic).sin()) as f32
        }).collect::<Vec<_>>().into();
        sources.push(Source {
            track: track.id.clone(), start_seconds,
            note_ids: track.notes.iter().map(|n| n.id.clone()).collect(),
            phoneme_timing: None,
            playback_key: None,
            audio: VerifiedAudio {
                path: PathBuf::from("synthetic-preverified-fixture.wav"), sha256: "synthetic fixture; not read from disk".into(),
                sample_rate, samples, f0_tracks: Vec::new(), time_map: Some(TimeMap::new(rendered, authored).unwrap()),
            },
        });
    }
    (project, sources)
}

fn duplicate_sources(sources: &[Source]) -> Vec<Source> {
    sources.to_vec()
}

fn notes_for<'a>(project: &'a Project, source: &Source) -> Vec<&'a Note> {
    let track = project.track(&source.track).unwrap();
    source.note_ids.iter().map(|id| track.note(id).unwrap()).collect()
}

fn stereo_pcm(path: &Path) -> Vec<f32> {
    let wav = std::fs::read(path).unwrap();
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    let mut at = 12;
    while at + 8 <= wav.len() {
        let size = u32::from_le_bytes(wav[at + 4..at + 8].try_into().unwrap()) as usize;
        let data = &wav[at + 8..at + 8 + size];
        if &wav[at..at + 4] == b"fmt " {
            assert_eq!(u16::from_le_bytes(data[..2].try_into().unwrap()), 3);
            assert_eq!(u16::from_le_bytes(data[2..4].try_into().unwrap()), 2);
            assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()), RATE);
        }
        if &wav[at..at + 4] == b"data" {
            return data.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
        }
        at += 8 + size + (size & 1);
    }
    panic!("export contains no PCM chunk");
}

fn prepared_clips(project: &Project, sources: &[Source]) -> Vec<verdant_audio::Clip> {
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).unwrap();
    sources.iter().map(|source| {
        let track_index = project.tracks.iter().position(|t| t.id == source.track).unwrap();
        let notes = notes_for(project, source);
        let samples = project_audio::prepare_phrase_pcm(&source.audio, &notes, &tempo, source.start_seconds,
            source.phoneme_timing.as_ref(), RATE).unwrap();
        verdant_audio::Clip { start: (source.start_seconds * RATE as f64).round() as u64,
            samples, gain: 0.9, track: track_index as u16 }
    }).collect()
}

fn callback_pcm(project: &Project, clips: &[verdant_audio::Clip], frames: usize, block: usize) -> Vec<f32> {
    let (mut control, rt) = verdant_audio::engine::channel(RATE, 1024);
    let any_solo = project.tracks.iter().any(|track| track.solo);
    for (i, track) in project.tracks.iter().enumerate() {
        control.set_channel(i, project_audio::channel_settings(&track.header(), any_solo));
    }
    control.set_timeline(Timeline::new(RATE, clips.to_vec()));
    control.play();
    let mut callback = CallbackRenderer::new(rt, RATE);
    let mut actual = vec![0.0; frames * 2];
    for chunk in actual.chunks_mut(block * 2) { callback.process_f32(chunk, 2).unwrap(); }
    actual
}

#[test]
fn progressive_playback_preserves_authored_gain_and_replaces_new_render() {
    let (project, mut sources) = fixture();
    for source in &mut sources { source.playback_key = Some((0, "verified render".into())); }
    let policy = verdant_media::AssetPolicy::new(Vec::<PathBuf>::new()).unwrap();
    let first = project_audio::prepare_timeline(&project, &sources[..1], policy.clone(), RATE, Default::default(), None).unwrap();
    let (clip, first_pcm) = (first.timeline.clips[0].start, &first.timeline.clips[0].samples);
    let expanded = project_audio::prepare_timeline(&project, &sources, policy.clone(), RATE,
        first.phrase_pcm.for_sources(&sources, RATE), None).unwrap();
    let expanded_clip = expanded.timeline.clips.iter().find(|entry| entry.start == clip).unwrap();
    assert_eq!(&*expanded_clip.samples, &**first_pcm, "progressive playback must not apply DYN twice");

    let edited = &mut sources[0];
    edited.audio.samples = vec![0.0; edited.audio.samples.len()].into();
    let replaced = project_audio::prepare_timeline(&project, &sources, policy, RATE,
        expanded.phrase_pcm.for_sources(&sources, RATE), None).unwrap();
    let replaced_clip = replaced.timeline.clips.iter().find(|entry| entry.start == clip).unwrap();
    assert!(replaced_clip.samples.iter().all(|&sample| sample == 0.0));
    assert!(first_pcm.iter().any(|&sample| sample.abs() > 0.01));
}

#[test]
fn actual_export_matches_realtime_callback_with_mapped_dyn_resampling_and_pan() {
    let scratch = Scratch::new();
    let (project, sources) = fixture();
    let path = scratch.0.join("actual-export.wav");
    let frames = render_outputs(&path, &project, duplicate_sources(&sources), None, &settings(&project), true, &|| false).unwrap().frames as usize;
    let expected = stereo_pcm(&path);
    assert_eq!(expected.len(), frames * 2);
    let clips = prepared_clips(&project, &sources);
    for block in [1, 63, 257, 1024] {
        let actual = callback_pcm(&project, &clips, frames, block);
        for (i, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
            assert_eq!(actual.to_bits(), expected.to_bits(), "block={block}, interleaved sample={i}");
        }
    }
}

#[test]
fn empty_trailing_solo_track_matches_export_without_changing_stored_mutes() {
    let scratch = Scratch::new();
    let (mut project, sources) = fixture();
    let mut empty = Track::new("empty solo track", "blue");
    empty.solo = true;
    project.tracks.push(empty);
    let path = scratch.0.join("empty-solo.wav");
    let frames = render_outputs(&path, &project, duplicate_sources(&sources), None, &settings(&project), true, &|| false).unwrap().frames as usize;
    let expected = stereo_pcm(&path);
    assert!(expected.iter().all(|sample| *sample == 0.0));
    let clips = prepared_clips(&project, &sources);
    assert_eq!(callback_pcm(&project, &clips, frames, 257), expected);
    assert!(project.tracks.iter().all(|track| !track.muted), "effective solo muting must not change project headers");

    project.tracks.last_mut().unwrap().solo = false;
    let path = scratch.0.join("solo-disabled.wav");
    render_outputs(&path, &project, duplicate_sources(&sources), None, &settings(&project), true, &|| false).unwrap();
    let audible = stereo_pcm(&path);
    assert!(audible.iter().any(|sample| sample.abs() > 0.01));
    assert_eq!(callback_pcm(&project, &clips, frames, 257), audible, "disabling solo restores the other tracks");
}

#[test]
fn anchored_prefix_dyn_matches_export_and_playback_across_phrase_splits() {
    use verdant_backend::wire::{PhonemeTimingNote, PhonemeTimingTrace};
    let scratch = Scratch::new();
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 120_000_000;
    let mut a = Note::new(384, 1920, 60); // 0.2 .. 1.2 s
    let mut b = Note::new(2304, 1920, 64); // 1.2 .. 2.2 s; prefix starts at 1.1 s
    a.curves.gain_millidecibel_points = vec![GainPoint {
        id: "mute-previous".into(), tick: 0, millidecibels: -60000, unknown: Default::default(),
    }];
    b.curves.gain_millidecibel_points = vec![GainPoint {
        id: "current-gain".into(), tick: 0, millidecibels: -6000, unknown: Default::default(),
    }];
    project.tracks[0].notes = vec![a.clone(), b.clone()];
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).unwrap();
    let anchor = |note: &Note, origin: f64, first: usize, prefix: f64, trim: f64| PhonemeTimingNote {
        note_id: note.id.as_str().into(),
        note_onset_seconds: tempo.seconds_at(note.start_tick as f64) - origin,
        note_end_seconds: tempo.seconds_at(note.end_tick() as f64) - origin,
        first_segment_index: first, nucleus_segment_index: first + usize::from(prefix > 0.0),
        end_segment_index: first + 1 + usize::from(prefix > 0.0), prefix_seconds: prefix, tail_trim_seconds: trim,
    };
    let source = |start_seconds: f64, seconds: f64, anchors: Vec<PhonemeTimingNote>| Source {
        track: project.tracks[0].id.clone(), start_seconds,
        note_ids: anchors.iter().map(|a| a.note_id.as_str().into()).collect(),
        phoneme_timing: Some(PhonemeTimingTrace { method: "vowel-anchor-v1".into(), notes: anchors }),
        playback_key: None,
        audio: VerifiedAudio {
            path: PathBuf::from("synthetic-anchored-dyn.wav"), sha256: "synthetic fixture; not read from disk".into(),
            sample_rate: RATE, samples: vec![0.2f32; (seconds * RATE as f64).round() as usize].into(), f0_tracks: Vec::new(),
            time_map: Some(TimeMap::new(vec![0., seconds], vec![0., seconds]).unwrap()),
        },
    };
    let together = vec![source(0.2, 2., vec![anchor(&a, 0.2, 0, 0., 0.1), anchor(&b, 0.2, 1, 0.1, 0.)])];
    let separate = vec![
        source(0.2, 0.9, vec![anchor(&a, 0.2, 0, 0., 0.1)]),
        source(1.1, 1.1, vec![anchor(&b, 1.1, 0, 0.1, 0.)]),
    ];
    let mut variants = Vec::new();
    for (name, sources) in [("joined", together), ("split", separate)] {
        let path = scratch.0.join(format!("{name}.wav"));
        let frames = render_outputs(&path, &project, duplicate_sources(&sources), None, &settings(&project), true, &|| false).unwrap().frames as usize;
        let exported = stereo_pcm(&path);
        let clips = prepared_clips(&project, &sources);
        assert_eq!(callback_pcm(&project, &clips, frames, 257), exported);
        let before = ((1.1 * RATE as f64).round() as usize - 1) * 2;
        let prefix = (1.1 * RATE as f64).round() as usize * 2;
        assert!((exported[before] - 0.2 * 0.9 * 0.001).abs() < 1e-8);
        assert!((exported[prefix] - 0.2 * 0.9 * 10f32.powf(-6. / 20.)).abs() < 1e-7,
            "the quiet previous note must not mute the following consonant prefix");
        variants.push(exported);
    }
    assert_eq!(variants[0], variants[1], "worker phrase splitting cannot change scheduled DYN ownership");
}

fn callback_timeline(project:&Project,timeline:Timeline,rate:u32,channels:usize,frames:usize,block:usize)->Vec<f32>{
    let engine_rate=timeline.sample_rate;let(mut control,rt)=verdant_audio::engine::channel(engine_rate,1024);
    let any_solo=project.tracks.iter().any(|track|track.solo);
    for(index,track)in project.tracks.iter().enumerate(){control.set_channel(index,project_audio::channel_settings(&track.header(),any_solo));}
    control.set_timeline(timeline);control.play();let mut renderer=CallbackRenderer::new(rt,engine_rate);renderer.set_device_rate(rate).unwrap();
    let mut pcm=vec![0.0;frames*channels];for chunk in pcm.chunks_mut(block*channels){renderer.process_f32(chunk,channels).unwrap();}pcm
}

#[test]
fn imported_stereo_automation_and_range_export_use_selected_processing_clock(){
    use verdant_domain::automation::{AutomationDocument,AutomationLane,AutomationPoint,AutomationTarget,Interpolation,AUTOMATION_KEY};
    let scratch=Scratch::new();let input=scratch.0.join("owned-stereo-source.wav");
    let source_options=ExportOptions{sample_rate:24_000,..Default::default()};let source_frames=14_400;
    let samples=(0..source_frames).flat_map(|i|{let t=i as f32/24_000.;[(t*220.*std::f32::consts::TAU).sin()*0.3,(t*330.*std::f32::consts::TAU).cos()*0.15]}).collect::<Vec<f32>>();
    let mut source_wav=AtomicWav::new(&input,source_frames,&source_options).unwrap();let mut bytes=vec![0;samples.len()*4];let len=encode(&samples,WavEncoding::Float32,&mut bytes);source_wav.write(&bytes[..len]).unwrap();source_wav.finish().unwrap();source_wav.publish(true).unwrap();
    let policy=verdant_media::AssetPolicy::for_file(&input).unwrap();let imported=verdant_media::MediaImporter::new(policy.clone(),Default::default()).import_file(&input).unwrap();
    let mut project=Project::new_empty();project.tempo_map[0].micro_bpm=120_000_000;project.tempo_map.push(verdant_domain::model::TempoPoint{tick:960,micro_bpm:60_000_000,unknown:Default::default()});project.tracks.push(Track::new("stereo reference","blue"));
    for(index,track)in project.tracks.iter_mut().enumerate(){let mut clip=verdant_media::AudioClip::new(imported.source.clone(),track.id.clone(),if index==0{240}else{960});clip.source_offset_frames=100;clip.duration_frames=10_000;clip.gain=if index==0{0.75}else{0.5};clip.pan=if index==0{-0.2}else{0.7};verdant_media::write_clips(track,&[clip]).unwrap();}
    let mut automation=AutomationDocument::default();let mut gain=AutomationLane::new(AutomationTarget::TrackGain{track_id:project.tracks[0].id.clone()});gain.set_point(AutomationPoint::new(0,-2000.)).unwrap();gain.set_point(AutomationPoint::new(1440,-12000.)).unwrap();automation.set_lane(gain);
    let mut pan=AutomationLane::new(AutomationTarget::MasterPan);let mut held=AutomationPoint::new(480,-500.);held.interpolation=Interpolation::Step;pan.set_point(held).unwrap();pan.set_point(AutomationPoint::new(1200,500.)).unwrap();automation.set_lane(pan);project.extensions.insert(AUTOMATION_KEY.into(),automation.extension_value().unwrap().unwrap());
    let prepared=project_audio::prepare_timeline(&project,&[],policy.clone(),RATE,Default::default(),None).unwrap();assert_eq!(prepared.phrase_count,0);assert_eq!(prepared.media_count,2);
    let mut render_settings=settings(&project);render_settings.media_policy=policy;let path=scratch.0.join("imported-mix.wav");let frames=render_outputs(&path,&project,Vec::new(),None,&render_settings,true,&||false).unwrap().frames as usize;let exported=stereo_pcm(&path);
    assert!(exported.iter().any(|sample|sample.abs()>0.01));assert!(exported.chunks_exact(2).any(|frame|(frame[0]-frame[1]).abs()>0.01));
    for block in [63,257]{assert_eq!(exported,callback_timeline(&project,prepared.timeline.clone(),RATE,2,frames,block),"stereo media and automation must be audible on both consumers");}
    render_settings.options.sample_rate=44_100;render_settings.options.channels=1;render_settings.options.encoding=WavEncoding::Pcm24;render_settings.options.range=ExportRange::Selection;render_settings.range_ticks=Some((480,1200));
    let path=scratch.0.join("selected-mono24.wav");let frames=render_outputs(&path,&project,Vec::new(),None,&render_settings,true,&||false).unwrap().frames as usize;let decoded=verdant_media::decode_bytes(&std::fs::read(path).unwrap(),Some("wav"),Default::default()).unwrap();
    assert_eq!((decoded.sample_rate,decoded.channels),(44_100,1));assert_eq!(decoded.frames()as usize,frames);
    let tempo=TempoMap::new(&project.tempo_map,project.ppq).unwrap();assert_eq!(frames,((tempo.seconds_at(1200.)-tempo.seconds_at(480.))*44_100.).round()as usize);
    // The requested export clock now drives preparation and automation directly.
    let at_rate=project_audio::prepare_timeline(&project,&[],render_settings.media_policy.clone(),44_100,Default::default(),None).unwrap();
    let skip=(tempo.seconds_at(480.)*44_100.).round()as usize;let callback=callback_timeline(&project,at_rate.timeline,44_100,1,skip+frames,BLOCK);
    for(actual,expected)in decoded.samples.iter().zip(&callback[skip..]){let quantized=((*expected as f64*8_388_608.).round().clamp(-8_388_608.,8_388_607.)/8_388_608.)as f32;assert!((*actual-quantized).abs()<=1e-7,"range history, mono conversion and PCM24 encoding changed the audible result");}
}

#[test]
fn no_clobber_preserves_existing_output_and_removes_temporary_wav() {
    let scratch = Scratch::new();
    let (project, sources) = fixture();
    let path = scratch.0.join("existing.wav");
    std::fs::write(&path, b"already owned by caller").unwrap();
    let result = render_outputs(&path, &project, sources, None, &settings(&project), false, &|| false);
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"already owned by caller");
    assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(), 1);
}

#[test]
fn cancellation_during_render_never_publishes_output() {
    let scratch = Scratch::new();
    let (project, sources) = fixture();
    let path = scratch.0.join("cancelled.wav");
    let calls = std::cell::Cell::new(0);
    let cancelled = || {
        let count = calls.get() + 1;
        calls.set(count);
        count >= 5
    };
    let error = render_outputs(&path, &project, sources, None, &settings(&project), false, &cancelled).unwrap_err();
    assert!(error.contains("取消"));
    assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(), 0);
}

#[test]
fn stems_report_the_same_generated_paths_that_are_published() {
    let scratch = Scratch::new();
    let (mut project, sources) = fixture();
    project.tracks[0].name = "lead / first".into();
    project.tracks[1].name = "second voice".into();
    let mut render_settings = settings(&project);
    render_settings.options.scope = AudioScope::Stems;
    let output = render_outputs(&scratch.0.join("song.wav"), &project, sources, None, &render_settings, false, &|| false).unwrap();
    assert_eq!(output.paths, [scratch.0.join("song-01-lead___first.wav"), scratch.0.join("song-02-second_voice.wav")]);
    assert!(output.frames > 0);
    for path in output.paths {
        let audio = verdant_media::decode_bytes(&std::fs::read(path).unwrap(), Some("wav"), Default::default()).unwrap();
        assert!(audio.frames() > 0);
        assert_eq!(audio.sample_rate, RATE);
    }
}

#[test]
fn bypassed_native_instrument_exports_silence_and_reenables_without_state_loss(){
    let dir=Scratch::new();
    for id in [verdant_plugins::builtin_dsp::SYNTH_ID,verdant_plugins::builtin_dsp::ONE_SYNTH_ID]{
        let mut project=Project::new_empty();let track=&mut project.tracks[0];
        track.notes.push(Note::new(0,480,60));track.refresh_index();
        track.unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(),serde_json::json!({
            "format":"builtin","id":id,"params":{"gain":-12.,"release":25.},"bypass":false
        }));
        let options=settings(&project);let on=dir.0.join(format!("{}-on.wav",id));
        let active=render_outputs(&on,&project,vec![],None,&options,false,&||false).unwrap();
        let sounding=stereo_pcm(&on);assert!(sounding.iter().any(|v|v.abs()>1e-4));
        project.tracks[0].unknown.get_mut(verdant_domain::model::INSTRUMENT_KEY).unwrap()["bypass"]=true.into();
        let off=dir.0.join(format!("{}-off.wav",id));
        let silent=render_outputs(&off,&project,vec![],None,&options,false,&||false).unwrap();
        assert_eq!(active.frames,silent.frames);assert!(stereo_pcm(&off).iter().all(|v|*v==0.));
        project.tracks[0].unknown.get_mut(verdant_domain::model::INSTRUMENT_KEY).unwrap()["bypass"]=false.into();
        let restored=dir.0.join(format!("{}-restored.wav",id));
        render_outputs(&restored,&project,vec![],None,&options,false,&||false).unwrap();
        assert_eq!(stereo_pcm(&restored),sounding);
    }
}
