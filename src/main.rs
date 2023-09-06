mod audio_input;
mod julius_input;
mod segmentation;

use std::{error::Error, fs::File, io::Write, path::PathBuf};

use audio_input::read_audio_i16_16khz;
use clap::Parser;
use jpreprocess_jpcommon::{overwrapping_phonemes, utterance_to_phoneme_vec, Utterance};
use segmentation::align_audio_input;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    model: PathBuf,
    #[arg(short, long)]
    dictionary: PathBuf,

    #[arg(short, long, default_value = "wav")]
    audio_directory: PathBuf,
    #[arg(short, long, default_value = "transcript_utf8.txt")]
    transcript: PathBuf,

    /// offset for result in senconds: 25ms / 2 = 12.5 ms = 0.0125 s
    #[arg(long, default_value = "0.0125")]
    offset_align: f32,

    #[arg(short, long, default_value = "lab")]
    output: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let jpre = jpreprocess::JPreprocess::from_config(jpreprocess::JPreprocessConfig {
        dictionary: jpreprocess::SystemDictionaryConfig::File(cli.dictionary.to_owned()),
        user_dictionary: None,
    })?;

    let transcript = std::fs::read_to_string(&cli.transcript)?;
    for line in transcript.split("\n") {
        let Some((id,text))=line.split_once(":")else{
            eprintln!("No id found in `{}`",line);
            continue;
        };
        let (script, ruby) = if let Some((script, ruby)) = text.split_once(",") {
            (script, Some(ruby))
        } else {
            (text, None)
        };

        let mut njd = jpre.text_to_njd(script)?;
        njd.preprocess();

        let ref_ruby = njd.nodes.iter().fold(String::new(), |acc, node| {
            format!("{}{}", acc, node.get_pron().to_string())
        });

        if ruby.is_some() && ruby.unwrap() == ref_ruby {
            eprintln!("WARN: Pronunciation mismatch.");
            eprintln!("    Original : {}", ruby.unwrap());
            eprintln!("    Generated: {}", ref_ruby);
        }

        let utterance = Utterance::from(njd.nodes.as_slice());
        let phoneme_vec = utterance_to_phoneme_vec(&utterance);

        let sentence = phoneme_vec[1..phoneme_vec.len() - 1].iter().fold(
            String::new(),
            |acc, (phoneme, _label)| {
                format!(
                    "{} {}",
                    acc,
                    phoneme
                        .replace("cl", "q")
                        .replace("pau", "sp")
                        .to_ascii_lowercase()
                )
            },
        );
        let words = vec![" silB", &sentence, " silE"];

        let dfa_path = cli.output.join(id).with_extension("dfa");
        let dfa = julius_input::dfa(words.len());
        File::create(&dfa_path)?.write_all(dfa.as_bytes())?;
        let dict_path = cli.output.join(id).with_extension("dict");
        let dict = julius_input::dict(&words);
        File::create(&dict_path)?.write_all(dict.as_bytes())?;

        let wav_path = cli.audio_directory.join(id).with_extension("wav");
        let audio_input = read_audio_i16_16khz(&wav_path);

        let aligned = align_audio_input(
            cli.model.to_str().unwrap(),
            dfa_path.to_str().unwrap(),
            dict_path.to_str().unwrap(),
            Box::new(audio_input),
        )?;

        let labels: Vec<String> = overwrapping_phonemes(phoneme_vec)
            .iter()
            .zip(aligned.iter())
            .enumerate()
            .map(|(i, (fullcontext, (begin_frame, end_frame, _)))| {
                // 1E-7 second
                let begin_time = begin_frame * 100_000
                    + if i == 0 {
                        0
                    } else {
                        (cli.offset_align * 1e7) as i32
                    };
                let end_time = (end_frame + 1) * 100_000 + (cli.offset_align * 1e7) as i32;
                format!("{} {} {}", begin_time, end_time, fullcontext)
            })
            .collect();

        let output_path = cli.output.join(id).with_extension("lab");
        File::create(&output_path)?.write_all(labels.join("\n").as_bytes())?;

        if id.ends_with("0025") {
            break;
        }
    }

    Ok(())
}
