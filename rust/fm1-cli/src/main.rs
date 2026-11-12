// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::planner::{self, Phrase, Seed, Settings};
use fm1_formats::{crc32, jam::JamPack};
use serde_json::json;
use std::{env, fs, process};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("layouts") => println!("{}",json!({
            "parameters":fm1_types::PARAMETER_COUNT,
            "pattern_bytes":std::mem::size_of::<fm1_types::PatternBank>(),
            "command_payload_bytes":std::mem::size_of::<[fm1_types::CommandWord;1024]>(),
            "phrase_bytes":std::mem::size_of::<Phrase>(),
            "phrase_pair_bytes":std::mem::size_of::<[Phrase;2]>(),
            "acid_line_bytes":std::mem::size_of::<fm1_types::AcidLine>(),
            "pluck_line_bytes":std::mem::size_of::<fm1_types::PluckLine>(),
            "progression_bytes":std::mem::size_of::<fm1_types::Progression>(),
            "chord_kind_bytes":std::mem::size_of::<fm1_types::ChordKinds>(),
            "factory_preset_bytes":std::mem::size_of::<fm1_engine::factory::FactoryPreset>(),
            "transport_bytes":std::mem::size_of::<fm1_engine::transport::Transport>(),
            "melodic_harmony_bytes":std::mem::size_of::<fm1_engine::melodic::Harmony>(),
            "acid_state_bytes":std::mem::size_of::<fm1_engine::melodic::Acid>(),
            "pluck_state_bytes":std::mem::size_of::<fm1_engine::melodic::Pluck>(),
            "bass_state_bytes":std::mem::size_of::<fm1_engine::melodic::Bass>(),
            "master_scalar_bytes":std::mem::size_of::<fm1_engine::effects::master::State>(),
            "render_frames":fm1_types::RENDER_FRAMES,"dma_half_frames":fm1_types::DMA_HALF_FRAMES
        })),
        Some("params") => {
            let values:Vec<_>=fm1_types::PARAMETERS.iter().enumerate().map(|(i,p)|json!({"i":i,"n":p.name,"min":p.min,"max":p.max,"def":p.default})).collect();
            println!("{}",json!(values));
        }
        Some("validate") | Some("plan") => {
            let path=args.get(1).ok_or("usage: fm1-rs validate|plan PACK [key minor bars section seed movement form bass]")?;
            let bytes=fs::read(path).map_err(|e|format!("{path}: {e}"))?;
            let pack=JamPack::parse(&bytes).map_err(|e|format!("invalid pack: {e:?}"))?;
            if args[0]=="validate" {
                println!("{}",json!({"name":String::from_utf8_lossy(pack.name()),"version":pack.version(),"bytes":bytes.len(),"nodes":pack.node_count(),"crc32":format!("{:08x}",crc32(&bytes))}));
            } else {
                let number=|index:usize,default:u32|->Result<u32,String> {
                    args.get(index).map(|s|s.parse::<u32>().map_err(|e|e.to_string())).unwrap_or(Ok(default))
                };
                let byte=|index,default|->Result<u8,String> {u8::try_from(number(index,default)?).map_err(|e|e.to_string())};
                let settings=Settings{key:byte(2,0)?,minor:byte(3,0)?,bars:byte(4,16)?,movement:byte(7,3)?,form:byte(8,0)?,bass:byte(9,0)?,..Settings::default()};
                let section=number(5,0)?;
                let mut seed=Seed::new(number(6,4)?);
                let mut first=Phrase::EMPTY;let mut second=Phrase::EMPTY;
                planner::plan_into(pack,&settings,section,None,&mut first,&mut seed).map_err(|e|format!("plan: {e:?}"))?;
                planner::plan_into(pack,&settings,section.wrapping_add(1),Some(&first),&mut second,&mut seed).map_err(|e|format!("plan: {e:?}"))?;
                let beats:Vec<_>=first.active_beats().iter().map(|b|json!([b.notes[0],b.notes[1],b.notes[2],b.notes[3],b.node,b.passing])).collect();
                let mut bass=Vec::new();
                for bar in 0..first.bars as usize {for step in 0..16 {
                    let event=planner::bass_event(pack,&settings,&first,&second,bar,step).map_err(|e|format!("bass: {e:?}"))?;
                    bass.push(json!([event.note,event.velocity,event.gate]));
                }}
                println!("{}",json!({"bars":first.bars,"role":first.role,"motif":first.motif,"section":first.section,"seed":seed.state(),"beats":beats,"bass":bass}));
            }
        }
        _ => return Err("usage: fm1-rs layouts | params | validate PACK | plan PACK [key minor bars section seed movement form bass]".into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(2);
    }
}
