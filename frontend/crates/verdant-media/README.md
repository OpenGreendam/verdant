# verdant-media

Authorized local audio decoding, clip preparation and waveform previews for
the editor. The library returns PCM and persisted clip metadata.

## Import and reload

Audio import is synchronous work for the editor's preparation/background path.
`MediaImporter::new(policy, ImportLimits::default())` accepts an `AssetPolicy`
constructed from explicitly authorized directory roots, or `AssetPolicy::for_file`
for one native file selection. `allow_file` can authorize a later explicit
selection/relink. Reading a project must not authorize its arbitrary saved
asset paths. Relative document paths are rejected; imported paths are stored
as canonical absolute paths. Moving a project therefore needs explicit asset
relink/authorization by the consumer UI.

`import_file` returns `ImportedAudio { source, audio }`. It opens a regular
file, binds opened-file identity, bounds the byte snapshot and hashes the same
bytes passed to the decoder. Unix opens walk every component through owned
directory descriptors with `O_NOFOLLOW`; final opens are nonblocking so a FIFO
swap can be rejected. Source files are never written or copied into the source
repository. `load_source` repeats authorization and decode and compares SHA-256,
encoded length, sample rate, channel count and decoded frame count. Missing,
changed, malformed and unsupported assets return errors.

Symphonia 0.5.5 is enabled with all its codec/format features. This includes
PCM16/24/32 and floating WAV, AIFF/CAF, FLAC, MP1/2/3, AAC/ALAC and Vorbis in
containers supported by the library. An extension is only a probe hint. The
chosen audio stream must have a stable sample rate and mono or left/right
stereo layout. There is no implicit surround downmix. Opus, DRM and codecs
outside the admitted library are not represented as supported input.

`DecodedAudio` holds interleaved immutable `Arc<[f32]>`, its sample rate/channel
count and the exact encoded source fingerprint. IEEE float source amplitudes
are retained; non-finite samples are rejected. WAV RIFF/data framing is checked
before decode so truncated source samples cannot turn into a shorter accepted
clip. Decoder errors and checksum failures are returned.

## Clip format and preparation

`AudioClip::new(source, track_id, start_tick)` creates a stable UUID and defaults
to the full source at unity gain, centered and unmuted. Its persisted contract is:

- ID and owning domain track ID;
- source canonical path, encoded SHA-256/byte length, sample rate/channels/frames;
- musical start tick;
- source trim offset and duration in source-rate frames;
- finite linear gain 0..64, pan -1..1, and mute.

`Track.unknown["audio_clips"]` contains `{ "schema": "verdant-audio-clips/1",
"clips": [...] }`. `read_clips`, `write_clips` and `clips_value` enforce schema,
source/trim bounds, track membership and unique clip IDs. An empty collection
removes the extension. Live editor edits should submit `clips_value` through
`Command::SetTrackData`, preserving the domain's atomic undo/journal behavior.
Other track extensions are retained.

`prepare_clip(clip, decoded, tempo_map, engine_rate, engine_track)` maps the
musical start using absolute TempoMap time, trims source frames, reuses the
existing verdant-audio windowed-sinc resampler per channel, and prepares clip
pan. Centered rate-matched full clips share the original PCM allocation.
Mono pan uses constant-power coefficients and becomes stereo when necessary;
stereo balance leaves the nearer channel at unity. Pan is baked once, so
`PreparedClip.pan` is zero. `.gain` contains the authored gain (zero when muted).
Stereo channel identity is preserved.

`MediaImporter::prepare_project_clips` authorizes all saved paths, decodes each
source once, checks project-wide retained PCM bounds, and returns prepared clips
with native mixer channel indices. The editor maps channel count 1 to its mono
`Clip`, and count 2 to `StereoClip`/`Timeline::with_stereo`. Playback, offline mix,
stems and range export must reuse these exact prepared samples and gain rather
than invoke separate decoders or resamplers. Track inserts/faders/pan remain
owned by the audio engine.

## Waveform previews

`WaveformCache` is a bounded LRU of per-channel min/max/RMS envelopes, keyed by
source fingerprint, frame range and bin count. `preview(&decoded, range, bins)`
returns an Arc envelope; the cache retains at most its configured bytes and
128 entries. Callers own the lifetime of Arcs they retain after eviction. PCM
and full source bytes are not stored in this preview cache.

## Checks

From `frontend/`:

```sh
cargo test --offline --locked -p verdant-media
```

Regression fixtures are tiny generated format/performance inputs, not music or
proprietary assets. They cover signed PCM depths and stereo ordering, float WAV,
AIFF, malformed sources, fingerprint reload, source authorization/symlink swaps,
trim/tempo placement, source sharing, DC resampling edge gain, clip pan, and
bounded waveform eviction. Consumer checks cover file-picker import/relink,
waveform display, stereo playback and equality of prepared samples across
playback, offline export, stems and range export.
