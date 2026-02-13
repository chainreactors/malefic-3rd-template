use malefic_3rd_ffi::*;
use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn GoModuleName() -> *const c_char;
    fn GoModuleSend(task_id: c_uint, data: *const c_char, data_len: c_int) -> c_int;
    fn GoModuleRecv(task_id: c_uint, out_len: *mut c_int, status: *mut c_int) -> *mut c_char;
    fn GoModuleCloseInput(task_id: c_uint);
}

/// Send a serialized protobuf request to the Go module via FFI.
fn go_send(id: u32, data: &[u8]) -> anyhow::Result<()> {
    let rc = unsafe {
        GoModuleSend(
            id as c_uint,
            data.as_ptr() as *const c_char,
            data.len() as c_int,
        )
    };
    if rc != 0 {
        return Err(anyhow!("GoModuleSend failed (task {})", id));
    }
    Ok(())
}

/// Blocking call that reads the next response from Go.
/// Returns `Ok(Some(bytes))` for data, `Ok(None)` when the module is done.
fn go_recv_blocking(id: u32) -> anyhow::Result<Option<Vec<u8>>> {
    let mut out_len: c_int = 0;
    let mut status: c_int = 0;
    let ptr = unsafe { GoModuleRecv(id as c_uint, &mut out_len, &mut status) };
    match status {
        0 => {
            if ptr.is_null() {
                return Err(anyhow!("GoModuleRecv returned null with status 0"));
            }
            let buf = unsafe { FfiBuffer::new(ptr, out_len as usize) };
            Ok(Some(buf.as_bytes().to_vec()))
        }
        1 => Ok(None), // done
        _ => Err(anyhow!("GoModuleRecv error (status={})", status)),
    }
}

pub struct GolangModule {
    name: String,
}

impl GolangModule {
    pub fn new() -> Self {
        let name = unsafe { ffi_module_name(GoModuleName, true) };
        Self { name }
    }
}

#[async_trait]
impl Module for GolangModule {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "golang_module"
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        GolangModule::new()
    }

    fn new_instance(&self) -> Box<MaleficModule> {
        Box::new(GolangModule {
            name: self.name.clone(),
        })
    }
}

#[async_trait]
impl ModuleImpl for GolangModule {
    async fn run(
        &mut self,
        id: u32,
        receiver: &mut Input,
        sender: &mut Output,
    ) -> ModuleResult {
        use futures::channel::mpsc;
        use futures::StreamExt;

        // Channel for the blocking recv thread to send decoded responses back.
        let (recv_tx, mut recv_rx) = mpsc::unbounded::<anyhow::Result<Option<Vec<u8>>>>();

        // Spawn a thread that blocks on GoModuleRecv and forwards results.
        let recv_task_id = id;
        let recv_handle = std::thread::spawn(move || {
            loop {
                let result = go_recv_blocking(recv_task_id);
                let is_done = matches!(&result, Ok(None));
                let is_err = result.is_err();
                let _ = recv_tx.unbounded_send(result);
                if is_done || is_err {
                    break;
                }
            }
        });

        let mut last_result: Option<TaskResult> = None;
        let mut go_done = false;
        let mut input_closed = false;

        loop {
            if go_done {
                break;
            }

            if input_closed {
                // Input already closed — only wait for Go responses.
                match recv_rx.next().await {
                    Some(Ok(Some(response_bytes))) => {
                        let response = decode_response(&response_bytes)?;
                        let task_result = TaskResult::new_with_body(id, Body::Response(response));
                        if let Some(prev) = last_result.take() {
                            let _ = sender.unbounded_send(prev);
                        }
                        last_result = Some(task_result);
                    }
                    Some(Ok(None)) => go_done = true,
                    Some(Err(e)) => {
                        go_done = true;
                        if last_result.is_none() {
                            return Err(e.into());
                        }
                    }
                    None => go_done = true,
                }
            } else {
                futures::select! {
                    // Incoming request from Rust side → forward to Go
                    body = receiver.next() => {
                        match body {
                            Some(Body::Request(request)) => {
                                let buf = encode_request(&request)?;
                                go_send(id, &buf)?;
                            }
                            _ => {
                                // Input closed or unexpected body — close Go input once
                                input_closed = true;
                                unsafe { GoModuleCloseInput(id as c_uint) };
                            }
                        }
                    }
                    // Response from Go recv thread
                    recv_result = recv_rx.next() => {
                        match recv_result {
                            Some(Ok(Some(response_bytes))) => {
                                let response = decode_response(&response_bytes)?;
                                let task_result = TaskResult::new_with_body(id, Body::Response(response));
                                if let Some(prev) = last_result.take() {
                                    let _ = sender.unbounded_send(prev);
                                }
                                last_result = Some(task_result);
                            }
                            Some(Ok(None)) => go_done = true,
                            Some(Err(e)) => {
                                go_done = true;
                                if last_result.is_none() {
                                    return Err(e.into());
                                }
                            }
                            None => go_done = true,
                        }
                    }
                }
            }
        }

        // Wait for the recv thread to finish.
        let _ = recv_handle.join();

        last_result.ok_or_else(|| anyhow!("Go module produced no output").into())
    }
}

/// Register the Go module into the bundle.
pub fn register(map: &mut MaleficBundle) {
    let module = GolangModule::new();
    map.insert(module.name.clone(), Box::new(module));
}
