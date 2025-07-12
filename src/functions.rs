use std::fs;
use std::process::exit;
use std::env;
use std::path::Path;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

// serde stuffs
fn str_to_f32<'de, D>(deserializer: D) -> Result<f32, D::Error> where D: serde::Deserializer<'de>,{
    let s = String::deserialize(deserializer)?;
    s.parse::<f32>().map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
pub struct MusicLoudness {
    #[serde(deserialize_with = "str_to_f32")]
    pub input_i: f32,
    #[serde(deserialize_with = "str_to_f32")]
    pub input_tp: f32,
    #[serde(deserialize_with = "str_to_f32")]
    pub input_lra: f32,
    #[serde(deserialize_with = "str_to_f32")]
    pub input_thresh: f32,
    #[serde(deserialize_with = "str_to_f32")]
    pub output_i: f32,
    #[serde(deserialize_with = "str_to_f32")]
    pub output_tp: f32,
    #[serde(deserialize_with = "str_to_f32")]
    pub output_lra: f32,
    #[serde(deserialize_with = "str_to_f32")]
    pub output_thresh: f32,
    pub normalization_type: String,
    #[serde(deserialize_with = "str_to_f32")]
    pub target_offset: f32,
}

impl MusicLoudness {
    pub fn show_loudness(&self){
        println!("Input I: {}", self.input_i);
        println!("Input TP: {}", self.input_tp);
        println!("Input LRA: {}", self.input_lra);
        println!("Input Threshold: {}", self.input_thresh);
        println!("Output I: {}", self.output_i);
        println!("Output TP: {}", self.output_tp);
        println!("Output LRA: {}", self.output_lra);
        println!("Output Threshold: {}", self.output_thresh);
        println!("Normalization Type: {}", self.normalization_type);
        println!("Target Offset: {}", self.target_offset);
    }
}

// file only
// path not exist: err 11
// path is dir in file process (file in dir process): 12
// Analysis result not exist: err 13
// ffmpeg normalization failed: err 14
pub fn ffmpeg_process(path: &str, ffmpeg_path: &str, limitation: f32) -> Result<String, Box<dyn std::error::Error>>{
    let path = Path::new(path); // convert to Path type
    println!("Current path: {}", env::current_dir().unwrap().display());
    // if path is not a file, return error
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                println!("Input is not a file: {}", path.display());
                exit(12);
            }
        }
        Err(_) => {
            println!("File not found: {}", path.display());
            exit(11);
        }
    }
    let file_name = path.file_name().unwrap().to_str().unwrap();
    // limit: -18LKFS
    // Analyze the audio file
    println!("Analyzing audio file: {}", path.display());
    let loudness_result = std::process::Command::new(ffmpeg_path).args([
        "-i", path.to_str().unwrap(),
        "-af", "loudnorm=I=-18:TP=-1.5:LRA=11:print_format=json",
        "-f", "null", "-"
    ])
    .output()?;
    // process output and store them in MusicLoudness struct
    let analysis_result_json = String::from_utf8_lossy(&loudness_result.stderr);  // ffmpeg outputs to stderr, weird fact
    let analysis_result_str = analysis_result_json.as_ref();
    // get {...} from json using regex
    let analysis_result_filter = Regex::new(r"(?s)(\{.*?})").unwrap();
    // capture_iter[0]: str before filtering, [1]: matched string
    let mut analysis_result: &str = "";
    if let Some(analysis_result_str_capture) = analysis_result_filter.captures(analysis_result_str) {
        if let Some(loudness_result_match) = analysis_result_str_capture.get(1) {
            // println!("Matched JSON: {}", loudness_result_match.as_str());
            analysis_result = loudness_result_match.as_str();
        }
    }
    // parse json to MusicLoudness struct
    let loudness: MusicLoudness = match serde_json::from_str(analysis_result) {
        Ok(loudness) => loudness,
        Err(e) => {
            println!("Failed to analyze: {}", e);
            return Err(Box::new(e));
        }
    };;

    // show loudness
    // println!("Loudness analysis result:");
    // loudness.show_loudness();

    // if <18, return, else, calculate target gain
    println!("Loudness analysis complete, loudness: {}", loudness.input_i);
    if loudness.input_i < -18.0 {
        println!("Loudness is already below -18 LKFS, no need to adjust.");
        return Ok(path.to_str().unwrap().to_string());
    }

    // rename original file to original-{}.mp3
    // output_path remains the same as input_path aka Path::path
    let current_dir = path.parent().expect("Failed to get parent directory");
    let original_file_name = format!("original-{}", file_name);
    let original_path = current_dir.join(&original_file_name);
    fs::rename(path, &original_path).expect("Failed to rename file");
    println!("Rename: {} -> {}", path.display(), format!("original-{}", path.display()));

    // ffmpeg
    std::process::Command::new(ffmpeg_path)
        .args([
            "-i", original_path.to_str().unwrap(),  // input path
            "-af",
            &format!(
                "loudnorm=I={limitation}:TP=-1.5:LRA=11:measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset=0:linear=true",
                loudness.input_i, loudness.input_tp, loudness.input_lra, loudness.input_thresh
            ),
            path.to_str().unwrap(), // output path
        ])
        .status()?;
    Ok(/*return*/path.to_str().unwrap().to_string())    // to string to avoid lifetime issues

}

pub fn ffmpeg_process_dir(path: &str, ffmpeg_path: &str, limitation: f32) -> Result<String, Box<dyn std::error::Error>> {
    println!("Processing directory: {}", path);
    // check if path is a directory
    match fs::metadata(path) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                println!("Input is not a directory: {}", path);
                exit(12);
            }
        }
        Err(_) => {
            println!("Directory not found: {}", path);
            exit(11);
        }
    }
    // get all files in the directory
    let files = fs::read_dir(path).expect("Failed to read directory");
    for file in files {
        let entry = file.expect("Failed to read entry");
        let file_path = entry.path();
        // if file is not .mp3 file, in the future more formats would be supported
        if file_path.is_file() && file_path.extension().map_or(false, |ext| ext == "mp3") {
            ffmpeg_process(file_path.to_str().unwrap(), ffmpeg_path, limitation)?;
        }

    }
    /*return*/Ok(path.to_string())
}