//! Report the native catalog and parameter/latency contract for release review.
//! cargo run -p verdant-plughost --example native_suite_inventory -- [--parameters]
use verdant_plugins::builtin_dsp as dsp;
fn main() {
    let include_parameters=std::env::args().skip(1).any(|arg|arg=="--parameters");
    let mut devices=Vec::new();
    for id in dsp::EFFECT_IDS {
        let mut instance=verdant_plughost::formats::builtin::effect(id,48000.).expect("catalog effect");
        let parameters=instance.params().expect("native descriptors");
        let automated=instance.automation_params().expect("native automation");
        let control_only=parameters.iter().filter(|p|!automated.iter().any(|a|a.id==p.id)).map(|p|p.id.clone()).collect::<Vec<_>>();
        let mut device=serde_json::json!({"id":id,"kind":"effect","parameters":parameters.len(),
            "base_parameters":parameters.len()-dsp::MOD_SPECS.len(),"modulation_parameters":dsp::MOD_SPECS.len(),
            "default_latency_samples":instance.latency(),"control_only":control_only});
        if include_parameters {device["descriptors"]=serde_json::to_value(parameters).unwrap();}
        devices.push(device);
    }
    for id in [dsp::SYNTH_ID,dsp::ONE_SYNTH_ID] {
        let parameters=dsp::instrument_specs(id);
        let mut device=serde_json::json!({"id":id,"kind":"instrument","parameters":parameters.len(),
            "base_parameters":parameters.len()-dsp::MOD_SPECS.len(),"modulation_parameters":dsp::MOD_SPECS.len(),
            "control_only":parameters.iter().filter(|p|!p.automatable).map(|p|p.id).collect::<Vec<_>>()});
        if include_parameters {
            let values=parameters.iter().map(|p|p.default).collect::<Vec<_>>();
            device["descriptors"]=serde_json::to_value(dsp::table_descs(&parameters,&values)).unwrap();
        }
        devices.push(device);
    }
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({
        "schema":"verdant-native-suite-inventory/1","sample_rate":48000,
        "control_protocol":verdant_plugins::proto::PROTOCOL_VERSION,"devices":devices})).unwrap());
}
