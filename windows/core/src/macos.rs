//! Apple IOBluetooth/CoreAudio backend. Native RFCOMM objects never leave
//! their owning run-loop thread; only requests and byte buffers cross threads.
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::identity::{friendly_name, is_bose};
use crate::models::Headphones;
use crate::platform::{endpoint_matches, Platform};
use crate::protocol::{canonical_mac, has_reply_to, BmapError};
use crate::transport::{open_first_answering, Transport, CONNECT_TIMEOUT};
use serde::Deserialize;

extern "C" {
    fn qc_paired_devices() -> *mut c_char;
    fn qc_audio_outputs() -> *mut c_char;
    fn qc_free_string(value: *mut c_char);
    fn qc_open(address: *const c_char, channel: u8, timeout: f64, status: *mut i32) -> *mut c_void;
    fn qc_write(handle: *mut c_void, bytes: *const u8, length: usize) -> i32;
    fn qc_read(handle: *mut c_void, bytes: *mut u8, capacity: usize) -> std::ffi::c_long;
    fn qc_pump(seconds: f64);
    fn qc_close(handle: *mut c_void);
}

// Both native JSON functions return malloc-owned, NUL-terminated UTF-8.
unsafe fn read_json<T: serde::de::DeserializeOwned>(ptr: *mut c_char) -> Option<T> {
    if ptr.is_null() {
        return None;
    }
    let result = serde_json::from_slice(CStr::from_ptr(ptr).to_bytes()).ok();
    qc_free_string(ptr);
    result
}

#[derive(Deserialize)]
struct Device {
    address: String,
    name: String,
    connected: bool,
    vendor: Option<u16>,
    source: Option<u16>,
}

fn headphones(devices: Vec<Device>) -> Vec<Headphones> {
    let mut found: Vec<_> = devices
        .into_iter()
        .filter(|d| is_bose(&d.name, d.vendor, d.source))
        .map(|d| Headphones {
            address: canonical_mac(&d.address),
            name: friendly_name(&d.name),
            connected: d.connected,
        })
        .collect();
    found.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    found
}

#[derive(Default)]
pub struct MacPlatform;
impl Platform for MacPlatform {
    fn paired_headphones(&mut self) -> Vec<Headphones> {
        headphones(unsafe { read_json(qc_paired_devices()) }.unwrap_or_default())
    }
    fn open(&mut self, address: &str, channels: &[u8]) -> Result<Box<dyn Transport>, BmapError> {
        open_first_answering(channels, |channel| {
            MacRfcomm::open(address, channel).map(|link| Box::new(link) as Box<dyn Transport>)
        })
    }
    fn pc_audio_available(&mut self, name_hint: &str) -> bool {
        let names: Vec<String> = unsafe { read_json(qc_audio_outputs()) }.unwrap_or_default();
        names.iter().any(|name| endpoint_matches(name, name_hint))
    }
}

struct Request {
    bytes: Vec<u8>,
    expect_reply: bool,
    drain: bool,
    timeout: Duration,
    reply: Sender<Result<Vec<u8>, BmapError>>,
}
struct MacRfcomm {
    requests: Option<Sender<Request>>,
    thread: Option<JoinHandle<()>>,
    channel: u8,
}
// This pointer is local to the I/O thread and intentionally not Send.
struct NativeLink(*mut c_void);
impl Drop for NativeLink {
    fn drop(&mut self) {
        unsafe { qc_close(self.0) };
    }
}
impl NativeLink {
    fn exchange(&self, request: &Request) -> Result<Vec<u8>, BmapError> {
        unsafe { qc_pump(0.008) };
        let status = unsafe { qc_write(self.0, request.bytes.as_ptr(), request.bytes.len()) };
        if status != 0 {
            return Err(BmapError::Transport(format!(
                "Bluetooth write failed ({status})."
            )));
        }
        if !request.expect_reply {
            unsafe { qc_pump(0.12) };
            return Ok(Vec::new());
        }
        let deadline = Instant::now() + request.timeout;
        let mut buffer = Vec::new();
        let mut last_data = Instant::now();
        while Instant::now() < deadline {
            unsafe { qc_pump(0.008) };
            let mut chunk = [0u8; 4096];
            let n = unsafe { qc_read(self.0, chunk.as_mut_ptr(), chunk.len()) };
            if n < 0 {
                return Err(BmapError::NotConnected);
            }
            if n > 0 {
                buffer.extend_from_slice(&chunk[..n as usize]);
                last_data = Instant::now();
                if buffer.len() > 256 * 1024 {
                    return Err(BmapError::Transport(
                        "Bluetooth response exceeded 256 KiB.".into(),
                    ));
                }
                if !request.drain && has_reply_to(&buffer, &request.bytes) {
                    break;
                }
            }
            if request.drain
                && !buffer.is_empty()
                && last_data.elapsed() >= Duration::from_millis(300)
            {
                break;
            }
        }
        if buffer.is_empty() {
            Err(BmapError::Timeout)
        } else {
            Ok(buffer)
        }
    }
}
impl MacRfcomm {
    fn open(address: &str, channel: u8) -> Result<Self, BmapError> {
        let address = CString::new(address)
            .map_err(|_| BmapError::Transport("Invalid Bluetooth address.".into()))?;
        let (requests, rx) = mpsc::channel::<Request>();
        let (ready, opened) = mpsc::channel();
        let thread = std::thread::Builder::new().name("qcbuddy-rfcomm".into()).spawn(move || {
            let mut status = 0;
            let ptr = unsafe { qc_open(address.as_ptr(), channel, CONNECT_TIMEOUT.as_secs_f64(), &mut status) };
            if ptr.is_null() {
                let error = if status == -2 { BmapError::Timeout } else {
                    BmapError::Transport(format!("macOS couldn't open Bluetooth channel {channel} ({status}). Check Bluetooth permission and the headset connection."))
                };
                let _ = ready.send(Err(error));
                return;
            }
            let link = NativeLink(ptr);
            if ready.send(Ok(())).is_err() { return; }
            loop {
                match rx.recv_timeout(Duration::from_millis(8)) {
                    Ok(request) => { let _ = request.reply.send(link.exchange(&request)); }
                    Err(mpsc::RecvTimeoutError::Timeout) => unsafe { qc_pump(0.008) },
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        }).map_err(|e| BmapError::Transport(e.to_string()))?;
        let result = opened.recv().unwrap_or(Err(BmapError::NotConnected));
        if let Err(error) = result {
            let _ = thread.join();
            return Err(error);
        }
        Ok(Self {
            requests: Some(requests),
            thread: Some(thread),
            channel,
        })
    }
}
impl Transport for MacRfcomm {
    fn exchange(
        &mut self,
        request: &[u8],
        expect_reply: bool,
        drain: bool,
        timeout: Duration,
    ) -> Result<Vec<u8>, BmapError> {
        let (reply, received) = mpsc::channel();
        self.requests
            .as_ref()
            .ok_or(BmapError::NotConnected)?
            .send(Request {
                bytes: request.to_vec(),
                expect_reply,
                drain,
                timeout,
                reply,
            })
            .map_err(|_| BmapError::NotConnected)?;
        received.recv().unwrap_or(Err(BmapError::NotConnected))
    }
    fn channel(&self) -> u8 {
        self.channel
    }
}
impl Drop for MacRfcomm {
    fn drop(&mut self) {
        self.requests.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_preserves_renamed_bose_and_prefers_connected_headsets() {
        let devices = serde_json::from_str(r#"[
            {"address":"aa-bb-cc-dd-ee-01","name":"Bose QC45","connected":false,"vendor":null,"source":null},
            {"address":"aa-bb-cc-dd-ee-02","name":"My cans","connected":true,"vendor":158,"source":1},
            {"address":"aa-bb-cc-dd-ee-03","name":"Other","connected":true,"vendor":158,"source":2}
        ]"#).unwrap();
        let found = headphones(devices);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "My cans");
        assert_eq!(found[0].address, "AA:BB:CC:DD:EE:02");
        assert!(!found[1].connected);
    }
}
