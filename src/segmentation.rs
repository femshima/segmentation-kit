use std::error::Error;

use julius::{sentence_align::SentenceAlignWithType, JConf, Recog};

pub fn align_audio_input(
    model_path: &str,
    dfa_path: &str,
    dict_path: &str,
    mut audio_input: Box<dyn Iterator<Item = i16>>,
) -> Result<Vec<(i32, i32, String)>, Box<dyn Error>> {
    let config = format!(
        "
-h {}
-dfa {}
-v {}
-palign
-input file
",
        model_path, dfa_path, dict_path
    );

    let mut result = None;
    {
        let jconf = JConf::from_string(&config)?;
        let mut recog = Recog::from_jconf(jconf)?;
        recog.add_callback(julius::CallbackType::Result, |recog| result = cb(recog));
        recog.adin_init()?;
        recog.custom_adin(|n| {
            let mut data = Vec::new();
            while let Some(a) = audio_input.next() {
                if data.len() > n {
                    break;
                }
                data.push(a);
            }
            if data.is_empty() {
                None
            } else {
                Some(data)
            }
        });
        // recog.open_stream(Some(wav_path))?;
        recog.recognize_stream()?;
        // recog.close_stream()?;
    }

    Ok(result.ok_or(anyhow::anyhow!("Failed to align"))?)
}

fn cb(recog: &mut Recog) -> Option<Vec<(i32, i32, String)>> {
    let Some(r) = recog.get_processes().next() else{return None};
    if !r.is_live() {
        return None;
    }

    let result = r.result();
    match result.status() {
        julius::recog_process::ResultStatus::RejectPower => {
            println!("<input rejected by power>")
        }
        julius::recog_process::ResultStatus::Terminate => {
            println!("<input teminated by request>")
        }
        julius::recog_process::ResultStatus::OnlySilence => {
            println!("<input rejected by decoder (silence input result)>")
        }
        julius::recog_process::ResultStatus::RejectGmm => {
            println!("<input rejected by GMM>")
        }
        julius::recog_process::ResultStatus::RejectShort => {
            println!("<input rejected by short input>")
        }
        julius::recog_process::ResultStatus::RejectLong => {
            println!("<input rejected by long input>")
        }
        julius::recog_process::ResultStatus::Fail => println!("<search failed>"),

        _ => (),
    }
    let s = &result.get_sent()[0];
    let Some(a)=s.get_align().next() else { return None};
    match a.t() {
        SentenceAlignWithType::Phoneme(phoneme_frame) => Some(
            phoneme_frame
                .frame_iter()
                .map(|phoneme| {
                    (
                        phoneme.begin_frame,
                        phoneme.end_frame,
                        phoneme.ph.name().unwrap(),
                    )
                })
                .collect(),
        ),
        _ => None,
    }
}
