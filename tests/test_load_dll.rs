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
