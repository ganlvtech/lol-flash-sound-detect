use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom};
use std::slice::from_raw_parts_mut;
use byteorder::{LittleEndian, ReadBytesExt};

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

pub fn conv(data: &[f32], kernel: &[f32]) -> f32 {
    let mut sum = 0.0;
    let len = min(data.len(), kernel.len());
    for i in 0..len {
        sum += data[i] * kernel[i];
    }
    sum
}

fn main() -> anyhow::Result<()> {
    let kernel1 = load_48k_32bit_wav("心之钢_01.wav")?;
    let kernel2 = load_48k_32bit_wav("心之钢_02.wav")?;
    let kernel3 = load_48k_32bit_wav("心之钢_03.wav")?;
    let len = kernel1.len();
    let data = load_48k_32bit_wav("心之钢测试.wav")?;
    let mut f = BufWriter::new(OpenOptions::new().write(true).create(true).truncate(true).open("心之钢测试结果.csv")?);
    writeln!(f, "sample,kernel1,kernel2,kernel3")?;
    for i in (0..(data.len() - len)).step_by(24) {
        let v1 = conv(&data[i..(i + len)], &kernel1);
        let v2 = conv(&data[i..(i + len)], &kernel2);
        let v3 = conv(&data[i..(i + len)], &kernel3);
        writeln!(f, "{},{},{},{}", i, v1, v2, v3)?;
    }
    return Ok(());
}
