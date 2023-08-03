use std::cmp::min;
use std::collections::VecDeque;
use std::ptr::null_mut;
use std::slice::from_raw_parts;
use anyhow::anyhow;
use windows::Win32::Media::Audio::{AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, eMultimedia, eRender, IAudioCaptureClient, IAudioClient, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, WAVEFORMATEX, WAVEFORMATEXTENSIBLE};
use windows::Win32::Media::KernelStreaming::WAVE_FORMAT_EXTENSIBLE;
use windows::Win32::Media::Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoInitialize, CoTaskMemFree, CoUninitialize};

pub struct Recorder {
    enumerator: IMMDeviceEnumerator,
    device: IMMDevice,
    audio_client: IAudioClient,
    capture_client: IAudioCaptureClient,
    buffer: VecDeque<f32>,
}

const REFTIMES_PER_SEC: i64 = 10000000;

impl Recorder {
    pub fn new() -> anyhow::Result<Recorder> {
        unsafe {
            // 代码改造自 MSDN 的示例代码
            // https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording
            // https://learn.microsoft.com/en-us/windows/win32/coreaudio/capturing-a-stream
            // https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclient-initialize#examples

            CoInitialize(None)?; // CoInitialize 是引用计数的，CoUninitialize 需要与 CoInitialize 个数一致。
            // 说明：windows-rs 封装了所有 IXxx 接口，底层的 IUnknown 实现了 Drop，会自动释放，不需要我们手动调用 Release
            // 但是我们必须将他保存在结构体的字段中，否则，离开当前函数就会被立刻释放了。
            let enumerator = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let audio_client = device.Activate::<IAudioClient>(CLSCTX_ALL, None)?;
            {
                let pwfx = ScopedWAVEFORMATEX(audio_client.GetMixFormat()?);
                if (*pwfx.0).wFormatTag != WAVE_FORMAT_EXTENSIBLE as u16 {
                    return Err(anyhow!("Only support WAVE_FORMAT_EXTENSIBLE wave format"));
                }
                if (*pwfx.0).nChannels != 2 {
                    return Err(anyhow!("Only support stereo wave format"));
                }
                if (*pwfx.0).nSamplesPerSec != 48000 {
                    return Err(anyhow!("Only support 48000kHz wave format"));
                }
                if (*pwfx.0).wBitsPerSample != 32 {
                    return Err(anyhow!("Only support 32bit wave format"));
                }
                let pwfxe = pwfx.0 as *mut WAVEFORMATEXTENSIBLE;
                let subtype = (*pwfxe).SubFormat;
                if subtype != KSDATAFORMAT_SUBTYPE_IEEE_FLOAT {
                    return Err(anyhow!("Only support 32bit float-point wave format"));
                }
                audio_client.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, REFTIMES_PER_SEC, 0, pwfx.0, None)?;
            }
            let capture_client = audio_client.GetService::<IAudioCaptureClient>()?;
            audio_client.Start()?;
            Ok(Self {
                enumerator,
                device,
                audio_client,
                capture_client,
                buffer: VecDeque::with_capacity(48000),
            })
        }
    }

    pub fn capture(&mut self) -> anyhow::Result<u32> {
        unsafe {
            let mut p_data: *mut u8 = null_mut();
            let mut num_frames_to_read = 0;
            let mut dwflags = 0;
            self.capture_client.GetBuffer(&mut p_data, &mut num_frames_to_read, &mut dwflags, None, None)?;
            if num_frames_to_read > 0 {
                let data = from_raw_parts(p_data as *mut f32, num_frames_to_read as usize * 2);
                self.buffer.extend(data);
                self.capture_client.ReleaseBuffer(num_frames_to_read)?;
            }
            Ok(num_frames_to_read)
        }
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn buffer_iter(&self) -> impl Iterator<Item=&f32> {
        self.buffer.iter()
    }

    pub fn buffer_pop_front_n(&mut self, n: usize) {
        for _ in 0..min(n, self.buffer.len()) {
            self.buffer.pop_front();
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        unsafe {
            let _ = self.audio_client.Stop();
            CoUninitialize();
        }
    }
}


struct ScopedWAVEFORMATEX(*mut WAVEFORMATEX);

impl Drop for ScopedWAVEFORMATEX {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0 as _));
        }
    }
}
