use libloading::{Library, Symbol};
use malefic_proto::module::MaleficBundle;
use malefic_proto::proto::implantpb::spite::Body;
use malefic_proto::proto::modulepb;
use std::path::PathBuf;

#[allow(improper_ctypes_definitions)]
type RegisterModulesFn = unsafe extern "C" fn() -> MaleficBundle;

fn find_dll() -> PathBuf {
    if let Ok(p) = std::env::var("MALEFIC_3RD_DLL") {
        return PathBuf::from(p);
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target").join("x86_64-pc-windows-gnu");

    for profile in &["release", "debug"] {
        let dll = target_dir.join(profile).join("deps").join("malefic_3rd.dll");
        if dll.exists() {
            return dll;
        }
        let dll = target_dir.join(profile).join("malefic_3rd.dll");
        if dll.exists() {
            return dll;
        }
    }

    panic!(
        "malefic_3rd.dll not found. Build it first:\n  \
         cargo build --target x86_64-pc-windows-gnu --features golang_module [--release]"
    );
}

unsafe fn load_bundle() -> (Library, MaleficBundle) {
    let dll_path = find_dll();
    println!("Loading DLL: {}", dll_path.display());

    let lib = Library::new(&dll_path).expect("Failed to load malefic_3rd.dll");
    let register: Symbol<RegisterModulesFn> = lib
        .get(b"register_modules")
        .expect("Symbol 'register_modules' not found");
    let bundle = register();
    (lib, bundle)
}

#[test]
fn test_dll_loads_and_registers_modules() {
    unsafe {
        let (_lib, bundle) = load_bundle();

        println!("Registered modules: {:?}", bundle.keys().collect::<Vec<_>>());
        assert!(bundle.contains_key("rust_module"));
        assert!(bundle.contains_key("example_go"));
        assert!(bundle.contains_key("example_c"));
        assert!(bundle.contains_key("example_zig"));
        assert!(bundle.contains_key("example_nim"));
        println!("All {} modules registered.", bundle.len());
    }
}

/// Single request → single response, then input closes → Go module finishes.
#[test]
fn test_golang_module_single_request() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();

        let module = bundle
            .get_mut("example_go")
            .expect("'example_go' module not found in bundle");

        let (input_tx, mut input_rx) = futures::channel::mpsc::unbounded::<Body>();
        let (mut output_tx, _output_rx) =
            futures::channel::mpsc::unbounded::<malefic_proto::module::TaskResult>();

        let request = modulepb::Request {
            input: "helloworld".to_string(),
            ..Default::default()
        };
        input_tx
            .unbounded_send(Body::Request(request))
            .expect("Failed to send request");
        drop(input_tx);

        let task_id = 100u32;
        let result =
            futures::executor::block_on(module.run(task_id, &mut input_rx, &mut output_tx));

        let task_result = result.expect("module.run() returned error");
        assert_eq!(task_result.task_id, task_id);

        match task_result.body {
            Body::Response(resp) => {
                println!("Single request response: {:?}", resp.output);
                assert_eq!(resp.output, "hello from golang module, input: helloworld");
            }
            _ => panic!("Expected Body::Response, got unexpected variant"),
        }

        println!("Single request test passed!");
    }
}

/// Multiple requests streamed in, then input closes → collect all responses.
#[test]
fn test_golang_module_multi_stream() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();

        let module = bundle
            .get_mut("example_go")
            .expect("'example_go' module not found in bundle");

        let (input_tx, mut input_rx) = futures::channel::mpsc::unbounded::<Body>();
        let (mut output_tx, mut output_rx) =
            futures::channel::mpsc::unbounded::<malefic_proto::module::TaskResult>();

        let messages = vec!["alpha", "beta", "gamma"];

        for msg in &messages {
            let request = modulepb::Request {
                input: msg.to_string(),
                ..Default::default()
            };
            input_tx
                .unbounded_send(Body::Request(request))
                .expect("Failed to send request");
        }
        drop(input_tx);

        let task_id = 200u32;
        let result =
            futures::executor::block_on(module.run(task_id, &mut input_rx, &mut output_tx));

        // Collect intermediate + final responses.
        let mut responses = Vec::new();
        drop(output_tx);
        {
            use futures::{FutureExt, StreamExt};
            while let Some(tr) = output_rx.next().now_or_never().flatten() {
                if let Body::Response(resp) = tr.body {
                    responses.push(resp.output);
                }
            }
        }

        let task_result = result.expect("module.run() returned error");
        assert_eq!(task_result.task_id, task_id);
        if let Body::Response(resp) = task_result.body {
            responses.push(resp.output);
        }

        println!("Multi-stream responses: {:?}", responses);
        assert_eq!(responses.len(), messages.len());
        for (i, msg) in messages.iter().enumerate() {
            let expected = format!("hello from golang module, input: {}", msg);
            assert_eq!(responses[i], expected);
        }

        println!("Multi-stream test passed!");
    }
}

/// C module: single request → single response (synchronous handler).
#[test]
fn test_c_module_single_request() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();

        let module = bundle
            .get_mut("example_c")
            .expect("'example_c' module not found in bundle");

        let (input_tx, mut input_rx) = futures::channel::mpsc::unbounded::<Body>();
        let (mut output_tx, _output_rx) =
            futures::channel::mpsc::unbounded::<malefic_proto::module::TaskResult>();

        let request = modulepb::Request {
            input: "helloworld".to_string(),
            ..Default::default()
        };
        input_tx
            .unbounded_send(Body::Request(request))
            .expect("Failed to send request");
        drop(input_tx);

        let task_id = 300u32;
        let result =
            futures::executor::block_on(module.run(task_id, &mut input_rx, &mut output_tx));

        let task_result = result.expect("module.run() returned error");
        assert_eq!(task_result.task_id, task_id);

        match task_result.body {
            Body::Response(resp) => {
                println!("C module response: {:?}", resp.output);
                assert_eq!(resp.output, "hello from c module, input: helloworld");
            }
            _ => panic!("Expected Body::Response, got unexpected variant"),
        }

        println!("C module single request test passed!");
    }
}

/// Zig module: single request → single response (synchronous handler).
#[test]
fn test_zig_module_single_request() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();

        let module = bundle
            .get_mut("example_zig")
            .expect("'example_zig' module not found in bundle");

        let (input_tx, mut input_rx) = futures::channel::mpsc::unbounded::<Body>();
        let (mut output_tx, _output_rx) =
            futures::channel::mpsc::unbounded::<malefic_proto::module::TaskResult>();

        let request = modulepb::Request {
            input: "helloworld".to_string(),
            ..Default::default()
        };
        input_tx
            .unbounded_send(Body::Request(request))
            .expect("Failed to send request");
        drop(input_tx);

        let task_id = 400u32;
        let result =
            futures::executor::block_on(module.run(task_id, &mut input_rx, &mut output_tx));

        let task_result = result.expect("module.run() returned error");
        assert_eq!(task_result.task_id, task_id);

        match task_result.body {
            Body::Response(resp) => {
                println!("Zig module response: {:?}", resp.output);
                assert_eq!(resp.output, "hello from zig module, input: helloworld");
            }
            _ => panic!("Expected Body::Response, got unexpected variant"),
        }

        println!("Zig module single request test passed!");
    }
}

// ── Multi-stream helpers & tests ──────────────────────────────────────────

/// Helper: batch multi-stream test for any module.
///
/// Sends all `messages` up-front, closes the input, runs the module,
/// then collects intermediate results (from sender) + final result (from return)
/// and asserts they match `"{prefix}{msg}"` in order.
unsafe fn run_multi_stream_batch(
    bundle: &mut MaleficBundle,
    module_name: &str,
    prefix: &str,
    messages: &[&str],
    task_id: u32,
) {
    let module = bundle
        .get_mut(module_name)
        .unwrap_or_else(|| panic!("'{}' not found in bundle", module_name));

    let (input_tx, mut input_rx) = futures::channel::mpsc::unbounded::<Body>();
    let (mut output_tx, mut output_rx) =
        futures::channel::mpsc::unbounded::<malefic_proto::module::TaskResult>();

    for msg in messages {
        let request = modulepb::Request {
            input: msg.to_string(),
            ..Default::default()
        };
        input_tx
            .unbounded_send(Body::Request(request))
            .expect("send failed");
    }
    drop(input_tx);

    let result =
        futures::executor::block_on(module.run(task_id, &mut input_rx, &mut output_tx));

    // Drain intermediate results from sender channel.
    let mut responses = Vec::new();
    drop(output_tx);
    {
        use futures::{FutureExt, StreamExt};
        while let Some(tr) = output_rx.next().now_or_never().flatten() {
            assert_eq!(tr.task_id, task_id);
            if let Body::Response(resp) = tr.body {
                responses.push(resp.output);
            }
        }
    }

    // Final result from return value.
    let task_result = result.expect("module.run() returned error");
    assert_eq!(task_result.task_id, task_id);
    if let Body::Response(resp) = task_result.body {
        responses.push(resp.output);
    } else {
        panic!("final result is not Body::Response");
    }

    assert_eq!(
        responses.len(),
        messages.len(),
        "[{}] expected {} responses, got {}",
        module_name,
        messages.len(),
        responses.len()
    );
    for (i, msg) in messages.iter().enumerate() {
        let expected = format!("{}{}", prefix, msg);
        assert_eq!(responses[i], expected, "[{}] response #{} mismatch", module_name, i);
    }
}

/// C module: multiple requests → ordered responses via ffi_run_loop.
#[test]
fn test_c_module_multi_stream() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();
        let msgs = vec!["alpha", "beta", "gamma"];
        run_multi_stream_batch(
            &mut bundle,
            "example_c",
            "hello from c module, input: ",
            &msgs,
            1100,
        );
        println!("C multi-stream ({} rounds) passed!", msgs.len());
    }
}

/// Zig module: multiple requests → ordered responses via ffi_run_loop.
#[test]
fn test_zig_module_multi_stream() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();
        let msgs = vec!["alpha", "beta", "gamma"];
        run_multi_stream_batch(
            &mut bundle,
            "example_zig",
            "hello from zig module, input: ",
            &msgs,
            1200,
        );
        println!("Zig multi-stream ({} rounds) passed!", msgs.len());
    }
}

/// Nim module: multiple requests → ordered responses via ffi_run_loop.
#[test]
fn test_nim_module_multi_stream() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();
        let msgs = vec!["alpha", "beta", "gamma"];
        run_multi_stream_batch(
            &mut bundle,
            "example_nim",
            "hello from nim module, input: ",
            &msgs,
            1300,
        );
        println!("Nim multi-stream ({} rounds) passed!", msgs.len());
    }
}

/// C module: 5-round batch to verify longer conversations.
#[test]
fn test_c_module_multi_stream_5_rounds() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();
        let msgs = vec!["r1", "r2", "r3", "r4", "r5"];
        run_multi_stream_batch(
            &mut bundle,
            "example_c",
            "hello from c module, input: ",
            &msgs,
            1400,
        );
        println!("C multi-stream 5-round passed!");
    }
}

// ── Interleaved (true alternating) tests ─────────────────────────────────

/// Helper: true interleaved send-receive test.
///
/// Runs the module on a background thread. The main thread sends one request
/// at a time, waits for the corresponding response, then sends the next.
/// This verifies genuine bidirectional multi-round alternating communication.
///
/// Because `ffi_run_loop` forwards the *previous* result when the *next*
/// request arrives, the protocol is:
///   send(req_i) → send(req_{i+1}) → recv(resp_i)
/// For the last request, dropping input_tx causes the loop to exit and
/// the final response is returned from `run()`.
unsafe fn run_interleaved_test(
    bundle: &mut MaleficBundle,
    module_name: &str,
    prefix: &str,
    messages: &[&str],
    task_id: u32,
) {
    use futures::StreamExt;

    assert!(
        messages.len() >= 2,
        "interleaved test needs at least 2 messages"
    );

    // Take an owned module so we can move it into a thread.
    let mut module = bundle
        .remove(module_name)
        .unwrap_or_else(|| panic!("'{}' not found in bundle", module_name));

    let (input_tx, mut input_rx) = futures::channel::mpsc::unbounded::<Body>();
    let (mut output_tx, mut output_rx) =
        futures::channel::mpsc::unbounded::<malefic_proto::module::TaskResult>();

    // Run the module on a background thread.
    let handle = std::thread::spawn(move || {
        let result =
            futures::executor::block_on(module.run(task_id, &mut input_rx, &mut output_tx));
        (result, output_tx) // return output_tx so it's dropped after run() finishes
    });

    let mut collected: Vec<String> = Vec::new();

    // Send first request — no response yet (nothing previous to flush).
    input_tx
        .unbounded_send(Body::Request(modulepb::Request {
            input: messages[0].to_string(),
            ..Default::default()
        }))
        .expect("send[0] failed");

    // For each subsequent request: send it, then read the previous response.
    for i in 1..messages.len() {
        input_tx
            .unbounded_send(Body::Request(modulepb::Request {
                input: messages[i].to_string(),
                ..Default::default()
            }))
            .expect(&format!("send[{}] failed", i));

        // The module should now have flushed response[i-1] via sender.
        // Block until we receive it.
        let tr = futures::executor::block_on(output_rx.next())
            .unwrap_or_else(|| panic!("expected intermediate response #{}", i - 1));
        assert_eq!(tr.task_id, task_id);
        if let Body::Response(resp) = tr.body {
            collected.push(resp.output);
        } else {
            panic!("intermediate #{} is not Body::Response", i - 1);
        }
    }

    // Close input → module loop exits, returning the last response.
    drop(input_tx);

    let (result, _output_tx) = handle.join().expect("module thread panicked");
    let task_result = result.expect("module.run() returned error");
    assert_eq!(task_result.task_id, task_id);
    if let Body::Response(resp) = task_result.body {
        collected.push(resp.output);
    } else {
        panic!("final result is not Body::Response");
    }

    // Verify all responses in order.
    assert_eq!(collected.len(), messages.len());
    for (i, msg) in messages.iter().enumerate() {
        let expected = format!("{}{}", prefix, msg);
        assert_eq!(
            collected[i], expected,
            "[{}] interleaved response #{} mismatch",
            module_name, i
        );
    }
}

/// C module: true alternating send→recv→send→recv across threads.
#[test]
fn test_c_module_interleaved() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();
        let msgs = vec!["ping", "pong", "fin"];
        run_interleaved_test(
            &mut bundle,
            "example_c",
            "hello from c module, input: ",
            &msgs,
            2100,
        );
        println!("C interleaved 3-round passed!");
    }
}

/// Zig module: true alternating send→recv→send→recv across threads.
#[test]
fn test_zig_module_interleaved() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();
        let msgs = vec!["ping", "pong", "fin"];
        run_interleaved_test(
            &mut bundle,
            "example_zig",
            "hello from zig module, input: ",
            &msgs,
            2200,
        );
        println!("Zig interleaved 3-round passed!");
    }
}

/// Nim module: true alternating send→recv→send→recv across threads.
#[test]
fn test_nim_module_interleaved() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();
        let msgs = vec!["ping", "pong", "fin"];
        run_interleaved_test(
            &mut bundle,
            "example_nim",
            "hello from nim module, input: ",
            &msgs,
            2300,
        );
        println!("Nim interleaved 3-round passed!");
    }
}

/// Go module: true alternating send→recv→send→recv across threads.
#[test]
fn test_golang_module_interleaved() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();
        let msgs = vec!["ping", "pong", "fin"];
        run_interleaved_test(
            &mut bundle,
            "example_go",
            "hello from golang module, input: ",
            &msgs,
            2400,
        );
        println!("Go interleaved 3-round passed!");
    }
}

/// Nim module: single request → single response (synchronous handler).
#[test]
fn test_nim_module_single_request() {
    unsafe {
        let (_lib, mut bundle) = load_bundle();

        let module = bundle
            .get_mut("example_nim")
            .expect("'example_nim' module not found in bundle");

        let (input_tx, mut input_rx) = futures::channel::mpsc::unbounded::<Body>();
        let (mut output_tx, _output_rx) =
            futures::channel::mpsc::unbounded::<malefic_proto::module::TaskResult>();

        let request = modulepb::Request {
            input: "helloworld".to_string(),
            ..Default::default()
        };
        input_tx
            .unbounded_send(Body::Request(request))
            .expect("Failed to send request");
        drop(input_tx);

        let task_id = 500u32;
        let result =
            futures::executor::block_on(module.run(task_id, &mut input_rx, &mut output_tx));

        let task_result = result.expect("module.run() returned error");
        assert_eq!(task_result.task_id, task_id);

        match task_result.body {
            Body::Response(resp) => {
                println!("Nim module response: {:?}", resp.output);
                assert_eq!(resp.output, "hello from nim module, input: helloworld");
            }
            _ => panic!("Expected Body::Response, got unexpected variant"),
        }

        println!("Nim module single request test passed!");
    }
}
