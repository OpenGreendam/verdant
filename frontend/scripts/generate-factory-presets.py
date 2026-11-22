#!/usr/bin/env python3
"""Rebuild Verdant's original, versioned factory recipes. No vendor assets."""
import json, re, pathlib, argparse
P=[]
def add(device, name, category, description, values, tags=()):
    slug=re.sub(r"[^a-z0-9]+","-",name.lower()).strip("-")
    P.append(dict(id=f"verdant.factory.{device}.{slug}.v1",device=f"verdant.builtin.{device}",name=name,category=category,
        author="Verdant",description=description,tags=list(dict.fromkeys([category.lower(),device,*tags])),parameters=values))
def rows(device, fields, text, advice):
    for line in text.strip().splitlines():
        name,cat,numbers,hint=line.strip().split("|")
        vals=[float(n) for n in numbers.split()]
        assert len(vals)==len(fields.split()),(name,vals,fields)
        add(device,name,cat,hint+" "+advice,dict(zip(fields.split(),vals)))
# Equalizer: every row specifies an independent spectral decision; dormant bands
# are reset, so selection never inherits a previous band's dynamic settings.
eqs=[
("Intimate Lead","Vocal",[(2,75,0,.707),(0,260,-2.5,1.1),(0,2900,1.5,.9),(3,11500,2,.707)],"Reduce proximity buildup while preserving breath."),
("Bright Pop Lead","Vocal",[(2,95,0,.707),(0,420,-2,1.2),(0,3400,2,.8),(3,12500,3,.707)],"Open a dense lead vocal without a narrow presence spike."),
("Dark Voice Clarity","Vocal",[(2,65,0,.707),(0,210,-2,1),(0,2300,3,.8)],"Bring consonants forward on a dark recording."),
("Soft Air","Vocal",[(3,14000,2.5,.707),(0,7200,-1.5,1.5)],"Add top air while keeping the sibilance region restrained."),
("Backing Vocal Space","Vocal",[(2,160,0,.707),(0,2500,-3,1),(4,14500,0,.707)],"Leave room for the lead in a stacked vocal arrangement."),
("Dialogue Presence","Dialogue",[(2,80,0,.707),(0,350,-3,1.2),(0,2100,2,.8)],"Improve speech intelligibility before level control."),
("Podcast Proximity","Dialogue",[(1,140,-3,.707),(0,500,-2,1.1),(3,8000,1,.707)],"Tame close-microphone weight on spoken voice."),
("Telephone Band","Creative",[(2,350,0,.707),(4,3200,0,.707),(0,1500,3,1.1)],"Narrow-band dialogue effect; keep level matched."),
("Kick Weight","Drums",[(0,65,3,.8),(0,320,-4,1.2),(0,3500,2,1)],"Separate kick body, boxiness and beater attack."),
("Kick Tight","Drums",[(2,35,0,.707),(0,90,2,1),(0,210,-3,1.4)],"Trim subsonic energy and shorten perceived low-end bloom."),
("Snare Snap","Drums",[(2,110,0,.707),(0,190,2,1),(0,750,-3,1.4),(3,7000,2,.707)],"Keep snare body with a clean upper attack."),
("Overhead Silk","Drums",[(2,240,0,.707),(0,4400,-2.5,1.1),(3,12500,1,.707)],"Reduce cymbal edge while keeping the room open."),
("Bass Definition","Bass",[(2,32,0,.707),(0,95,2,.8),(0,280,-3,1),(0,1200,2,.9)],"Clarify bass note definition under a kick."),
("Bass Small Speaker","Bass",[(0,750,3,.9),(0,2300,1.5,1.2),(4,9000,0,.707)],"Emphasize audible harmonics on small playback systems."),
("Acoustic Detail","Instruments",[(2,85,0,.707),(0,220,-2.5,1.1),(3,9500,2,.707)],"Reduce acoustic-guitar boom while retaining pick detail."),
("Piano Room","Instruments",[(2,45,0,.707),(0,300,-2,.8),(0,3200,1.5,.8)],"Balance a piano across low mids and hammer detail."),
("Guitar Center Space","Instruments",[(2,110,0,.707),(0,1800,-2,1),(4,10000,0,.707)],"Make space for a lead vocal in a guitar arrangement."),
("Mix Gentle Tilt","Bus",[(7,1100,1,.707),(2,25,0,.707)],"A restrained tonal tilt for a balanced mix."),
("Master Low Cleanup","Mastering",[(2,25,0,.707),(0,240,-.8,.7),(3,14000,.7,.707)],"Small broad corrections; compare at matched loudness."),
("Dynamic Harshness","Vocal",[(0,3200,0,1.4)],"Reduce upper-mid buildup only while its detector is active."),
]
for i,(name,cat,bands,hint) in enumerate(eqs):
    v={f"b{b}_on":0 for b in range(1,25)}
    for n,(shape,freq,gain,q) in enumerate(bands,1):
        v.update({f"b{n}_on":1,f"b{n}_shape":shape,f"b{n}_freq":freq,f"b{n}_gain":gain,f"b{n}_q":q})
    if name=="Dynamic Harshness":v.update(b1_dynamic=1,b1_dyn_range=-4,b1_dyn_threshold=-30,b1_dyn_attack=6,b1_dyn_release=110)
    add("eq",name,cat,hint+" Adjust frequencies to the source; bypass at matched output.",v,("tonal balance",))
rows("compressor","threshold ratio attack release knee makeup mix style range sc_hpf",r"""
Vocal Leveler|Vocal|-22 2.5 18 160 9 2 100 2 8 100|Smooth phrase-to-phrase level on a close vocal.
Pop Vocal Forward|Vocal|-24 4 8 85 6 3 100 3 10 120|Bring a lead forward while retaining articulation.
Gentle Voice|Vocal|-18 2 25 200 12 1 100 0 5 90|A restrained first stage for an expressive singer.
Rap Focus|Vocal|-20 5 5 65 4 3 100 5 10 130|Control sharp peaks on a dense rhythmic performance.
Backing Stack|Vocal|-25 3 12 140 9 2 100 1 9 160|Keep stacked backing parts behind the lead.
Breathy Parallel|Vocal|-30 6 3 180 12 0 35 2 14 180|Blend a compressed body under the untouched voice.
Dialogue Level|Dialogue|-24 3 10 180 10 2 100 2 9 100|Even speech with a broad transition around threshold.
Podcast Fast Peaks|Dialogue|-15 5 1 90 3 0 100 0 6 80|Catch occasional close-microphone peaks.
Kick Punch|Drums|-18 4 28 85 3 2 100 4 10 70|Let the initial kick hit through before compression.
Snare Sustain|Drums|-24 5 18 150 6 3 80 4 12 100|Lift the body beneath the snare transient.
Drum Room Crush|Drums|-32 10 2 160 6 0 45 5 20 150|Parallel room excitement; raise blend gradually.
Drum Bus Glue|Bus|-16 2 30 120 6 1 100 1 5 100|Gentle shared movement across a drum group.
Bass Even|Bass|-22 4 20 130 9 2 100 2 10 35|Hold a bass line steady without losing attack.
Bass Finger Attack|Bass|-18 3 35 90 6 1.5 100 0 7 45|Preserve pluck while controlling sustained notes.
Acoustic Strum|Instruments|-20 3 15 160 9 2 100 1 7 120|Keep strums even under a vocal.
Piano Shape|Instruments|-18 2 35 230 12 1 100 0 6 80|Preserve piano hammers and control body.
Mix Gentle|Bus|-14 1.5 40 240 12 0 100 0 3 140|Subtle bus control after a balanced mix.
Master Polish|Mastering|-12 1.3 50 300 12 0 100 0 2 180|Low-ratio finishing with conservative range.
Pump Starter|Creative|-28 8 2 220 2 0 70 7 16 30|A deliberate pumping envelope; route a rhythmic key if desired.
Transient Tamer|Utility|-12 8 0.3 45 3 0 100 0 5 20|Catch short overshoots before a slower leveler.
""","Set threshold for the intended gain reduction on your own material.")
rows("gate","threshold range ratio attack hold release style knee hysteresis sc_hpf sc_lpf",r"""
Vocal Breath Keeper|Vocal|-48 10 2 2 80 220 1 8 4 90 14000|Gently lower room noise between sung phrases.
Close Vocal Cleanup|Vocal|-42 18 3 1 100 180 1 6 4 110 13000|Reduce headphone spill while retaining phrase ends.
Backing Vocal Tight|Vocal|-38 24 4 1 70 140 0 4 5 160 12000|Tighten a backing stack before compression.
Whisper Soft Expansion|Vocal|-58 8 1.5 5 140 400 1 12 3 100 12000|Subtle expansion for quiet intimate lines.
Speech Room Lower|Dialogue|-44 12 2 2 120 250 1 10 4 80 12000|Reduce the room between words without hard closure.
Podcast Pause Cleanup|Dialogue|-40 20 3 1 150 300 1 8 5 90 10000|Long hold prevents chatter on spoken passages.
Broadcast Firm Gate|Dialogue|-35 30 6 0.5 100 160 0 5 6 120 10000|Firm pause separation for a controlled booth.
Kick Isolation|Drums|-26 50 30 0.1 45 100 0 2 5 35 2500|Focus the key detector on kick body.
Snare Isolation|Drums|-30 45 20 0.1 60 160 0 3 4 150 9000|Keep the snare ring after a quick opening.
Tom Natural|Drums|-32 42 12 0.2 120 420 0 4 6 70 6000|Long release follows a ringing tom.
Tom Tight|Drums|-28 55 30 0.1 70 150 0 2 5 80 5000|Shorter drum tails for a dense arrangement.
Drum Room Rhythm|Creative|-30 35 8 1 90 100 3 3 5 120 9000|Shape room decay into an audible rhythmic envelope.
Percussion Detail|Drums|-38 22 4 0.2 40 120 1 6 3 500 14000|Keep fast high percussion above its spill.
Bass Quiet Gaps|Bass|-48 24 3 2 130 300 1 8 5 30 5000|Allow bass decay before gently closing the gaps.
Guitar Amp Hiss|Instruments|-50 30 4 1 90 200 1 8 4 100 9000|Reduce amp hiss between phrases.
Guitar Palm Mute|Instruments|-32 50 15 0.1 40 65 0 2 5 90 8000|A fast stop for tightly muted rhythm guitar.
Acoustic Room|Instruments|-52 12 2 3 160 350 1 10 4 100 12000|A slow, modest room-noise reduction.
Keyed Texture|Creative|-24 60 40 3 80 180 0 2 5 50 16000|Open a texture from an external rhythmic source.
Reverb Tail Shape|Creative|-38 36 6 4 200 90 3 4 4 150 12000|Shape an inserted effect tail into a finite space.
Transparent Expansion|Utility|-60 6 1.3 6 180 500 1 12 2 20 20000|Very gentle expansion for already clean material.
""","Set threshold just above the unwanted sound; audition phrase endings.")
rows("deesser","threshold range freq mode release detection sc_lpf lookahead attack sensitivity transient_protect",r"""
Lead Natural|Vocal|-28 5 6200 0 70 0 14000 2 0.5 55 20|A restrained split-band control for a clear lead.
Bright Lead Soft|Vocal|-30 7 7200 0 65 0 16000 3 0.3 65 15|Smooth brighter upper sibilants.
Low Voice Sibilance|Vocal|-26 6 4500 0 90 0 11000 2 0.7 55 25|Target a lower sibilance center.
Airy Voice Guard|Vocal|-32 4 9000 0 55 0 18000 2 0.4 60 30|Keep airy highs while restraining sharp esses.
Rap Crisp|Vocal|-25 7 6500 0 45 0 14000 1 0.2 65 35|Fast recovery between dense syllables.
Backing Stack Soft|Vocal|-31 8 5800 0 100 0 14000 3 0.6 60 15|Control the cumulative top of a vocal stack.
Wideband Phrase|Vocal|-24 4 6000 1 90 0 14000 3 0.5 50 20|Lower the whole vocal briefly during an ess.
Whisper Edge|Vocal|-36 5 7800 0 120 0 17000 4 1 70 10|Reduce sustained breath hiss on quiet lines.
Dialogue Close|Dialogue|-29 5 5200 0 85 0 12000 3 0.5 60 25|Tame close speech without darkening vowels.
Podcast Bright Mic|Dialogue|-27 6 6800 0 80 0 15000 2 0.4 55 20|A practical starting point for a bright speech microphone.
Narration Smooth|Dialogue|-32 4 5600 0 110 0 13000 4 0.8 60 15|Slow recovery for measured narration.
Live Dialogue|Dialogue|-24 6 4900 1 65 1 12000 1 0.3 50 35|Firm, quick control of speech peaks.
Cymbal Edge|Drums|-25 5 8000 0 60 1 18000 1 0.2 45 65|Lower harsh cymbal wash while protecting hits.
Hi Hat Smooth|Drums|-24 4 10000 0 35 1 20000 1 0.1 40 75|Fast high-frequency control for tight hats.
Overhead Wash|Drums|-30 6 7600 0 130 1 18000 3 0.6 55 50|Reduce sustained overhead brightness.
Acoustic Pick|Instruments|-22 4 4200 0 55 1 10000 2 0.2 45 65|Control upper pick noise without removing the instrument body.
Guitar Fizz|Instruments|-28 7 4800 0 130 1 12000 3 0.8 55 20|Reduce sustained distorted-guitar fizz.
Synth Brightness|Instruments|-24 5 8500 0 110 1 18000 2 0.5 50 40|Restrain a bright synth layer.
Mix Top Guard|Bus|-20 2 9500 0 120 2 18000 4 1 40 70|Very light broadband-material high control.
Master Subtle|Mastering|-18 1.5 11000 0 160 2 20000 5 1.5 35 80|A conservative final safeguard for excess top energy.
""","Sweep the detector on your source before increasing reduction.")
rows("limiter","gain ceiling release lookahead style attack true_peak oversampling link",r"""
Transparent Safety|Utility|0 -1 100 3 0 3 1 2 100|Catch unexpected peaks with a conservative output ceiling.
Vocal Peak Guard|Vocal|2 -1.5 80 2 0 2 1 1 100|Light peak control after vocal compression.
Dialogue Delivery|Dialogue|3 -1 140 4 1 4 1 2 100|A measured speech starting point with linked stereo.
Podcast Final Guard|Dialogue|4 -1.5 180 4 0 4 1 2 100|Conservative peak headroom for a finished podcast.
Drum Punch Guard|Drums|3 -0.8 45 1.5 2 1 1 2 80|Preserve drum attack with quick recovery.
Drum Dense|Drums|5 -1 70 2 3 2 1 2 90|A denser drum bus; compare transients carefully.
Bass Peak Guard|Bass|2.5 -1 180 4 0 4 1 2 100|Longer recovery follows low-frequency cycles.
Acoustic Safety|Instruments|1.5 -1.2 110 3 0 3 1 1 100|A small safety margin for acoustic peaks.
Piano Peaks|Instruments|2 -1 150 4 1 4 1 2 100|Control large piano accents with a smooth recovery.
Mix Streaming Start|Mastering|3 -1 180 5 0 5 1 2 100|A peak-safe starting point; loudness remains source dependent.
Mix Open|Mastering|1.5 -1 240 6 1 6 1 2 100|Gentle finishing for an open dynamic arrangement.
Mix Compact|Mastering|5 -1 100 3 3 3 1 2 100|Compact program peaks for a dense mix.
Electronic Firm|Mastering|6 -1 65 2 4 2 1 2 100|Firm control for sustained electronic material.
Rock Bus Energy|Bus|4 -0.9 90 3 2 3 1 2 95|Balance density and drum movement on a rock bus.
Orchestral Safety|Mastering|0.5 -1.5 350 8 0 8 1 2 100|Minimal peak intervention on wide dynamics.
Ambient Long|Mastering|2 -1 450 8 1 8 1 2 100|Smooth recovery over long atmospheric tones.
Parallel Density|Creative|8 -2 55 1 5 1 0 1 100|An intentionally dense branch for parallel routing.
Transient Soft|Bus|2.5 -1.2 120 5 6 5 1 2 100|Round isolated overshoots with a longer lookahead.
Quick Preview|Utility|1 -1 100 2 0 2 1 1 100|Modest oversampling for fast interactive preview.
High Quality Finish|Mastering|2 -1 200 6 0 6 1 3 100|Extra oversampling for a final offline comparison.
""","Measure rendered loudness and true peak; this preset does not guarantee a delivery loudness.")
rows("reverb","predelay decay size damping diffusion width low_cut high_cut mix modulation style early_level",r"""
Vocal Intimate Room|Vocal|18 0.7 35 6500 80 95 180 10000 14 12 0 18|A short space that keeps the voice close.
Vocal Clear Plate|Vocal|38 1.8 65 9000 90 110 200 13000 19 18 1 8|Predelay separates consonants from a smooth tail.
Vocal Dark Plate|Vocal|45 2.4 70 4500 95 110 240 8000 20 20 1 5|A darker tail behind a bright lead.
Vocal Wide Hall|Vocal|55 3.2 85 7000 90 130 250 11000 18 25 0 10|A spacious halo for slower lead lines.
Backing Glue Room|Vocal|12 1.2 55 6000 85 100 220 10000 16 10 0 20|Place a backing stack in one shared room.
Dialogue Small Room|Dialogue|8 0.45 25 5500 65 80 180 9000 8 0 0 25|A restrained room cue for dry recorded speech.
Snare Bright Plate|Drums|12 1.4 60 10000 85 105 220 15000 23 10 1 15|A bright snare tail with a clean low end.
Snare Dark Room|Drums|5 0.8 45 4500 75 95 180 7500 24 5 2 30|Short, dark drum ambience.
Drum Chamber|Drums|15 1.1 55 7000 75 115 300 11000 18 8 0 35|A common room around a drum group.
Percussion Air|Drums|3 0.6 40 12000 70 120 500 18000 12 15 0 20|A light top-end space for small percussion.
Acoustic Warm Room|Instruments|20 1.3 55 5500 85 100 160 9500 18 12 0 25|A warm room around acoustic guitar.
Piano Concert Start|Instruments|30 2.8 90 7500 92 120 120 13000 20 18 0 20|A broad hall around a solo piano.
Strings Bloom|Instruments|28 3.6 90 6500 95 130 180 11500 22 25 0 12|A slow diffuse bloom behind sustained strings.
Synth Cloud|Creative|65 6.5 100 5000 100 145 300 9500 30 45 0 5|A modulated dark cloud for pads.
Shimmer Foundation|Creative|80 8 100 11000 100 145 400 18000 25 55 1 4|A bright long tail ready for layered effects.
Vintage Narrow|Creative|6 1.6 50 3500 70 45 250 6500 26 18 2 35|A narrow, colored room impression.
Distant Stage|Creative|0 4.5 95 4500 90 125 300 8000 35 20 0 45|Push a texture farther into a large space.
Mix Tiny Ambience|Bus|7 0.35 30 8000 80 95 350 13000 5 0 0 15|A very small shared ambience for a dry mix.
Aux Vocal Hall|Send|48 2.6 80 7000 92 125 240 12000 100 22 0 12|Fully wet send return for a vocal hall.
Aux Drum Room|Send|10 0.9 45 6500 78 110 250 11000 100 8 0 30|Fully wet send return for a drum room.
""","Use Mix for inserts; the Aux recipes are fully wet.")
rows("delay","time feedback offset pingpong low_cut high_cut drive width mix ducking mod_rate mod_depth",r"""
Vocal Slap Mono|Vocal|95 8 0 0 180 6000 1 0 13 4 0.4 0|A short mono thickening echo.
Vocal Slap Stereo|Vocal|125 12 8 0 200 7000 1 90 15 5 0.5 0.4|A slightly spread slap behind the lead.
Vocal Quarter Start|Vocal|500 28 0 0 250 7000 1 100 18 12 0.3 0.5|A quarter-note echo at 120 BPM; enable project sync if needed.
Vocal Dotted Start|Vocal|375 32 0 1 300 6500 1.5 100 18 14 0.4 0.6|A dotted-eighth echo at 120 BPM.
Vocal Long Duck|Vocal|750 38 0 1 350 6000 2 100 20 20 0.25 0.8|Long repeats recede while the voice is active.
Backing Spread|Vocal|22 0 35 0 250 10000 0 100 20 0 0.6 1.2|Short detuned motion for a backing texture.
Dialogue Memory|Dialogue|420 22 0 0 400 4500 2 45 17 8 0.2 0.5|A dark, receding narrative echo.
Guitar Tape Slap|Instruments|145 18 -3 0 180 5000 4 70 19 0 0.35 1.2|A worn slap on picked guitar.
Guitar Lead Echo|Instruments|440 36 6 1 250 6500 3 100 24 8 0.4 1|A spacious lead-guitar repeat.
Piano Soft Echo|Instruments|600 26 0 1 200 8000 0 100 15 5 0.2 0.3|A subtle musical echo behind piano.
Percussion Ping|Drums|250 24 0 1 500 11000 0 100 16 0 0.6 0|Clean alternating percussion repeats.
Drum Dub|Drums|333 55 -5 1 300 4500 6 100 23 4 0.7 2|A dark feedback texture for a dub arrangement.
Bass Dub Echo|Bass|500 32 0 0 100 3000 4 45 15 8 0.25 0.6|Low-mid repeats that leave space for the direct bass.
Synth Wide Echo|Creative|625 42 12 1 350 9000 2 100 27 6 0.35 1.5|Wide repeating motion behind a synth line.
Ambient Drift|Creative|1100 62 7 1 450 5000 3 100 32 4 0.12 3|Long, filtered repeats with slow drift.
Bright Cascade|Creative|188 48 14 1 800 14000 1 100 25 0 1.2 1|A fast bright cascade on sparse input.
Broken Tape|Creative|280 45 -12 0 300 3500 8 65 28 0 2.3 5|Obvious modulation and drive for a degraded echo.
Tiny Double|Utility|16 0 -20 0 250 12000 0 80 12 0 0.7 0.6|A low-blend short doubling texture.
Aux Clean Echo|Send|500 30 0 1 250 9000 0 100 100 12 0.3 0|A fully wet clean send return.
Aux Dark Echo|Send|750 44 5 1 400 4500 4 100 100 18 0.25 1.5|A fully wet dark send with ducking.
""","Adjust time or tempo sync to the project and check feedback tails.")
rows("saturation","drive style tone mix output oversampling dynamics",r"""
Vocal Warmth|Vocal|3 0 -0.5 45 -1 1 0|A small harmonic lift under a lead.
Vocal Presence Color|Vocal|4 1 1 35 -1.5 1 0|A brighter parallel vocal color.
Backing Stack Density|Vocal|5 2 -1 40 -2 1 10|Add shared density beneath stacked voices.
Breathy Vintage|Vocal|6 3 -2 30 -2 1 -10|A darker rough edge on an intimate voice.
Dialogue Weight|Dialogue|2 0 -1 50 -0.8 1 0|A restrained body boost for speech.
Podcast Edge|Dialogue|3.5 1 0.5 30 -1 1 5|Add definition at a conservative blend.
Kick Harmonics|Drums|7 4 -1 60 -3 2 15|Make a kick more audible on small speakers.
Snare Bite|Drums|8 5 2 45 -3 2 20|Add upper attack and density to snare.
Drum Parallel Dirt|Drums|14 7 -2 25 -4 2 30|A deliberately dirty parallel drum layer.
Drum Bus Warm|Bus|4 2 -0.5 50 -1.5 2 5|Gentle harmonic glue across drums.
Bass Round|Bass|6 0 -2 65 -2.5 2 10|A round low-frequency harmonic foundation.
Bass Growl|Bass|10 6 1.5 50 -4 2 20|More midrange growl for an exposed bass.
Bass Translation|Bass|8 1 3 40 -3 2 5|Lift audible bass harmonics while blending direct low end.
Acoustic Texture|Instruments|3 3 -0.5 30 -1 1 0|A subtle textured layer under acoustic guitar.
Piano Felt Color|Instruments|4 2 -2 40 -1.5 1 -5|Round the brightest piano accents.
Guitar Rough Edge|Instruments|12 8 1 60 -5 2 25|A gritty guitar color with a restrained output.
Synth Fold Color|Creative|10 12 2 40 -4 2 10|A more complex synth timbre; inspect high-frequency energy.
Lo Fi Layer|Creative|9 18 -4 35 -4 1 -15|A filtered, colored layer for creative blending.
Mix Gentle Harmonics|Mastering|1.5 0 0 35 -0.5 2 0|Very subtle mix harmonics at matched loudness.
Master Warm Finish|Mastering|2 2 -0.5 40 -0.7 3 0|A conservative high-quality finishing color.
""","Match bypass loudness and lower drive if the source is already dense.")
# Multiband dynamics recipes use a few purposeful bands; untouched ranges stay zero.
mbrows=[
("Vocal Low Mid Tidy","Vocal",2,-28,-4,2.5,12,150),
("Vocal Presence Hold","Vocal",4,-25,-3,2,5,95),
("Vocal Air Calm","Vocal",5,-30,-3.5,2.5,2,80),
("Backing Warm Balance","Vocal",3,-30,-4,3,15,170),
("Dialogue Boom Control","Dialogue",2,-27,-5,3,10,180),
("Dialogue Edge Control","Dialogue",4,-26,-3,2.5,4,100),
("Podcast Low Body","Dialogue",1,-24,-3,2,25,220),
("Kick Low Consistency","Drums",1,-23,-6,4,20,120),
("Snare Body Hold","Drums",3,-25,-5,3,18,140),
("Cymbal Wash Control","Drums",5,-28,-4,2.5,3,160),
("Drum Bus Low Glue","Bus",2,-22,-3,2,30,180),
("Bass Sub Anchor","Bass",1,-26,-5,3,30,200),
("Bass Mid Growl Hold","Bass",3,-24,-4,3,12,120),
("Acoustic Low Mid","Instruments",2,-28,-3,2,20,160),
("Piano Mid Balance","Instruments",3,-25,-2.5,2,30,240),
("Guitar Upper Bite","Instruments",4,-24,-4,3,8,110),
("Mix Gentle Low","Mastering",1,-20,-1.5,1.5,40,250),
("Mix Gentle Top","Mastering",5,-22,-1.5,1.5,15,200),
("Parallel Low Density","Creative",2,-32,4,2,25,220),
("Wideband Dynamic Glue","Bus",4,-20,-2,1.5,25,180),
]
for name,cat,b,t,r,ratio,a,rel in mbrows:
    v={f"b{b}.threshold":t,f"b{b}.range":r,f"b{b}.ratio":ratio,f"b{b}.attack":a,f"b{b}.release":rel}
    if name=="Wideband Dynamic Glue":v.update({"layout":1,f"b{b}.low":60,f"b{b}.high":16000,f"b{b}.slope":12})
    add("multiband",name,cat,"Control the selected frequency region while other bands remain neutral. Adjust threshold for the source and compare at matched level.",v,("frequency dynamics",))
filterrows=r"""
Warm Low Pass|Tone|4800 0.75 0 1 2 0 0 0.5 0|Round a bright track.
Deep Low Pass|Tone|1200 0.8 0 2 3 0 0 0.5 -1|Leave only a warm foundation.
Air High Pass|Tone|900 0.75 1 1 0 0 0 0.5 0|Isolate upper texture.
Bass High Pass|Utility|65 0.707 1 2 0 0 0 0.5 0|Remove very low motion.
Vocal Band Texture|Vocal|1800 1.2 2 1 2 0 0 0.5 -1|Focus a vocal texture into the midrange.
Telephone Motion|Creative|1200 1.5 2 2 4 4 0 0.4 -2|Animate a narrow midrange voice.
Soft Auto Wah|Creative|700 2.4 0 1 3 0 16 0.5 -2|Open the filter from the source envelope.
Sharp Auto Wah|Creative|420 4.5 0 2 5 0 24 0.5 -4|An assertive envelope accent.
Reverse Envelope|Creative|4800 2 0 1 3 0 -18 0.5 -2|Close the filter as the source becomes louder.
Slow Sweep|Motion|2300 1.8 0 1 2 14 0 0.16 -2|A slow timbral sweep.
Fast Wobble|Motion|900 3 0 2 6 22 0 3.5 -4|A fast resonant bass movement.
Gentle Pulse|Motion|3600 1.1 0 1 1 6 0 1.2 -1|A small periodic color change.
Bright Resonance|Tone|6800 3 0 0 1 0 0 0.5 -3|A focused high-frequency emphasis.
Low Resonance|Bass|180 3.5 0 2 4 0 10 0.5 -4|A resonant bass envelope.
Dark Drum Loop|Drums|2800 1.2 0 2 5 0 8 0.5 -3|Round a drum loop while retaining envelope movement.
Thin Drum Loop|Drums|750 1.1 1 1 3 5 0 0.7 -2|Create a lighter rhythmic layer.
Notch Sweep|Creative|1600 2.5 3 1 2 16 0 0.22 -1|Move a notch across a broad texture.
Soft Pad Movement|Motion|4200 0.9 0 1 1 10 0 0.1 -1|Slowly open and close a sustained pad.
Gritty Mid Focus|Creative|950 2.2 2 2 10 6 10 0.8 -5|Drive and animate a compact midrange.
Clean Top Trim|Utility|14500 0.707 0 1 0 0 0 0.5 0|A high low-pass edge for an already balanced signal.
"""
for device in ("filter","simplon"):
    for line in filterrows.strip().splitlines():
        name,cat,num,hint=line.split("|");f,q,shape,slope,drive,lfo,env,rate,out=map(float,num.split())
        v={"f1.freq":f,"f1.resonance":q,"f1.shape":shape,"f1.slope":slope,"f1.drive":drive,"lfo_depth":lfo,"env_depth":env,"lfo_rate":rate,"output":out,"oversampling":1}
        if device=="simplon":
            v.update({"f2.enabled":1,"f2.shape":1,"f2.freq":40 if shape!=1 else 80,"f2.slope":1})
            if shape==1:v["f2.shape"]=0;v["f2.freq"]=16000
        else:
            v["f2.enabled"]=0
        add(device,name,cat,hint+" Control resonance and output together when sweeping.",v)
rows("micro","freq resonance shape drive env_depth env_attack env_release mix output",r"""
Warm Edge|Tone|5500 0.75 0 1 0 10 250 100 0|Gently soften the top.
Dark Bed|Tone|1400 0.8 0 2 0 10 250 100 -1|Create a darker background texture.
Sub Cleanup|Utility|55 0.707 1 0 0 10 250 100 0|Remove slow low-frequency motion.
Light Layer|Utility|900 0.8 1 1 0 10 250 100 -1|Keep a light upper layer.
Vocal Soften|Vocal|8500 0.75 0 1 0 20 200 100 0|Reduce excess top on a vocal texture.
Vocal Warm Motion|Vocal|4200 1.1 0 2 6 25 220 80 -1|Add gentle envelope movement behind a voice.
Bass Round|Bass|950 1.2 0 3 8 20 250 100 -2|Open a warm bass timbre with its envelope.
Bass Pluck|Bass|240 2.5 0 4 24 2 140 100 -3|A quick resonant bass accent.
Bass Reverse|Bass|3200 1.8 0 3 -18 8 180 100 -2|Darken the bass attack and reveal the release.
Drum Low Room|Drums|1800 1 0 3 6 12 180 75 -2|A dark moving drum layer.
Drum Air Layer|Drums|1600 1.2 1 2 8 5 120 70 -2|Keep the top rhythm moving.
Snare Body Color|Drums|3000 1.6 0 4 12 1 140 65 -3|A resonant envelope around snare body.
Acoustic Warm Pick|Instruments|6200 1 0 2 5 3 180 80 -1|A subtle moving color on picked material.
Guitar Auto Wah|Instruments|650 3 0 5 26 2 220 100 -4|An envelope-driven wah for guitar.
Piano Dark Layer|Instruments|2200 0.9 0 2 3 30 350 55 -1|A warm parallel piano layer.
Envelope Bright Burst|Creative|1100 4 0 7 32 0.5 100 100 -5|Sharp, bright filter bursts.
Slow Bloom|Creative|800 1.5 0 3 20 300 900 100 -2|A slow opening texture on sustained audio.
Reverse Bloom|Creative|7000 2 0 3 -28 200 800 100 -3|A sustained negative envelope movement.
Resonant High Pass|Creative|450 3.5 1 5 10 10 250 90 -4|A resonant low-end removal effect.
Tiny Color|Utility|12500 0.707 0 0.5 0 10 250 35 0|A nearly transparent parallel touch.
""","Input level controls the envelope; compare the dry/wet balance.")
synthrows=r"""
Soft Analog Bass|Bass|0 3 180 45 100 700 1.1 38 -12 1 50 2|A round, short bass with controlled filter movement.
Pluck Bass|Bass|1 2 140 10 70 450 1.8 65 -13 1 35 4|A pointed low-end pluck.
Deep Sub|Bass|3 5 500 85 160 220 0.7 5 -12 1 50 0|A restrained low waveform for a solid foundation.
Rubber Bass|Bass|1 4 220 25 110 620 2.8 60 -14 1 40 6|A resonant elastic bass envelope.
Mono Glide Lead|Lead|0 8 350 70 220 3600 1.5 35 -14 1 50 3|A connected monophonic lead.
Soft Square Lead|Lead|1 12 450 75 260 2800 1.1 25 -13 1 45 1|A rounded pulse lead.
Bright Saw Lead|Lead|0 4 240 65 180 6500 1.2 35 -16 1 50 4|A forward lead with a quick attack.
Hollow Lead|Lead|1 6 380 60 240 1900 2 40 -14 1 25 2|A narrow pulse lead with a hollow body.
Warm Keys|Keys|2 12 700 45 420 3200 0.8 22 -12 10 50 1|Soft polyphonic keys for chords.
Glass Pluck|Keys|3 2 600 0 450 10000 0.7 15 -12 10 50 0|A clean, long-decaying pluck.
Muted Pluck|Keys|0 2 180 0 120 1700 1.1 45 -13 10 50 1|A short muted chord texture.
Pulse Mallet|Keys|1 3 400 0 250 4200 1.4 30 -14 10 30 2|A percussive pulse tone.
Warm Pad|Pad|0 550 1600 75 1600 1900 0.8 15 -16 10 50 1|A slow, warm sustained chord.
Air Pad|Pad|2 800 2000 80 2200 6200 0.7 10 -15 10 50 0|A light pad with long transitions.
Dark Pad|Pad|1 900 1600 85 2600 1000 0.9 8 -16 10 38 1|A dark supporting bed.
Slow Brass|Pad|0 100 700 70 600 2400 1.6 50 -16 10 50 3|A slower filtered brass-like onset.
Tiny Pulse|Sequence|1 1 120 0 70 5200 1.1 20 -15 10 20 2|A compact step-sequence sound.
Acid Start|Sequence|0 2 180 15 80 650 5 75 -18 1 50 8|A resonant monophonic sequence starting point.
Retro Organ|Keys|1 2 100 100 90 7000 0.7 0 -15 10 50 0|An immediate held tone for organ-style parts.
Soft Triangle|Utility|2 8 350 65 250 4800 0.7 5 -12 10 50 0|A clear simple tone for arrangement checks.
"""
for device in ("synth","one"):
    for line in synthrows.strip().splitlines():
        name,cat,num,hint=line.split("|")
        fields="osc1 attack decay sustain release cutoff resonance env_amount gain voices pulse_width drive".split()
        v=dict(zip(fields,map(float,num.split())))
        if "Glide" in name:v["portamento"]=95
        if device=="synth":
            v.update(osc2_mix=25 if cat!="Bass" else 12,detune=5 if cat!="Pad" else 11,width=60 if cat!="Bass" else 10,high_quality=1)
            if cat=="Pad":v.update(osc3_mix=16,unison=3,fx_chorus=18,fx_reverb=12)
            if cat=="Lead":v.update(fx_delay=10,fx_delay_time=375,fx_delay_feedback=22)
            if cat=="Sequence":v.update(arp_on=1,arp_sync=5,arp_gate=55,arp_octaves=2)
        add(device,name,cat,hint+" Play in the intended register and leave headroom before bus processing.",v,("original sound",))
assert len(P)==280,len(P)
assert len({p["id"] for p in P})==280
if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=pathlib.Path,default=pathlib.Path(__file__).resolve().parents[1]/"crates/verdant-plugins/src/factory_presets.json")
    parser.add_argument("--inventory",type=pathlib.Path)
    args=parser.parse_args()
    if args.inventory:
        inventory=json.loads(args.inventory.read_text())
        ds={d["id"]:{p["id"]:p for p in d["descriptors"]}for d in inventory["devices"]}
        for p in P:
            for k,v in p["parameters"].items():
                d=ds[p["device"]][k]
                assert d["min"]<=v<=d["max"],(p["id"],k,v)
                if d["step_count"]:assert abs((v-d["min"])/(d["max"]-d["min"])*d["step_count"]-round((v-d["min"])/(d["max"]-d["min"])*d["step_count"]))<1e-8,(p["id"],k,v)
    args.output.write_text(json.dumps(dict(schema="verdant-factory-presets/1",version="1.0.0",presets=P),ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    print(f"wrote {len(P)} original presets for {len(set(p['device']for p in P))} native devices")
