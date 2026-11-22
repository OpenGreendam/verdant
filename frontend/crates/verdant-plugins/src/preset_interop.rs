//! Bounded translation of user-supplied FabFilter preset data.
//!
//! This translates parameter meanings to Verdant's own DSP. It does not claim
//! binary compatibility with a plugin, identical filter curves or identical audio.
//! Unknown/unsupported audio fields are returned to the caller for a preview.
//! The strict convenience API refuses a partial translation.
//!
//! Format evidence (independently implemented; no factory preset assets bundled):
//! - Official Pro-Q 4 / Pro-C 3 / Pro-R 2 factory INI presets, downloaded from
//!   https://www.fabfilter.com/download (2026-11-22).
//! - https://github.com/raoulsh/preset-toolkit (MIT), Pro-Q 3 binary record layout.
//! - https://github.com/willosof/rew2fabfilter (MIT), log-frequency and Q encoding.
//! - https://github.com/phierceweb/logicxkit (Apache-2.0),
//!   src/logicxkit/data/translate/FabF_FQ4p.json: measured current shape/slope enums.
//! Older Q1-derived slope enums in some writers are not valid for current Q4.

use std::collections::{BTreeMap, BTreeSet};
use crate::builtin_dsp::{self as dsp, EqParams, Spec};

const MAX_BYTES: usize = 1_048_576;
const MAX_FIELDS: usize = 4096;
const MAX_LINE: usize = 16_384;

#[derive(Clone, Debug, PartialEq)]
pub struct FabFilterImportReport {
    pub product: String,
    pub format_version: u32,
    /// Validated native values only. Callers fill other controls from native
    /// defaults, display both diagnostic lists, then offer explicit partial import.
    pub values: Vec<(String, f64)>,
    pub warnings: Vec<String>,
    pub unsupported: Vec<String>,
}

impl FabFilterImportReport {
    pub fn is_complete(&self) -> bool { !self.values.is_empty() && self.unsupported.is_empty() }
}

/// Inspect and translate on the control thread. No device or project is changed.
pub fn inspect_fabfilter_preset(bytes: &[u8], device: &str) -> Result<FabFilterImportReport, String> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES { return Err("FFP 预设为空或超过 1 MiB".into()); }
    if bytes.starts_with(b"FPMb") { return translate_multiband(bytes, device); }
    if bytes.starts_with(b"FQ3p") {
        let (version, parameters) = read_q3(bytes)?;
        require_device(device, dsp::EQ_ID, "Pro-Q 3")?;
        return translate_eq(parameters, device, "Pro-Q 3", version, false);
    }
    let text = std::str::from_utf8(bytes).ok().map(|s| s.trim_start_matches('\u{feff}'));
    if let Some(text) = text.filter(|s| s.trim_start().starts_with("[Preset]")) {
        let (signature, version, parameters) = read_ini(text)?;
        return match signature.as_str() {
            "FQ4p" => {
                require_version(version, 4, "Pro-Q 4")?;
                require_device(device, dsp::EQ_ID, "Pro-Q 4")?;
                translate_eq(parameters, device, "Pro-Q 4", version, true)
            }
            "FC3p" => {
                require_version(version, 2, "Pro-C 3")?;
                require_device(device, dsp::COMPRESSOR_ID, "Pro-C 3")?;
                translate_comp(parameters, device, version)
            }
            "FR2p" => {
                require_version(version, 6, "Pro-R 2")?;
                require_device(device, dsp::REVERB_ID, "Pro-R 2")?;
                translate_reverb(parameters, device, version)
            }
            _ => Err(format!("未支持的 FFP 文本签名 {signature}，版本 {version}")),
        };
    }
    identify_binary(bytes, device)
}

/// Strict import: unsupported audio fields cause an error before any values are
/// returned. A UI supporting partial translation should use the inspect helper.
pub fn import_fabfilter_preset(bytes: &[u8], device: &str) -> Result<Vec<(String, f64)>, String> {
    let report = inspect_fabfilter_preset(bytes, device)?;
    if report.values.is_empty() || !report.unsupported.is_empty() {
        return Err(format!("{} 不能完整转换：{}", report.product, report.unsupported.join("；")));
    }
    Ok(report.values)
}

fn require_device(actual: &str, expected: &str, product: &str) -> Result<(), String> {
    if actual == expected { Ok(()) } else { Err(format!("{product} 预设应导入 {expected}，当前设备为 {actual}")) }
}
fn require_version(actual: u32, expected: u32, product: &str) -> Result<(), String> {
    if actual == expected { Ok(()) } else { Err(format!("{product} FFP 版本 {actual} 未验证，支持版本为 {expected}")) }
}
fn uint(value: f64, key: &str, max: usize) -> Result<usize, String> {
    if value >= 0.0 && value <= max as f64 && value.fract() == 0.0 { Ok(value as usize) }
    else { Err(format!("FFP 字段 {key} 的枚举值 {value} 无效")) }
}
fn flag(value: f64, key: &str) -> Result<bool, String> { Ok(uint(value, key, 1)? == 1) }

type Parameters = BTreeMap<String, f64>;
fn take(p: &mut Parameters, key: &str, fallback: f64) -> f64 { p.remove(key).unwrap_or(fallback) }
fn required(p: &mut Parameters, key: &str) -> Result<f64, String> {
    p.remove(key).ok_or_else(|| format!("FFP 缺少字段 {key}"))
}

fn used_band(p: &mut Parameters, prefix: &str) -> Result<bool, String> {
    let key = format!("{prefix}Used");
    match p.remove(&key) {
        Some(value) => flag(value, &key),
        None if !p.keys().any(|name| name.starts_with(prefix)) => Ok(false),
        None => Err(format!("FFP 缺少字段 {key}")),
    }
}

fn read_ini(text: &str) -> Result<(String, u32, Parameters), String> {
    if text.contains('\0') { return Err("FFP 文本含 NUL 字节".into()); }
    let mut section = "";
    let mut sections = BTreeSet::new();
    let mut header = BTreeMap::<String, String>::new();
    let mut parameters = Parameters::new();
    for (index, raw) in text.lines().enumerate() {
        if index >= MAX_FIELDS + 32 || raw.len() > MAX_LINE { return Err("FFP 文本行数或行长度超过限制".into()); }
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') { continue; }
        if line.starts_with('[') {
            if !matches!(line, "[Preset]" | "[Parameters]") || !sections.insert(line) {
                return Err(format!("FFP 段名无效或重复：{line}"));
            }
            section = line;
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| format!("FFP 第 {} 行缺少等号", index + 1))?;
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() || key.len() > 160 { return Err("FFP 字段名称无效".into()); }
        match section {
            "[Preset]" => {
                if header.insert(key.to_string(), value.trim_matches('"').to_string()).is_some() {
                    return Err(format!("FFP 字段重复：{key}"));
                }
            }
            "[Parameters]" => {
                let number: f64 = value.parse().map_err(|_| format!("FFP 字段 {key} 不是数字"))?;
                if !number.is_finite() { return Err(format!("FFP 字段 {key} 不是有限数")); }
                if parameters.len() >= MAX_FIELDS || parameters.insert(key.to_string(), number).is_some() {
                    return Err(format!("FFP 参数过多或重复：{key}"));
                }
            }
            _ => return Err("FFP 字段位于有效段之外".into()),
        }
    }
    let signature = header.remove("Signature").ok_or("FFP 缺少 Signature")?;
    let version: u32 = header.remove("Version").ok_or("FFP 缺少 Version")?.parse().map_err(|_| "FFP Version 无效")?;
    if parameters.is_empty() { return Err("FFP 没有参数".into()); }
    Ok((signature, version, parameters))
}

fn read_q3(bytes: &[u8]) -> Result<(u32, Parameters), String> {
    const COUNT: usize = 334;
    if bytes.len() != 12 + COUNT * 4 { return Err("Pro-Q 3 FFP 长度错误：应为 334 个 float32 参数".into()); }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    require_version(version, 4, "Pro-Q 3")?;
    if u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != COUNT as u32 {
        return Err("Pro-Q 3 FFP 参数数量无效".into());
    }
    let mut values = [0.0; COUNT];
    for (i, chunk) in bytes[12..].chunks_exact(4).enumerate() {
        let value = f32::from_le_bytes(chunk.try_into().unwrap()) as f64;
        if !value.is_finite() { return Err(format!("Pro-Q 3 参数 {i} 不是有限数")); }
        values[i] = value;
    }
    let mut result = Parameters::new();
    let fields = ["Used", "Enabled", "Frequency", "Gain", "Dynamic Range",
        "Dynamics Enabled", "Threshold", "Q", "Shape", "Slope", "Stereo Placement", "Legacy Reserved 1", "Legacy Unknown"];
    for band in 0..24 {
        for (field, value) in fields.iter().zip(&values[band * 13..band * 13 + 13]) {
            result.insert(format!("Band {} {field}", band + 1), *value);
        }
    }
    let globals = ["Processing Mode", "Processing Resolution", "Gain Scale", "Output Level", "Output Pan",
        "Legacy Global 1", "Bypass", "Output Invert Phase", "Auto Gain", "Analyzer Show Pre-Processing",
        "Analyzer Show Post-Processing", "Analyzer External Spectrum", "Analyzer Range", "Analyzer Resolution",
        "Analyzer Speed", "Analyzer Tilt", "Legacy Global 2", "Analyzer Show Collisions", "Spectrum Grab",
        "Display Range", "Receive Midi", "Legacy Global 3"];
    for (field, value) in globals.iter().zip(&values[312..]) { result.insert((*field).into(), *value); }
    Ok((version, result))
}

struct Translation<'a> {
    device: &'a str,
    report: FabFilterImportReport,
    values: BTreeMap<String, f64>,
}
impl<'a> Translation<'a> {
    fn new(device: &'a str, product: &str, version: u32) -> Self {
        Self { device, report: FabFilterImportReport { product: product.into(), format_version: version,
            values: Vec::new(), warnings: vec!["这是参数翻译；滤波器、时间响应和算法使用 Verdant 实现，不保证与原插件声音相同。".into()],
            unsupported: Vec::new() }, values: BTreeMap::new() }
    }
    fn set(&mut self, id: &str, value: f64, source: &str) -> bool {
        let Some(spec) = dsp::spec_for(self.device, id) else {
            self.unsupported(source, "当前 Verdant 版本没有对应参数"); return false;
        };
        if !value.is_finite() || spec.check(value).is_err() || (spec.steps > 0 && value.fract() != 0.0) {
            self.unsupported(source, &format!("转换值 {value} 超出 {id} 的有效范围 {}…{}，未截断", spec.min, spec.max)); return false;
        }
        self.values.insert(id.to_string(), value);
        true
    }
    fn unsupported(&mut self, source: &str, detail: &str) { self.report.unsupported.push(format!("{source}：{detail}")); }
    fn warn(&mut self, source: &str, detail: &str) { self.report.warnings.push(format!("{source}：{detail}")); }
    fn defaults(&mut self, specs: &[Spec]) {
        for spec in specs { self.values.insert(spec.id.into(), spec.default); }
    }
    fn neutral(&mut self, p: &mut Parameters, key: &str, expected: f64, detail: &str) {
        if let Some(value) = p.remove(key) {
            if (value - expected).abs() > 1e-6 { self.unsupported(key, &format!("{value}；{detail}")); }
        }
    }
    fn finish(mut self, remaining: Parameters) -> FabFilterImportReport {
        let mut display = false;
        for (key, value) in remaining {
            if key.starts_with("Analyzer ") || matches!(key.as_str(), "Spectrum Grab" | "Display Range"
                | "Meter Scale" | "Show Side Chain" | "Knee Display Enabled" | "Show Input Level Meter"
                | "Lock Mix" | "Lock Auto Threshold" | "Midi State" | "Receive Midi") {
                display = true;
            } else { self.unsupported(&key, &format!("未识别的字段值 {value}")); }
        }
        if display { self.warn("界面/控制器偏好", "分析器、显示比例和 MIDI 学习等插件偏好未导入"); }
        self.report.values = self.values.into_iter().collect();
        self.report
    }
}

fn translate_eq(mut p: Parameters, device: &str, product: &str, version: u32, q4: bool) -> Result<FabFilterImportReport, String> {
    let mut t = Translation::new(device, product, version);
    for d in EqParams::default().descs() { t.values.insert(d.id, d.value); }
    let scale = take(&mut p, "Gain Scale", 1.0);
    if !(0.0..=2.0).contains(&scale) { return Err("FFP Gain Scale 超出 0…2".into()); }
    let phase = uint(take(&mut p, "Processing Mode", 0.0), "Processing Mode", 2)?;
    t.set("phase", phase as f64, "Processing Mode");
    let resolution = uint(take(&mut p, "Processing Resolution", 1.0), "Processing Resolution", 4)?;
    if phase == 2 { t.set("resolution", resolution as f64, "Processing Resolution"); }
    let auto_gain = flag(take(&mut p, "Auto Gain", 0.0), "Auto Gain")?;
    t.set("auto_gain", auto_gain as u8 as f64, "Auto Gain");
    for key in ["Output Level", "Output Pan", "Output Pan Mode", "Output Invert Phase", "Bypass", "Character",
        "Solo Gain", "Legacy Global 1", "Legacy Global 2", "Legacy Global 3"] {
        t.neutral(&mut p, key, 0.0, "尚无经过验证的完整映射；保留原生默认值");
    }
    for band in 1..=24 {
        let prefix = format!("Band {band} ");
        let used = used_band(&mut p, &prefix)?;
        let enabled = flag(take(&mut p, &(prefix.clone() + "Enabled"), 1.0), &(prefix.clone() + "Enabled"))?;
        t.set(&format!("b{band}_on"), 0.0, &prefix);
        let names: Vec<String> = p.keys().filter(|k| k.starts_with(&prefix)).cloned().collect();
        let mut fields = Parameters::new();
        for key in names { fields.insert(key[prefix.len()..].to_string(), p.remove(&key).unwrap()); }
        if !used { continue; }
        let mut core_ok = true;
        let frequency = 2.0_f64.powf(required(&mut fields, "Frequency")?);
        let q = 10.0_f64.powf((required(&mut fields, "Q")? - 0.5) / 0.312098175);
        let shape = uint(required(&mut fields, "Shape")?, &(prefix.clone() + "Shape"), if q4 {9} else {8})?;
        let native_shape = match shape { 8 => 9, 9 => 8, _ => shape };
        let gain = required(&mut fields, "Gain")? * scale;
        for (field, value) in [("freq",frequency),("gain",gain),("q",q),("shape",native_shape as f64)] {
            core_ok &= t.set(&format!("b{band}_{field}"), value, &(prefix.clone() + field));
        }
        let place = uint(take(&mut fields, "Stereo Placement", 2.0), &(prefix.clone() + "Stereo Placement"), 4)?;
        t.set(&format!("b{band}_place"), [1.0,2.0,0.0,3.0,4.0][place], &(prefix.clone() + "Stereo Placement"));
        if take(&mut fields, "Legacy Reserved 1", 1.0) != 1.0 { t.unsupported(&(prefix.clone()+"Legacy Reserved 1"), "旧版未知字段发生变化"); }
        let speakers = take(&mut fields, "Speakers", 1.0);
        if speakers != 1.0 { t.unsupported(&(prefix.clone()+"Speakers"), "环绕扬声器选择无法映射到立体声设备"); core_ok = false; }
        let slope = take(&mut fields, "Slope", if q4 {2.0} else {1.0});
        let slope_db = if q4 && slope.fract() == 0.0 && (0.0..=9.0).contains(&slope) {
            Some([0.0,6.0,12.0,18.0,24.0,30.0,36.0,48.0,72.0,96.0][slope as usize])
        } else if q4 && slope == 0.5 { Some(3.0) }
        else if !q4 && slope == 1.0 { Some(12.0) } else { None };
        if q4 && slope == 10.0 && matches!(shape,2|4) {
            t.set(&format!("b{band}_brickwall"),1.0,&(prefix.clone()+"Slope"));
        } else if let Some(db) = slope_db {
            if dsp::spec_for(device,&format!("b{band}_slope_continuous")).is_some() {
                t.set(&format!("b{band}_slope_continuous"),1.0,&(prefix.clone()+"Slope"));
                t.set(&format!("b{band}_slope_db"),db,&(prefix.clone()+"Slope"));
            } else if matches!(shape,2|4) {
                if let Some(index) = dsp::SLOPES.iter().position(|s| *s as f64 == db) {
                    t.set(&format!("b{band}_slope"),index as f64,&(prefix.clone()+"Slope"));
                } else { t.unsupported(&(prefix.clone()+"Slope"),&format!("{db} dB/oct 尚不能表示")); }
            } else if db != 12.0 && shape != 8 {
                t.unsupported(&(prefix.clone()+"Slope"),&format!("当前形状的 {db} dB/oct 尚不能表示"));
            }
        } else { t.unsupported(&(prefix.clone()+"Slope"), &format!("编码 {slope} 的物理映射尚未验证，未取整")); }
        let range = take(&mut fields, "Dynamic Range", 0.0);
        let dynamic = flag(take(&mut fields,"Dynamics Enabled",1.0),&(prefix.clone()+"Dynamics Enabled"))?;
        t.set(&format!("b{band}_dyn_range"),range*scale,&(prefix.clone()+"Dynamic Range"));
        if dynamic && range != 0.0 {
            t.unsupported(&(prefix.clone()+"Dynamics"),"已保存动态范围；阈值归一化、自动阈值和时间映射未验证，动态处理保持关闭");
        }
        let spectral = flag(take(&mut fields,"Spectral Enabled",0.0),&(prefix.clone()+"Spectral Enabled"))?;
        if spectral { t.unsupported(&(prefix.clone()+"Spectral Enabled"),"频谱动态需要先验证动态阈值/时间映射，保持关闭"); }
        t.set(&format!("b{band}_density"),take(&mut fields,"Spectral Density",50.0),&(prefix.clone()+"Spectral Density"));
        for key in ["Dynamics Auto","Threshold","Attack","Release","Side Chain Low Frequency","Side Chain High Frequency"] { fields.remove(key); }
        for key in ["External Side Chain","Side Chain Filtering","Side Chain Audition","Solo","Legacy Unknown"] {
            if take(&mut fields,key,0.0) != 0.0 { t.unsupported(&(prefix.clone()+key),"该预设字段尚无对应路由/监听映射"); }
        }
        for (key,value) in fields { t.unsupported(&(prefix.clone()+&key),&format!("未知字段 {value}")); }
        t.set(&format!("b{band}_on"),(enabled && core_ok) as u8 as f64,&prefix);
    }
    Ok(t.finish(p))
}

fn translate_comp(mut p: Parameters, device: &str, version: u32) -> Result<FabFilterImportReport, String> {
    let mut t = Translation::new(device,"Pro-C 3",version);
    t.defaults(dsp::COMP_SPECS);
    let style = uint(required(&mut p,"Style")?,"Style",13)?;
    t.set("style",[0.0,8.0,1.0,2.0,6.0,9.0,10.0,11.0,12.0,5.0,3.0,13.0,4.0,7.0][style],"Style");
    for (source,target) in [("Threshold","threshold"),("Knee","knee"),("Range","range")] {
        t.set(target,required(&mut p,source)?,source);
    }
    for (source,target) in [("Auto Threshold","auto_threshold"),("Auto Release","auto_release"),("Auto Gain","auto_gain"),
        ("Audition Side Chain","sc_listen")] {
        let value = flag(take(&mut p,source,0.0),source)? as u8 as f64; t.set(target,value,source);
    }
    t.set("mix",100.0*take(&mut p,"Mix",1.0),"Mix");
    for source in ["Ratio","Attack","Release","Stereo Link"] {
        let value = required(&mut p,source)?;
        t.unsupported(source,&format!("归一化值 {value} 到物理量的转换尚未验证；保留原生默认值"));
    }
    for source in ["Hold","Lookahead","Character","Character Drive","Character Routing","Side Chain Input","Side Chain Level",
        "Stereo Link Mode","Input Level","Output Level","Input Pan","Output Pan","Wet Gain","Wet Pan","Dry Pan","Bypass",
        "Audition Triggering","Host Trigger Offset"] {
        let default = if source=="Character Routing"{1.0}else{0.0};
        t.neutral(&mut p,source,default,"该增益/时间/路由映射尚未验证");
    }
    t.neutral(&mut p,"Dry Gain",-1.0,"独立干声增益尚无经过验证的映射");
    for source in ["Maximum Lookahead","Host Trigger Sync","Host Trigger Length","Stereo Link Center","Stereo Link Surrounds",
        "Stereo Link Tops","Stereo Link LFE"] {
        if let Some(v)=p.remove(source) { if !matches!(source,"Host Trigger Sync"|"Host Trigger Length"|"Maximum Lookahead") && v != if source.ends_with("Surrounds")||source.ends_with("Tops"){1.0}else{0.0} {
            t.unsupported(source,"环绕联动字段未映射");
        }}
    }
    let quality=uint(take(&mut p,"Oversampling",0.0),"Oversampling",5)?;
    if quality==0 { t.set("oversampling",0.0,"Oversampling"); }
    else { t.unsupported("Oversampling",&format!("编码 {quality} 的倍率尚未验证")); }
    let mut any=false;
    for band in 1..=6 {
        let prefix=format!("Side Chain EQ Band {band} ");
        let used=used_band(&mut p,&prefix)?;
        let enabled=flag(take(&mut p,&(prefix.clone()+"Enabled"),1.0),&(prefix.clone()+"Enabled"))?;
        let names:Vec<String>=p.keys().filter(|k|k.starts_with(&prefix)).cloned().collect();
        let mut f=Parameters::new();for key in names{f.insert(key[prefix.len()..].into(),p.remove(&key).unwrap());}
        t.set(&format!("sc{band}_on"),0.0,&prefix);
        if !used{continue;}
        let shape=uint(required(&mut f,"Shape")?,&(prefix.clone()+"Shape"),9)?;
        let mut ok=shape<=4;
        if !ok{t.unsupported(&(prefix.clone()+"Shape"),"该侧链形状尚不能表示");}
        let shape_native=[0.0,1.0,3.0,2.0,4.0].get(shape).copied().unwrap_or(0.0);
        for (name,value) in [("freq",2.0_f64.powf(required(&mut f,"Frequency")?)),
            ("gain",required(&mut f,"Gain")?),("q",10.0_f64.powf((required(&mut f,"Q")?-0.5)/0.312098175)),("shape",shape_native)] {
            ok &=t.set(&format!("sc{band}_{name}"),value,&(prefix.clone()+name));
        }
        if take(&mut f,"Slope",1.0)!=1.0 {t.unsupported(&(prefix.clone()+"Slope"),"侧链滤波器目前使用原生固定斜率");}
        if take(&mut f,"Stereo Placement",0.0)!=0.0 || take(&mut f,"Speakers",1.0)!=1.0 {
            t.unsupported(&prefix,"侧链声道选择无法表示");ok=false;
        }
        for (key,value)in f{t.unsupported(&(prefix.clone()+&key),&format!("未知字段 {value}"));}
        t.set(&format!("sc{band}_on"),(ok&&enabled)as u8 as f64,&prefix);any|=ok&&enabled;
    }
    t.set("sc_eq_on",any as u8 as f64,"Side Chain EQ");
    Ok(t.finish(p))
}

fn translate_reverb(mut p: Parameters,device:&str,version:u32)->Result<FabFilterImportReport,String>{
    let mut t=Translation::new(device,"Pro-R 2",version);t.defaults(dsp::REVERB_SPECS);
    for (source,target)in[("Mix","mix"),("Auto Gate","gate_time")] {t.set(target,required(&mut p,source)?,source);}
    for(source,target)in[("Freeze","freeze"),("Auto Gate Enabled","auto_gate")] {
        t.set(target,flag(take(&mut p,source,0.0),source)? as u8 as f64,source);
    }
    t.set("style",uint(required(&mut p,"Style")?,"Style",2)? as f64,"Style");
    t.set("decay_rate",100.0*2.0_f64.powf(take(&mut p,"Decay Rate",0.0)),"Decay Rate");
    for(source,target)in[("Distance","distance"),("Thickness","thickness")] {
        t.set(target,100.0*required(&mut p,source)?,source);
    }
    for source in["Space","Brightness","Character","Stereo Width"] {
        let v=required(&mut p,source)?;t.unsupported(source,&format!("归一化值 {v} 的房间/音色模型映射未验证"));
    }
    for source in["Predelay","Predelay Offset","Predelay Sync","Auto Gate Offset","Auto Gate Sync","Ducking","Input Level",
        "Output Level","Input Pan","Output Pan","Bypass"] {
        t.neutral(&mut p,source,0.0,"时间、音量或路由转换尚未验证");
    }
    for source in["Decay Rate Tilt","Distance Tilt","Brightness Tilt","Character Tilt","Thickness Tilt",
        "Stereo Width Tilt","Ducking Tilt","Mix Tilt","Predelay Tilt","Surround Cross Feed"] {
        t.neutral(&mut p,source,0.5,"环绕倾斜不能表示");
    }
    for(source,default)in[("LFE Level",0.0),("Center Level",0.8),("Top Level",0.8)]{
        t.neutral(&mut p,source,default,"环绕声道不能表示");
    }
    // The native legacy tone controls are neutralized when importing node EQ.
    for(id,value)in[("low_cut",20.0),("high_cut",20000.0),("post_low_gain",0.0),("post_high_gain",0.0),("post_mid_gain",0.0)]{t.set(id,value,"Post EQ");}
    for(kind,native)in[("Decay","decay"),("Post","post")]{
        for band in 1..=6{
            let prefix=format!("{kind} EQ Band {band} ");
            let used=used_band(&mut p,&prefix)?;
            let enabled=flag(take(&mut p,&(prefix.clone()+"Enabled"),1.0),&(prefix.clone()+"Enabled"))?;
            let names:Vec<String>=p.keys().filter(|k|k.starts_with(&prefix)).cloned().collect();
            let mut f=Parameters::new();for key in names{f.insert(key[prefix.len()..].into(),p.remove(&key).unwrap());}
            if !used{continue;}
            let shape=uint(required(&mut f,"Shape")?,&(prefix.clone()+"Shape"),9)?;
            let mapped=if kind=="Decay"{match shape{0=>Some(0.0),1=>Some(1.0),3=>Some(2.0),_=>None}}
                else{[Some(0.0),Some(1.0),Some(3.0),Some(2.0),Some(4.0)].get(shape).copied().flatten()};
            let mut ok=mapped.is_some();
            if !ok{t.unsupported(&(prefix.clone()+"Shape"),"当前节点类型无法表示");}
            for(name,value)in[("freq",2.0_f64.powf(required(&mut f,"Frequency")?)),
                ("q",10.0_f64.powf((required(&mut f,"Q")?-0.5)/0.312098175)),("shape",mapped.unwrap_or(0.0))]{
                ok &=t.set(&format!("{native}{band}_{name}"),value,&(prefix.clone()+name));
            }
            let(field,value)=if kind=="Decay"{("rate",100.0*2.0_f64.powf(required(&mut f,"Rate")?))}else{("gain",required(&mut f,"Gain")?)};
            ok &=t.set(&format!("{native}{band}_{field}"),value,&(prefix.clone()+field));
            if take(&mut f,"Speakers",1.0)!=1.0||take(&mut f,"Stereo Placement",2.0)!=2.0{
                t.unsupported(&prefix,"环绕/单声道节点路由无法表示");ok=false;
            }
            if let Some(slope)=f.remove("Slope"){if slope!=1.0{t.unsupported(&(prefix.clone()+"Slope"),"输出 EQ 的斜率映射未验证");}}
            for(key,value)in f{t.unsupported(&(prefix.clone()+&key),&format!("未知字段 {value}"));}
            t.set(&format!("{native}{band}_on"),(ok&&enabled)as u8 as f64,&prefix);
        }
    }
    Ok(t.finish(p))
}


/// Pro-MB format v3: six positional 22-float bands and eighteen globals.
/// The factory FPMb files use the same physical parameter ordering measured
/// through the AU host in logicxkit's FabF_FPMb.json (2026-09-23).
fn translate_multiband(bytes:&[u8],device:&str)->Result<FabFilterImportReport,String>{
    require_device(device,dsp::MULTIBAND_ID,"Pro-MB")?;
    if bytes.len()!=612{return Err("Pro-MB FFP 长度错误：应为 150 个 float32 参数".into());}
    let version=u32::from_le_bytes(bytes[4..8].try_into().unwrap());require_version(version,3,"Pro-MB")?;
    if u32::from_le_bytes(bytes[8..12].try_into().unwrap())!=150{return Err("Pro-MB FFP 参数数量无效".into());}
    let mut v=[0.0;150];
    for(i,bytes)in bytes[12..].chunks_exact(4).enumerate(){
        v[i]=f32::from_le_bytes(bytes.try_into().unwrap())as f64;
        if !v[i].is_finite(){return Err(format!("Pro-MB 参数 {i} 不是有限数"));}
    }
    let mut t=Translation::new(device,"Pro-MB",version);t.defaults(dsp::MULTIBAND_SPECS);
    t.set("layout",1.0,"Pro-MB 独立频段");t.set("band_count",6.0,"Pro-MB 频段容量");
    t.set("mix",v[133]*100.0,"Mix");
    if v[136]==-1.0{t.unsupported("Output Level","原厂静音哨兵 -1 无法表示为有限 dB");}
    else{t.set("output",v[136]*36.0,"Output Level");}
    if v[134]!=0.0{t.unsupported("Input Level",&format!("{} dB；原生多段目前没有独立输入增益",v[134]*36.0));}
    let lookahead=flag(v[141],"Lookahead Enabled")?;
    t.warn("阈值/比率","使用原作者通过 AU 测得的 41 个标定点；点间线性插值属于近似转换。");
    for band in 0..6{
        let row=&v[band*22..band*22+22];let n=band+1;let label=format!("Band {n}");
        t.set(&format!("b{n}.enabled"),0.0,&label);
        let state=uint(row[0],&format!("{label} State"),2)?;
        if state==2{continue;}
        let mut ok=true;
        for(field,value)in[("low",2.0_f64.powf(row[1])),("high",2.0_f64.powf(row[3])),
            ("mode",uint(row[5],&format!("{label} Mode"),1)?as f64),
            ("range",row[7]),("knee",row[11]),("gain",row[13]),("lookahead",if lookahead{row[12]}else{0.0})]{
            ok &=t.set(&format!("b{n}.{field}"),value,&format!("{label} {field}"));
        }
        let threshold=measured_threshold(row[6])?;
        let ratio=measured_ratio(row[8])?;
        ok &=t.set(&format!("b{n}.threshold"),threshold,&format!("{label} Threshold"));
        ok &=t.set(&format!("b{n}.ratio"),ratio,&format!("{label} Ratio"));
        t.unsupported(&format!("{label} Attack/Release"),&format!("原厂使用依赖节目内容的 {}% / {}%；没有可靠毫秒映射，使用原生默认时间",row[9],row[10]));
        let unknown=[2usize,4,14,15,16,17,18,19,20,21].iter().map(|&i|format!("{i}={}",row[i])).collect::<Vec<_>>().join(", ");
        t.unsupported(&format!("{label} 斜率/路由等剩余字段"),&format!("尚未验证的带内参数位置：{unknown}"));
        t.set(&format!("b{n}.enabled"),(state==1&&ok)as u8 as f64,&label);
    }
    let globals=[132usize,135,137,138,139,140,142,143,144,145,146,147,148,149].iter()
        .map(|&i|format!("{i}={}",v[i])).collect::<Vec<_>>().join(", ");
    t.unsupported("Pro-MB 其余全局设置",&format!("相位、监听/控制偏好等字段尚未验证：{globals}"));
    Ok(t.finish(Parameters::new()))
}

fn measured_threshold(raw:f64)->Result<f64,String>{
    if !(0.0..=1.0).contains(&raw){return Err("Pro-MB Threshold 归一化值超出 0…1".into());}
    // These three straight segments pass through every measured 0.025 step.
    Ok(if raw<=0.1{-90.0+180.0*raw}else if raw<=0.2{-72.0+240.0*(raw-0.1)}else{-48.0+60.0*(raw-0.2)})
}
fn measured_ratio(raw:f64)->Result<f64,String>{
    const TABLE:[f64;41]=[1.0,1.02,1.05,1.08,1.1,1.14,1.17,1.21,1.25,1.31,1.38,1.44,1.5,1.62,1.75,1.88,
        2.0,2.19,2.38,2.56,2.75,3.06,3.38,3.69,4.0,4.5,5.0,5.5,6.0,6.5,7.0,7.5,8.0,8.5,9.0,9.5,10.0,14.5,24.4,54.78,100.0];
    if !(0.0..=1.0).contains(&raw){return Err("Pro-MB Ratio 归一化值超出 0…1".into());}
    let position=raw*40.0;let i=(position.floor()as usize).min(39);let f=position-i as f64;
    Ok(TABLE[i]+f*(TABLE[i+1]-TABLE[i]))
}

fn identify_binary(bytes:&[u8],device:&str)->Result<FabFilterImportReport,String>{
    if bytes.len()<8{return Err("FFP 二进制头被截断".into());}
    let signature=&bytes[..4];
    let(product,target)=match signature{
        b"FPGr"=>("Pro-G",dsp::GATE_ID),b"FPSr"=>("Pro-DS",dsp::DEESSER_ID),b"FL2p"=>("Pro-L 2",dsp::LIMITER_ID),
        b"FPRr"=>("Pro-R",dsp::REVERB_ID),b"FS2a"=>("Saturn 2",dsp::SATURATION_ID),b"FSat"=>("Saturn",dsp::SATURATION_ID),
        b"FSim"=>("Simplon",dsp::SIMPLON_ID),b"FMic"=>("Micro",dsp::MICRO_ID),b"FV3l"=>("Volcano 3",dsp::FILTER_ID),
        b"F3Ts"=>("Timeless 3",dsp::DELAY_ID),b"FPMb"=>("Pro-MB",dsp::MULTIBAND_ID),
        b"FT3i"=>("Twin 3","verdant.builtin.synth"),b"FFPr"=>("One","verdant.builtin.one"),
        b"FC2p"=>("Pro-C 2",dsp::COMPRESSOR_ID),b"FQ2p"=>("Pro-Q 2",dsp::EQ_ID),
        _=>return Err("未识别的 FabFilter 预设签名；未尝试读取为原生状态".into()),
    };
    require_device(device,target,product)?;
    let version=u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    Ok(FabFilterImportReport{product:product.into(),format_version:version,values:Vec::new(),warnings:Vec::new(),
        unsupported:vec![format!("已识别 {product} 二进制预设，格式版本 {version}；此产品的参数布局/归一化尚未验证，没有应用任何参数")]})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn q4(extra:&str)->Vec<u8>{
        let mut text=String::from("[Preset]\nSignature=FQ4p\nVersion=4\n[Parameters]\n");
        for band in 1..=24 {
            text+=&format!("Band {band} Used={}\n",if band==1{1}else{0});
            if band==1{text+="Band 1 Enabled=1\nBand 1 Frequency=10\nBand 1 Q=0.5\nBand 1 Gain=-6\nBand 1 Shape=0\nBand 1 Slope=2\n";}
        }
        text+=extra;text.into_bytes()
    }
    fn value(report:&FabFilterImportReport,id:&str)->f64{report.values.iter().find(|(key,_)|key==id).unwrap().1}
    #[test]
    fn q4_translates_full_static_state_and_keeps_legacy_shape_indexes(){
        let bytes=q4("Band 1 Stereo Placement=3\nGain Scale=0.5\nProcessing Mode=1\n");
        let report=inspect_fabfilter_preset(&bytes,dsp::EQ_ID).unwrap();
        assert!(report.is_complete(),"{:?}",report.unsupported);
        assert_eq!(value(&report,"b1_freq"),1024.0);assert_eq!(value(&report,"b1_gain"),-3.0);
        assert_eq!(value(&report,"b1_place"),3.0);assert_eq!(value(&report,"b2_on"),0.0);
        assert_eq!(value(&report,"phase"),1.0);
        let flat=String::from_utf8(q4("")).unwrap().replace("Band 1 Shape=0","Band 1 Shape=8");
        assert_eq!(value(&inspect_fabfilter_preset(flat.as_bytes(),dsp::EQ_ID).unwrap(),"b1_shape"),9.0);
        let all=flat.replace("Band 1 Shape=8","Band 1 Shape=9");
        assert_eq!(value(&inspect_fabfilter_preset(all.as_bytes(),dsp::EQ_ID).unwrap(),"b1_shape"),8.0);
    }
    #[test]
    fn dynamic_and_unknown_features_are_visible_and_strict_import_is_atomic(){
        let bytes=q4("Band 1 Dynamic Range=-4\nBand 1 Dynamics Enabled=1\nCharacter=2\nFuture Audio Field=1\n");
        let report=inspect_fabfilter_preset(&bytes,dsp::EQ_ID).unwrap();
        assert!(report.unsupported.iter().any(|s|s.contains("Dynamics")));
        assert!(report.unsupported.iter().any(|s|s.contains("Character")));
        assert!(report.unsupported.iter().any(|s|s.contains("Future Audio Field")));
        assert_eq!(value(&report,"b1_dynamic"),0.0);
        assert!(import_fabfilter_preset(&bytes,dsp::EQ_ID).is_err());
    }
    #[test]
    fn malformed_text_is_rejected_before_returning_any_values(){
        for bad in ["Band 1 Gain=3\n","Output Level=NaN\n","Output Level=inf\n","[Parameters]\nOutput Level=0\n"]{
            assert!(inspect_fabfilter_preset(&q4(bad),dsp::EQ_ID).is_err(),"{bad}");
        }
        assert!(inspect_fabfilter_preset(&q4(""),dsp::GATE_ID).is_err());
        assert!(inspect_fabfilter_preset(&vec![0;MAX_BYTES+1],dsp::EQ_ID).is_err());
        let future=String::from_utf8(q4("")).unwrap().replace("Version=4","Version=99");
        assert!(inspect_fabfilter_preset(future.as_bytes(),dsp::EQ_ID).is_err());
    }
    #[test]
    fn binary_q3_validates_exact_count_finiteness_and_little_endian_fields(){
        let mut v=[0.0f32;334];
        for band in 0..24{let i=band*13;v[i+1]=1.0;v[i+2]=10.0;v[i+7]=0.5;v[i+9]=1.0;v[i+10]=2.0;v[i+11]=1.0;}
        v[0]=1.0;v[3]=3.0;v[314]=1.0;
        let mut bytes=b"FQ3p".to_vec();bytes.extend_from_slice(&4u32.to_le_bytes());bytes.extend_from_slice(&334u32.to_le_bytes());
        for n in v{bytes.extend_from_slice(&n.to_le_bytes());}
        let report=inspect_fabfilter_preset(&bytes,dsp::EQ_ID).unwrap();
        assert!(report.is_complete(),"{:?}",report.unsupported);assert_eq!(value(&report,"b1_gain"),3.0);
        assert_eq!(value(&report,"b1_freq"),1024.0);
        bytes[12..16].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(inspect_fabfilter_preset(&bytes,dsp::EQ_ID).is_err());
        bytes.truncate(24);assert!(inspect_fabfilter_preset(&bytes,dsp::EQ_ID).is_err());
    }
    #[test]
    fn other_binary_families_identify_without_inventing_parameter_maps(){
        let mut bytes=b"FL2p".to_vec();bytes.extend_from_slice(&2u32.to_le_bytes());
        let report=inspect_fabfilter_preset(&bytes,dsp::LIMITER_ID).unwrap();
        assert_eq!(report.product,"Pro-L 2");assert_eq!(report.format_version,2);
        assert!(report.values.is_empty());assert!(!report.unsupported.is_empty());
        assert!(import_fabfilter_preset(&bytes,dsp::LIMITER_ID).is_err());
    }

    fn text_family(signature:&str,version:u32,globals:&str,prefixes:&[&str])->Vec<u8>{
        let mut s=format!("[Preset]\nSignature={signature}\nVersion={version}\n[Parameters]\n{globals}");
        for prefix in prefixes{for n in 1..=6{s+=&format!("{prefix} EQ Band {n} Used=0\n");}}s.into_bytes()
    }
    #[test]
    fn c3_and_r2_preview_known_fields_and_mark_unknown_physical_transforms(){
        let c=text_family("FC3p",2,"Style=13\nThreshold=-20\nKnee=6\nRange=40\nRatio=0.56\nAttack=0.1\nRelease=0.3\nStereo Link=0.4\nMix=0.25\n",&["Side Chain"]);
        let report=inspect_fabfilter_preset(&c,dsp::COMPRESSOR_ID).unwrap();
        assert_eq!(value(&report,"style"),7.0);assert_eq!(value(&report,"threshold"),-20.0);
        assert_eq!(value(&report,"mix"),25.0);assert!(!report.is_complete());
        assert!(report.unsupported.iter().any(|s|s.starts_with("Ratio")));
        let r=text_family("FR2p",6,"Mix=30\nAuto Gate=250\nStyle=2\nDecay Rate=1\nDistance=0.25\nThickness=-0.5\nSpace=0.5\nBrightness=0.5\nCharacter=0.3\nStereo Width=0.7\n",&["Decay","Post"]);
        let report=inspect_fabfilter_preset(&r,dsp::REVERB_ID).unwrap();
        assert_eq!(value(&report,"mix"),30.0);assert_eq!(value(&report,"decay_rate"),200.0);
        assert_eq!(value(&report,"distance"),25.0);assert_eq!(value(&report,"thickness"),-50.0);
        assert!(report.unsupported.iter().any(|s|s.starts_with("Space")));
        assert!(report.values.iter().all(|(id,v)|dsp::spec_for(dsp::REVERB_ID,id).unwrap().check(*v).is_ok()));
    }
    #[test]
    fn unrepresentable_band_is_disabled_and_never_clamped(){
        let bad=String::from_utf8(q4("")).unwrap().replace("Band 1 Frequency=10","Band 1 Frequency=100");
        let report=inspect_fabfilter_preset(bad.as_bytes(),dsp::EQ_ID).unwrap();
        assert_eq!(value(&report,"b1_on"),0.0);
        assert!(!report.unsupported.is_empty());
        assert!(report.values.iter().all(|(id,v)|dsp::spec_for(dsp::EQ_ID,id).unwrap().check(*v).is_ok()));
    }
    /// Optional local probe; commercial factory files are never checked in.
    #[test]
    #[ignore]
    fn external_factory_corpus_probe(){
        let root=std::env::var_os("VERDANT_FFP_REFERENCE_ROOT").expect("set VERDANT_FFP_REFERENCE_ROOT");
        let mut pending=vec![std::path::PathBuf::from(root)];
        let mut files=Vec::new();
        while let Some(path)=pending.pop(){
            for entry in std::fs::read_dir(path).unwrap(){
                let path=entry.unwrap().path();if path.is_dir(){pending.push(path);}
                else if path.extension().and_then(|s|s.to_str())==Some("ffp"){files.push(path);}
            }
        }
        let mut counts=BTreeMap::<String,[usize;4]>::new();
        for file in files{
            let bytes=std::fs::read(&file).unwrap();
            let text=std::str::from_utf8(&bytes).unwrap_or("");
            let sig=if text.contains("Signature=FQ4p"){"FQ4p"}else if text.contains("Signature=FC3p"){"FC3p"}
                else if text.contains("Signature=FR2p"){"FR2p"}else{std::str::from_utf8(&bytes[..4]).unwrap_or("?")};
            let device=match sig{
                "FQ4p"|"FQ3p"=>dsp::EQ_ID,"FC3p"|"FC2p"=>dsp::COMPRESSOR_ID,"FR2p"|"FPRr"=>dsp::REVERB_ID,
                "FPGr"=>dsp::GATE_ID,"FPSr"=>dsp::DEESSER_ID,"FL2p"=>dsp::LIMITER_ID,"FS2a"|"FSat"=>dsp::SATURATION_ID,
                "FSim"=>dsp::SIMPLON_ID,"FMic"=>dsp::MICRO_ID,"FV3l"=>dsp::FILTER_ID,"F3Ts"=>dsp::DELAY_ID,
                "FPMb"=>dsp::MULTIBAND_ID,"FT3i"=>"verdant.builtin.synth","FFPr"=>"verdant.builtin.one",_=>"unknown"};
            let row=counts.entry(sig.into()).or_default();row[0]+=1;
            match inspect_fabfilter_preset(&bytes,device){
                Ok(report)=>{
                    if report.is_complete(){row[1]+=1;}else if !report.values.is_empty(){row[2]+=1;}else{row[3]+=1;}
                    assert!(report.values.iter().all(|(id,v)|dsp::spec_for(device,id).unwrap().check(*v).is_ok()));
                }
                Err(e)=>panic!("factory preset {} failed structural parsing: {e}",file.display()),
            }
        }
        eprintln!("FFP corpus: signature => [recognized, complete-parameter-translation, partial-with-diagnostics, identified-only] {counts:?}");
    }


    #[test]
    fn multiband_binary_maps_independent_bands_and_reports_program_dependent_times(){
        let mut v=[0.0f32;150];
        for b in 0..6{v[b*22]=2.0;}
        v[0]=1.0;v[1]=100.0f32.log2();v[3]=1000.0f32.log2();v[6]=0.6;v[7]=-6.0;v[8]=0.6;
        v[9]=20.0;v[10]=30.0;v[11]=6.0;v[12]=2.0;v[13]=1.0;v[133]=0.75;v[141]=1.0;
        let mut bytes=b"FPMb".to_vec();bytes.extend_from_slice(&3u32.to_le_bytes());bytes.extend_from_slice(&150u32.to_le_bytes());
        for n in v{bytes.extend_from_slice(&n.to_le_bytes());}
        let report=inspect_fabfilter_preset(&bytes,dsp::MULTIBAND_ID).unwrap();
        assert_eq!(value(&report,"layout"),1.0);assert_eq!(value(&report,"b1.enabled"),1.0);
        assert!((value(&report,"b1.low")-100.0).abs()<0.001);
        assert!((value(&report,"b1.threshold")+24.0).abs()<0.001);
        assert!((value(&report,"b1.ratio")-4.0).abs()<0.001);
        assert_eq!(value(&report,"b1.lookahead"),2.0);assert_eq!(value(&report,"mix"),75.0);
        assert_eq!(value(&report,"b2.enabled"),0.0);
        assert!(report.unsupported.iter().any(|x|x.contains("Attack/Release")));
        assert!(import_fabfilter_preset(&bytes,dsp::MULTIBAND_ID).is_err());
        bytes[8..12].copy_from_slice(&149u32.to_le_bytes());
        assert!(inspect_fabfilter_preset(&bytes,dsp::MULTIBAND_ID).is_err());
    }

}
