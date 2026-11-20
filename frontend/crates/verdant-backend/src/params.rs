//! Parameter ↔ renderer capability matrix (SPEC §8, §12). Every editable
//! parameter states how the *current* route realises it, or why it cannot.
//! A parameter with no real mapping is shown disabled with its reason; it is
//! never offered as an effective control.

use serde::Serialize;

use crate::wire::{Capabilities, Route};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Param {
    /// Pitch bend curve (VOCALOID PIT; stored as note-relative millicents).
    Pit,
    Vibrato,
    Portamento,
    /// Dynamics (VOCALOID DYN; stored as note-relative millidecibels).
    Dyn,
    Breathiness,
    Brightness,
    Clearness,
    Gender,
    Opening,
    Growl,
}

pub const ALL: [Param; 10] = [
    Param::Pit,
    Param::Vibrato,
    Param::Portamento,
    Param::Dyn,
    Param::Breathiness,
    Param::Brightness,
    Param::Clearness,
    Param::Gender,
    Param::Opening,
    Param::Growl,
];

impl Param {
    pub fn short(self) -> &'static str {
        match self {
            Param::Pit => "PIT",
            Param::Vibrato => "VIB",
            Param::Portamento => "POR",
            Param::Dyn => "DYN",
            Param::Breathiness => "BRE",
            Param::Brightness => "BRI",
            Param::Clearness => "CLE",
            Param::Gender => "GEN",
            Param::Opening => "OPE",
            Param::Growl => "GWL",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Param::Pit => "音高弯曲",
            Param::Vibrato => "颤音",
            Param::Portamento => "滑音",
            Param::Dyn => "力度",
            Param::Breathiness => "气声",
            Param::Brightness => "明亮度",
            Param::Clearness => "清晰度",
            Param::Gender => "性别因子",
            Param::Opening => "开口度",
            Param::Growl => "嘶吼",
        }
    }
    pub fn is_expression(self) -> bool {
        matches!(self, Self::Breathiness | Self::Brightness | Self::Clearness | Self::Gender | Self::Opening | Self::Growl | Self::Portamento)
    }
    pub fn default_value(self) -> f64 {
        verdant_domain::model::expression_default(self.short()).unwrap_or(0) as f64
    }
    pub fn samples(self, note: &verdant_domain::Note) -> Vec<(i64, f64)> {
        match self {
            Self::Pit => note.curves.pitch_millicent_points.iter().map(|p| (p.tick, p.millicents as f64 / 1000.)).collect(),
            Self::Dyn => note.curves.gain_millidecibel_points.iter().map(|p| (p.tick, p.millidecibels as f64 / 1000.)).collect(),
            Self::Portamento => note.curves.expression("POR").first()
                .map(|_| vec![(0, crate::f0::portamento_position(note))]).unwrap_or_default(),
            _ if self.is_expression() => note.curves.expression(self.short()).iter().map(|p| (p.tick, p.value as f64)).collect(),
            _ => Vec::new(),
        }
    }
    pub fn point_samples(self, note: &verdant_domain::Note) -> Vec<(verdant_domain::PointId, i64, f64)> {
        match self {
            Self::Pit => note.curves.pitch_millicent_points.iter().map(|p| (p.id.clone(), p.tick, p.millicents as f64 / 1000.)).collect(),
            Self::Dyn => note.curves.gain_millidecibel_points.iter().map(|p| (p.id.clone(), p.tick, p.millidecibels as f64 / 1000.)).collect(),
            Self::Portamento => note.curves.expression("POR").first()
                .map(|point| vec![(point.id.clone(), 0, crate::f0::portamento_position(note))]).unwrap_or_default(),
            _ if self.is_expression() => note.curves.expression(self.short()).iter().map(|p| (p.id.clone(), p.tick, p.value as f64)).collect(),
            _ => Vec::new(),
        }
    }
    pub fn interpolate_note(self,note:&verdant_domain::Note,tick:f64)->f64{
        let tick=if self==Self::Portamento{0.}else{tick};
        let mode=note.curves.interpolation(self.short());
        match self{
            Self::Pit=>verdant_domain::editing::interpolate(&note.curves.pitch_millicent_points,tick,0.,mode,|p|p.tick,|p|p.millicents as f64/1000.),
            Self::Dyn=>verdant_domain::editing::interpolate(&note.curves.gain_millidecibel_points,tick,0.,mode,|p|p.tick,|p|p.millidecibels as f64/1000.),
            _ if self.is_expression()=>verdant_domain::editing::interpolate(note.curves.expression(self.short()),tick,self.default_value(),mode,|p|p.tick,|p|f64::from(p.value)),
            _=>self.default_value(),
        }
    }
    pub fn interpolate(self, points: &[(i64, f64)], tick: f64) -> f64 {
        if points.is_empty() { self.default_value() }
        else { crate::f0::interp(points, if self == Self::Portamento { 0. } else { tick }) }
    }
}

/// A POR edit sets the destination note's one effective onset value. Merely
/// opening/importing a legacy multi-point curve does not mutate it; an explicit
/// edit canonicalizes it while retaining the effective point's ID/metadata.
pub fn portamento_points_for_value(note: &verdant_domain::Note, value: f64) -> Vec<verdant_domain::model::ExpressionPoint> {
    let mut point = note.curves.expression("POR").first().cloned().unwrap_or_else(||
        verdant_domain::model::ExpressionPoint { id: verdant_domain::PointId::generate(), tick: 0, value: 64, unknown: Default::default() });
    point.tick = 0;
    point.value = value.round().clamp(0., 127.) as u8;
    vec![point]
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Support {
    /// Realised by the synthesiser itself.
    Synthesis(&'static str),
    /// Realised after rendering, recorded separately from synthesis.
    PostProcess(&'static str),
    Unsupported(&'static str),
}

impl Support {
    pub fn is_effective(&self) -> bool {
        !matches!(self, Support::Unsupported(_))
    }
}

pub fn support(p: Param, route: Route, caps: Option<&Capabilities>) -> Support {
    let Some(caps) = caps else { return Support::Unsupported("尚未收到后端能力声明") };
    match p {
        Param::Pit | Param::Vibrato | Param::Portamento
            if caps.native_score.as_ref().is_some_and(|n| n.continuous_f0_control) =>
            Support::Synthesis("连续 F0 与谐波相位"),
        Param::Pit | Param::Vibrato | Param::Portamento =>
            Support::Unsupported("后端未声明逐帧音高控制能力"),
        Param::Dyn => Support::PostProcess("渲染后逐样本增益；不是合成参数，来源单独记录"),
        _ if caps.native_score.as_ref().and_then(|n| n.expression.as_ref())
            .is_some_and(|e| e.methods.iter().any(|m| m == "vocaloid-controls-v1") && e.parameters.iter().any(|k| k == p.short())
                && e.routes.iter().any(|r| r == route.as_str())) =>
            Support::PostProcess("V3/V4 参数语义：重合成后按音素实现；GWL 为独立近似，非原厂引擎"),
        _ => Support::Unsupported("后端未声明该表达参数的实际输出处理能力"),
    }
}
