//! Duration controls expose the frontend allocation budget. Vowel-anchored
//! scheduling separately shows the materialized plan, including adjacent tails.
use super::*;
use verdant_backend::adapter::{Diagnostic, NoteSupport, minimum_segment_seconds, note_support};
use verdant_backend::phoneme_timing::{PhonemeBudget, minimum_permille, note_budget};

pub struct PhonemeTiming {
    pub labels: Vec<String>,
    pub weights: Vec<u32>,
    pub seconds: f64,
    pub minimum_segment_seconds: f64,
    pub initial_seconds: f64,
    pub durations: Vec<f64>,
    pub explicit: bool,
    pub editable: bool,
}

pub struct ScheduledPhonemeTiming {
    pub prefix_seconds: f64,
    pub tail_trim_seconds: f64,
    pub durations: Vec<f64>,
}

type ScheduledNote = (usize, usize, usize);

#[derive(Default)]
pub(crate) struct ScheduledPhonemes(HashMap<TrackId, HashMap<String, ScheduledNote>>);

impl ScheduledPhonemes {
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn rebuild(&mut self, plans: &[PhrasePlan]) {
        self.clear();
        for (plan_index, plan) in plans.iter().enumerate() {
            for (phrase_index, phrase) in plan.phrases.iter().enumerate() {
                let Some(trace) = &phrase.score.phoneme_timing else { continue };
                let track = self.0.entry(plan.track.clone()).or_default();
                for (note_index, note) in trace.notes.iter().enumerate() {
                    track.entry(note.note_id.clone()).or_insert((plan_index, phrase_index, note_index));
                }
            }
        }
    }
}

impl Editor {
    fn with_resolved_note<R>(&self, note: &Note, read: impl FnOnce(&Note) -> R) -> R {
        let mut readings = self.resolved_readings.borrow_mut();
        let readings = readings.get_or_insert_with(|| {
            let Some(track) = self.active_track() else { return Default::default() };
            let Ok(settings) = verdant_backend::phonemizer::settings_from_project(self.project()) else { return Default::default() };
            verdant_backend::phonemizer::resolve_track(track, &settings)
        });
        match readings.notes.get(&note.id) {
            Some(reading) if note.pronunciation != reading.pronunciation => {
                let mut resolved = note.clone();
                resolved.pronunciation = reading.pronunciation.clone();
                read(&resolved)
            }
            _ => read(note),
        }
    }

    pub fn resolved_note_support(&self, note: &Note) -> Option<Result<NoteSupport, Diagnostic>> {
        let caps = self.scheduler.capabilities.as_ref()?;
        Some(self.with_resolved_note(note, |note| note_support(note, caps)))
    }

    pub fn note_phone_budget(&self, note: &Note, tempo: &TempoMap) -> std::cell::Ref<'_, Result<PhonemeBudget, String>> {
        if let Ok(hit) = std::cell::Ref::filter_map(self.phone_budgets.borrow(), |budgets| budgets.get(&note.id)) {
            return hit;
        }
        let budget = self.scheduler.capabilities.as_ref().ok_or_else(|| "等待后端音素能力声明".to_string())
            .and_then(|caps| self.with_resolved_note(note, |note| note_budget(note, tempo, caps, self.route())));
        self.phone_budgets.borrow_mut().insert(note.id.clone(), budget);
        std::cell::Ref::map(self.phone_budgets.borrow(), |budgets| &budgets[&note.id])
    }

    fn scheduled_phoneme_note(&self, id: &NoteId) -> Option<(&verdant_backend::wire::PhonemeTimingNote, &verdant_backend::wire::NativeScore)> {
        let &(plan, phrase, note) = self.scheduled_phonemes.0.get(&self.session.track)?.get(id.as_str())?;
        let score = &self.plans[plan].phrases[phrase].score;
        Some((&score.phoneme_timing.as_ref()?.notes[note], score))
    }

    pub fn scheduled_phoneme_prefix(&self, id: &NoteId) -> Option<f64> {
        self.scheduled_phoneme_note(id).map(|(note, _)| note.prefix_seconds)
    }

    pub fn scheduled_phoneme_timing(&self, id: &NoteId) -> Option<ScheduledPhonemeTiming> {
        let (note, score) = self.scheduled_phoneme_note(id)?;
        let phones = score.segments.get(note.first_segment_index..note.end_segment_index)?;
        Some(ScheduledPhonemeTiming {
            prefix_seconds: note.prefix_seconds,
            tail_trim_seconds: note.tail_trim_seconds,
            durations: phones.iter().map(|phone| phone.duration_seconds).collect(),
        })
    }
    pub fn phoneme_timing(&self, note: &Note) -> Option<PhonemeTiming> {
        let caps = self.scheduler.capabilities.as_ref()?;
        self.with_resolved_note(note, |note| phoneme_timing(note, &self.tempo(), caps, self.route()))
    }

    pub fn phoneme_timing_text(&self) -> String {
        let Some(caps) = self.scheduler.capabilities.as_ref() else { return "等待后端音素时长能力声明".into() };
        if !caps.native_score.as_ref().is_some_and(|ns| ns.phoneme_duration_control) {
            return "当前后端仅提供只读音素时长。".into();
        }
        let ms = minimum_segment_seconds(self.route(), caps) * 1000.0;
        if self.project().render_preferences.phoneme_timing.is_some() {
            format!("元音对拍会结合相邻音符调整韵母长度。下方列出计划时长，每段至少 {ms:.0} ms。")
        } else {
            format!("各音素占比合计 100%，每段至少 {ms:.0} ms。重置可恢复自动切分。")
        }
    }
}

fn phoneme_timing(note: &Note, tempo: &TempoMap, caps: &verdant_backend::Capabilities, route: Route) -> Option<PhonemeTiming> {
        let minimum_segment_seconds = minimum_segment_seconds(route, caps);
        if route == Route::VerdantVoice {
            let budget = note_budget(note, tempo, caps, route).ok()?;
            return Some(PhonemeTiming {
                explicit: note.pronunciation.alphabet.is_some(),
                initial_seconds: if budget.labels.len() > 1 { budget.durations_seconds[0] } else { 0.0 },
                seconds: budget.total_seconds,
                minimum_segment_seconds,
                labels: budget.labels,
                weights: budget.weights_permille.unwrap_or_default(),
                durations: budget.durations_seconds,
                editable: budget.editable,
            });
        }
        let support = note_support(note, caps).ok()?;
        let seconds = tempo.seconds_at(note.end_tick() as f64) - tempo.seconds_at(note.start_tick as f64);
        if let NoteSupport::Phonemes(p) = &support {
            let durations = verdant_backend::phonemes::allocate_for_note(p, seconds,
                note.pronunciation.phoneme_durations_permille.as_deref(), note.pronunciation.mora_weights.as_deref()).ok()?;
            let mut weights: Vec<u32> = durations.iter().map(|&d| (d / seconds * 1000.0).floor() as u32).collect();
            let missing = 1000u32.saturating_sub(weights.iter().sum());
            let mut fractions: Vec<_> = durations.iter().enumerate().map(|(i, &d)| (i, d / seconds * 1000.0 - weights[i] as f64)).collect();
            fractions.sort_by(|a, b| b.1.total_cmp(&a.1));
            for &(i, _) in fractions.iter().take(missing as usize) { weights[i] += 1; }
            return Some(PhonemeTiming {
                labels: p.phones.iter().map(|phone| format!("{} → {}", phone.canonical.symbol, phone.native_key)).collect(),
                weights, seconds, minimum_segment_seconds, initial_seconds: durations[0], durations, explicit: true,
                editable: caps.native_score.as_ref().is_some_and(|ns| ns.phoneme_duration_control),
            });
        }
        let (labels, initial_seconds) = match support {
            NoteSupport::Pinyin(token) if !token.initial.is_empty() => {
                let initial = verdant_backend::pinyin::initial_seconds(&token.initial, seconds);
                (vec![token.initial, token.final_], initial)
            }
            NoteSupport::Pinyin(token) => (vec![token.final_], 0.0),
            NoteSupport::VowelProxy { vowel } => (vec![vowel], 0.0),
            NoteSupport::Unsupported | NoteSupport::Phonemes(_) => return None,
        };
        let weights = note.pronunciation.phoneme_durations_permille.clone().unwrap_or_else(|| {
            if labels.len() == 1 { vec![1000] } else {
                let initial = (initial_seconds / seconds * 1000.0).round().clamp(1.0, 999.0) as u32;
                vec![initial, 1000 - initial]
            }
        });
        let initial_seconds = if labels.len() == 2 {
            if note.pronunciation.phoneme_durations_permille.is_some() { seconds * weights.first().copied().unwrap_or(0) as f64 / 1000.0 }
            else { initial_seconds }
        } else { 0.0 };
        Some(PhonemeTiming {
            durations: if labels.len() == 1 { vec![seconds] } else { vec![initial_seconds, seconds - initial_seconds] },
            explicit: false,
            labels, weights, seconds, minimum_segment_seconds, initial_seconds,
            editable: caps.native_score.as_ref().is_some_and(|ns| ns.phoneme_duration_control),
        })
}

impl Editor {
    pub fn set_phoneme_percent(&mut self, id: &NoteId, initial: bool, percent: f64, cx: &mut Context<Self>) {
        let Some(note) = self.active_track().and_then(|t| self.index.note(t, id)).cloned() else { return };
        let Some(timing) = self.phoneme_timing(&note) else { self.notice("请先设定后端支持的显式拼音"); cx.notify(); return };
        if !timing.editable { self.notice(self.phoneme_timing_text()); cx.notify(); return; }
        if timing.labels.len() == 1 { self.notice("此音节只有韵母，韵母占整音节的 100%"); cx.notify(); return; }
        if timing.labels.len() != 2 { self.notice("请使用各音素旁的时长控件"); cx.notify(); return; }
        let floor = if self.route() == Route::VerdantVoice { 2.0 * timing.minimum_segment_seconds }
            else { verdant_backend::pinyin::MIN_SYLLABLE_SECONDS };
        if timing.seconds < floor - 1e-12 || timing.seconds > verdant_backend::pinyin::MAX_SYLLABLE_SECONDS {
            self.notice(format!("拼音音节须介于 {:.0} ms 和 30 秒之间", floor * 1000.0));
            cx.notify();
            return;
        }
        if !percent.is_finite() { return; }
        let minimum = minimum_permille(timing.seconds, timing.minimum_segment_seconds);
        if minimum > 500 { self.notice("当前长度无法用千分比表示两个音素的最短时长"); cx.notify(); return; }
        let proposed = (percent * 10.0).round() as u32;
        let first = if initial { proposed } else { 1000u32.saturating_sub(proposed) }.clamp(minimum, 1000 - minimum);
        let mut pronunciation = note.pronunciation;
        if pronunciation.alphabet.is_none() { pronunciation.phonemes = timing.labels; }
        pronunciation.phoneme_durations_permille = Some(vec![first, 1000 - first]);
        self.commit(self.revision(), CommandBatch::single("音素时长", Command::SetPronunciation {
            track: self.session.track.clone(), note_id: id.clone(), pronunciation,
        }), cx);
    }

    pub fn set_phone_percent(&mut self, id: &NoteId, index: usize, percent: f64, cx: &mut Context<Self>) {
        let Some(note) = self.active_track().and_then(|t| self.index.note(t, id)).cloned() else { return };
        let Some(timing) = self.phoneme_timing(&note) else { return };
        let count = timing.weights.len();
        if !timing.editable || count < 2 || index >= count { return; }
        let minimum = minimum_permille(timing.seconds, timing.minimum_segment_seconds);
        if minimum as usize * count > 1000 { self.notice("当前长度无法用千分比表示各音素的最短时长"); cx.notify(); return; }
        if !percent.is_finite() { return; }
        let chosen = ((percent * 10.0).round() as i32).clamp(minimum as i32, (1000 - minimum * (count as u32 - 1)) as i32) as u32;
        let others: Vec<_> = (0..count).filter(|&i| i != index).collect();
        let old_excess = others.iter().map(|&i| timing.weights[i].saturating_sub(minimum) as u64).sum::<u64>();
        let budget = 1000 - chosen - minimum * (count as u32 - 1);
        let mut weights = vec![minimum; count];
        weights[index] = chosen;
        for &i in &others {
            weights[i] += if old_excess == 0 { budget / others.len() as u32 }
                else { (budget as u64 * timing.weights[i].saturating_sub(minimum) as u64 / old_excess) as u32 };
        }
        let remainder = 1000 - weights.iter().sum::<u32>();
        for &i in others.iter().take(remainder as usize) { weights[i] += 1; }
        let mut pronunciation = note.pronunciation;
        if pronunciation.alphabet.is_none() { pronunciation.phonemes = timing.labels; }
        pronunciation.phoneme_durations_permille = Some(weights);
        self.commit(self.revision(), CommandBatch::single("音素时长", Command::SetPronunciation {
            track: self.session.track.clone(), note_id: id.clone(), pronunciation,
        }), cx);
    }

    pub fn toggle_phoneme_approximation(&mut self, id: &NoteId, cx: &mut Context<Self>) {
        let Some(note) = self.active_track().and_then(|t| self.index.note(t, id)).cloned() else { return };
        let mut pronunciation = note.pronunciation;
        if pronunciation.alphabet.is_none() { return; }
        pronunciation.allow_approximation = !pronunciation.allow_approximation;
        self.commit(self.revision(), CommandBatch::single("切换音素近似映射", Command::SetPronunciation {
            track: self.session.track.clone(), note_id: id.clone(), pronunciation,
        }), cx);
    }

    pub fn reset_phoneme_durations(&mut self, id: &NoteId, cx: &mut Context<Self>) {
        let Some(note) = self.active_track().and_then(|t| self.index.note(t, id)).cloned() else { return };
        let mut pronunciation = note.pronunciation;
        pronunciation.phoneme_durations_permille = None;
        self.commit(self.revision(), CommandBatch::single("恢复音素自动切分", Command::SetPronunciation {
            track: self.session.track.clone(), note_id: id.clone(), pronunciation,
        }), cx);
    }
}
