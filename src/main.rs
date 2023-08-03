use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::slice::{from_raw_parts_mut};
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use byteorder::{LittleEndian, ReadBytesExt};
use crate::recorder::Recorder;

mod recorder;

pub fn load_48k_32bit_wav(file_name: &str) -> std::io::Result<Vec<f32>> {
    let mut f = BufReader::new(File::open(file_name)?);
    let _ = f.read_u32::<LittleEndian>()?; // "RIFF"
    let _ = f.read_u32::<LittleEndian>()?; // file_len
    let _ = f.read_u32::<LittleEndian>()?; // "WAVE"
    let _ = f.read_u32::<LittleEndian>()?; // "fmt "
    let header_len = f.read_u32::<LittleEndian>()?; // header_len
    let _ = f.seek(SeekFrom::Current(header_len as i64))?; // skip header
    let _ = f.read_u32::<LittleEndian>()?; // "data"
    let data_len = f.read_u32::<LittleEndian>()?; // data_len
    let mut buf = vec![0f32; data_len as usize / 4];
    let buf_u8 = unsafe { from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, data_len as usize) };
    let _ = f.read_exact(buf_u8)?;
    Ok(buf)
}

pub fn conv<'a>(data: impl Iterator<Item=&'a f32>, kernel: &[f32]) -> f32 {
    data.zip(kernel).map(|(a, b)| a * b).sum()
}

fn main() -> anyhow::Result<()> {
    let kernel1 = load_48k_32bit_wav("闪现_01.wav")?;
    let kernel2 = load_48k_32bit_wav("闪现_02.wav")?;
    let kernel3 = load_48k_32bit_wav("闪现_03.wav")?;
    let len = kernel1.len();
    let mut v1_history = VecDeque::with_capacity(50);
    let mut v2_history = VecDeque::with_capacity(50);
    let mut v3_history = VecDeque::with_capacity(50);
    let t0 = SystemTime::now();
    let mut recorder = Recorder::new()?;
    while let Ok(num_frames_to_read) = recorder.capture() {
        if num_frames_to_read > 0 {
            while recorder.buffer_len() >= len {
                let v1 = conv(recorder.buffer_iter(), &kernel1);
                let v2 = conv(recorder.buffer_iter(), &kernel2);
                let v3 = conv(recorder.buffer_iter(), &kernel3);
                v1_history.push_front(v1);
                v2_history.push_front(v2);
                v3_history.push_front(v3);
                while v1_history.len() > 54 {
                    v1_history.pop_back();
                }
                while v2_history.len() > 54 {
                    v2_history.pop_back();
                }
                while v3_history.len() > 54 {
                    v3_history.pop_back();
                }
                let matched = match_flash(&v1_history, &v2_history, &v3_history);
                if matched > 0 {
                    let dt = SystemTime::now().duration_since(t0).unwrap().as_secs_f32();
                    println!("{}s flash{}", dt, matched);
                    recorder.buffer_pop_front_n(len); // 直接跳过当前已匹配的音频
                    v1_history.clear();
                    v2_history.clear();
                    v3_history.clear();
                } else {
                    recorder.buffer_pop_front_n(24); // 往后移动 0.0005 秒
                }
            }
        } else {
            sleep(Duration::from_millis(1));
        }
    }
    Ok(())
}

pub fn match_flash(v1_history: &VecDeque<f32>, v2_history: &VecDeque<f32>, v3_history: &VecDeque<f32>) -> usize {
    if v1_history.len() < 54
        || v2_history.len() < 54
        || v3_history.len() < 54 {
        return 0;
    }
    if v3_history[39] > 200.0
        && (v1_history[39] / v3_history[39] - 0.03).abs() < 0.15
        && (v2_history[50] / v3_history[39] - 0.13).abs() < 0.15
        && (v1_history[20] / v3_history[39] - (-0.18)).abs() < 0.15
        && (v2_history[11] / v3_history[39] - (-0.23)).abs() < 0.15
        && (v3_history[0] / v3_history[39] - (-0.4)).abs() < 0.15
    {
        return 3;
    }
    if v1_history[10] > 100.0
        && (v2_history[0] / v1_history[10] - 0.53).abs() < 0.15
        && (v3_history[30] / v1_history[10] - (-0.25)).abs() < 0.15
        && (v1_history[48] / v1_history[10] - (-0.21)).abs() < 0.15
        && (v2_history[38] / v1_history[10] - (-0.22)).abs() < 0.15
        && (v3_history[29] / v1_history[10] - (-0.26)).abs() < 0.15
    {
        return 1;
    }
    if v2_history[12] > 100.0
        && (v1_history[22] / v2_history[10] - 0.56).abs() < 0.15
        && (v3_history[0] / v2_history[10] - 0.21).abs() < 0.15
        && (v1_history[53] / v2_history[10] - (-0.16)).abs() < 0.15
        && (v2_history[42] / v2_history[10] - (-0.23)).abs() < 0.15
        && (v3_history[40] / v2_history[10] - (-0.33)).abs() < 0.15
    {
        return 2;
    }
    0
}

