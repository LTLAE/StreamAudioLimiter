use std::fs;
use std::process::exit;
use std::env;
use std::path::Path;
use regex::Regex;
use serde::Deserialize;

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
pub fn ffmpeg_process(path: &str, ffmpeg_path: &str, limitation: f32, terminal_output: &mut String) -> Result<String, Box<dyn std::error::Error>>{
    let path = Path::new(path); // convert to Path type
    terminal_output.push_str(&format!("Current working directory: {}\n", env::current_dir().unwrap().display()));
    // if path is not a file, return error
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                let error_msg = format!("Input is not a file: {}", path.display());
                terminal_output.push_str(&format!("ERROR: {}\n", error_msg));
                exit(12);
            }
        }
        Err(_) => {
            let error_msg = format!("File not found: {}", path.display());
            terminal_output.push_str(&format!("ERROR: {}\n", error_msg));
            exit(11);
        }
    }
    let file_name = path.file_name().unwrap().to_str().unwrap();
    // limit: -18LKFS
    // Analyze the audio file
    terminal_output.push_str(&format!("Analyzing audio file: {}\n", path.display()));
    let loudness_result = std::process::Command::new(ffmpeg_path).args([
        "-i", path.to_str().unwrap(),
        "-af", "loudnorm=I=-18:TP=-1.5:LRA=11:print_format=json",
        "-f", "null", "-"
    ])
    .output()?;

    // Convert loudness_result to string for terminal output
    let strloudness_result = String::from_utf8_lossy(&loudness_result.stderr);
    terminal_output.push_str(&format!("FFmpeg analysis output:\n{}\n", strloudness_result));

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
            let error_msg = format!("Failed to analyze: {}", e);
            terminal_output.push_str(&format!("ERROR: {}\n", error_msg));
            return Err(Box::new(e));
        }
    };

    // show loudness
    // println!("Loudness analysis result:");
    // loudness.show_loudness();

    // if <18, return, else, calculate target gain
    terminal_output.push_str(&format!("Loudness analysis complete, loudness: {}\n", loudness.input_i));
    if loudness.input_i < limitation {
        terminal_output.push_str(&format!("Loudness is already below {} LKFS, no need to adjust.\n", limitation));
        return Ok(path.to_str().unwrap().to_string());
    }

    // rename original file to original-{}.mp3
    // output_path remains the same as input_path aka Path::path
    let current_dir = path.parent().expect("Failed to get parent directory");
    let original_file_name = format!("original-{}", file_name);
    let original_path = current_dir.join(&original_file_name);
    fs::rename(path, &original_path).expect("Failed to rename file");
    terminal_output.push_str(&format!("Renamed: {} -> {}\n", path.display(), original_file_name));

    // ffmpeg normalization
    terminal_output.push_str(&format!("Starting normalization to {} LKFS...\n", limitation));
    let normalization_result = std::process::Command::new(ffmpeg_path)
        .args([
            "-y", // overwrite output if exists
            "-i", original_path.to_str().unwrap(),  // input path
            "-af",
            &format!(
                "loudnorm=I={limitation}:TP=-1.5:LRA=11:measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset=0:linear=true",
                loudness.input_i, loudness.input_tp, loudness.input_lra, loudness.input_thresh
            ),
            "-vn", // no video
            "-acodec", "libmp3lame",
            "-ar", "44100",
            "-ac", "2",
            path.to_str().unwrap(), // output path
        ])
        .output()?;

    if normalization_result.status.success() {
        terminal_output.push_str(&format!("Normalization completed successfully for: {}\n", file_name));
    } else {
        let error_output = String::from_utf8_lossy(&normalization_result.stderr);
        terminal_output.push_str(&format!("Normalization failed: {}\n", error_output));
    }

    Ok(/*return*/path.to_str().unwrap().to_string())    // to string to avoid lifetime issues

}

pub fn ffmpeg_process_dir(path: &str, ffmpeg_path: &str, limitation: f32, terminal_output: &mut String) -> Result<String, Box<dyn std::error::Error>> {
    terminal_output.push_str(&format!("Processing directory: {}\n", path));
    // check if path is a directory
    match fs::metadata(path) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                let error_msg = format!("Input is not a directory: {}", path);
                terminal_output.push_str(&format!("ERROR: {}\n", error_msg));
                exit(12);
            }
        }
        Err(_) => {
            let error_msg = format!("Directory not found: {}", path);
            terminal_output.push_str(&format!("ERROR: {}\n", error_msg));
            exit(11);
        }
    }
    // get all files in the directory
    let files = fs::read_dir(path).expect("Failed to read directory");
    let mut processed_count = 0;
    for file in files {
        let entry = file.expect("Failed to read entry");
        let file_path = entry.path();
        // if file is not .mp3 file, in the future more formats would be supported
        if file_path.is_file() && file_path.extension().map_or(false, |ext| ext == "mp3") {
            terminal_output.push_str(&format!("Processing file {} in directory...\n", file_path.file_name().unwrap().to_str().unwrap()));
            match ffmpeg_process(file_path.to_str().unwrap(), ffmpeg_path, limitation, terminal_output) {
                Ok(_) => processed_count += 1,
                Err(e) => {
                    terminal_output.push_str(&format!("Failed to process {}: {}\n", file_path.display(), e));
                }
            }
        }
    }
    terminal_output.push_str(&format!("Directory processing completed. Processed {} files.\n", processed_count));
    /*return*/Ok(path.to_string())
}