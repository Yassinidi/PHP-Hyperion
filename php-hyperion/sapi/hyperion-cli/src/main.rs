use std::env;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::io::Write;
use hyperion_vm::{fibre::Fibre, fibre::GlobalEngineState};
use hyperion_vm::scheduler::Scheduler;
use hyperion_vm::io::Reactor;

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("💥 RUST PANIC: {}", panic_info);
        eprintln!("{:?}", std::backtrace::Backtrace::capture());
    }));

    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
        let mut act: libc::sigaction = std::mem::zeroed();
        act.sa_sigaction = libc::SIG_IGN;
        libc::sigaction(libc::SIGPIPE, &act, std::ptr::null_mut());
    }

    let builder = std::thread::Builder::new()
        .name("hyperion-main".to_string())
        .stack_size(64 * 1024 * 1024);
    let handler = builder.spawn(|| {
        real_main();
    }).unwrap();
    let _ = handler.join();
}

fn real_main() {
    hyperion_core::types::string_table::preintern_http_symbols();
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: sapi [--server <host:port>] <file.php>");
        return;
    }
    
    let mut server_addr = None;
    let mut fastcgi_addr = None;
    let mut file_path = String::new();
    let mut docroot: Option<String> = None;
    let mut print_ast = false;
    let mut worker_mode = false;
    let mut engine_mode = std::env::var("HYPERION_ENGINE").unwrap_or_else(|_| "auto".to_string()).to_lowercase();
    
    let mut i = 1;
    let mut script_args = Vec::new();
    while i < args.len() {
        if args[i] == "--engine" || args[i] == "-E" {
            if i + 1 < args.len() {
                engine_mode = args[i + 1].to_lowercase();
                i += 2;
                continue;
            } else {
                eprintln!("Error: --engine requires a mode: vm, php84, zend, or auto");
                return;
            }
        }
        if args[i].starts_with("--engine=") {
            engine_mode = args[i]["--engine=".len()..].to_lowercase();
            i += 1;
            continue;
        }

        if args[i] == "--server" || args[i] == "-S" {
            if i + 1 < args.len() {
                server_addr = Some(args[i + 1].clone());
                i += 2;
                continue;
            } else {
                eprintln!("Error: --server requires a <host:port> argument");
                return;
            }
        }

        if args[i] == "--bind" || args[i] == "-b" {
            if i + 1 < args.len() {
                fastcgi_addr = Some(args[i + 1].clone());
                i += 2;
                continue;
            } else {
                eprintln!("Error: -b requires a <host:port> argument");
                return;
            }
        }

        if args[i] == "--worker" || args[i] == "-W" || args[i] == "--octane" {
            worker_mode = true;
            std::env::set_var("HYPERION_WORKER", "1");
            i += 1;
            continue;
        }

        if args[i] == "-t" {
            if i + 1 < args.len() {
                docroot = Some(args[i + 1].clone());
                i += 2;
                continue;
            } else {
                eprintln!("Error: -t requires a <docroot> argument");
                return;
            }
        }

        if args[i] == "--ast" {
            print_ast = true;
            i += 1;
            continue;
        }
        
        if args[i] == "--print-opcodes" {
            std::env::set_var("PRINT_OPCODES", "1");
            i += 1;
            continue;
        }
        
        if file_path.is_empty() {
            file_path = args[i].clone();
            script_args.push(file_path.clone());
        } else {
            script_args.push(args[i].clone());
        }
        i += 1;
    }

    std::env::set_var("HYPERION_ENGINE", &engine_mode);

    if let Some(ref dr) = docroot {
        let _ = std::env::set_current_dir(dr);
    }

    let (file_path, routing_script) = if file_path.is_empty() {
        if server_addr.is_some() || fastcgi_addr.is_some() {
            if std::path::Path::new("index.php").is_file() {
                let canon = std::fs::canonicalize("index.php").map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "index.php".to_string());
                (canon.clone(), Some(canon))
            } else if std::path::Path::new("public/index.php").is_file() {
                let canon = std::fs::canonicalize("public/index.php").map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "public/index.php".to_string());
                (canon.clone(), Some(canon))
            } else {
                ("".to_string(), None)
            }
        } else {
            eprintln!("Error: No script or file provided.");
            return;
        }
    } else {
        let canon = if let Ok(canon) = std::fs::canonicalize(&file_path) {
            canon.to_string_lossy().into_owned()
        } else if let Ok(canon) = std::fs::canonicalize(std::path::Path::new("..").join(&file_path)) {
            canon.to_string_lossy().into_owned()
        } else {
            file_path
        };
        (canon.clone(), Some(canon))
    };

    let engine_state = Arc::new(GlobalEngineState::new());
    
    // Inject core classes like stdClass, Exception, Closure, etc
    engine_state.inject_core_classes();

    if server_addr.is_some() || fastcgi_addr.is_some() {
        let num_cores = std::env::var("PHP_CLI_SERVER_WORKERS")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or_else(|| {
                let avail = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
                avail.max(8)
            });
        let mut scheduler = Scheduler::new(num_cores);
        
        let global_queue = scheduler.global_queue.clone();
        let unparkers = scheduler.unparkers.clone();
        *engine_state.unparkers.write().unwrap() = unparkers.clone();
        
        let idle_fibres = Arc::new(hyperion_vm::io::Injector::new());
        let idle_octane_fibres = Arc::new(hyperion_vm::io::Injector::new());

        let pool_size = std::env::var("HYPERION_MAX_FIBRES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8);

        if !file_path.is_empty() {
            match engine_state.compile_and_load_script(&file_path) {
                Ok(func_ptr) => {
                    if worker_mode {
                        for id in 0..pool_size {
                            let mut f = Fibre::new(2000 + id as u64, func_ptr, engine_state.clone());
                            let res = hyperion_vm::vm::VM::run_fibre(&mut f);
                            if let hyperion_vm::vm::ExecutionResult::Yielded(hyperion_vm::vm::YieldReason::WaitForHttpRequest) = res {
                                f.mark_boot_checkpoint();
                                idle_octane_fibres.push(f);
                            } else {
                                f.reset_for_worker_request(func_ptr);
                                idle_fibres.push(f);
                            }
                        }
                    } else {
                        for id in 0..pool_size {
                            let fibre = Fibre::new(2000 + id as u64, func_ptr, engine_state.clone());
                            idle_fibres.push(fibre);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Fatal error: Failed to compile script '{}': {}", file_path, e);
                }
            }
        }

        let num_reactors = match std::env::var("HYPERION_REACTORS") {
            Ok(v) => v.parse::<usize>().unwrap_or((num_cores / 2).clamp(2, 8)),
            Err(_) => (num_cores / 2).clamp(2, 8),
        };
        let mut reactors = Vec::new();
        
        for r_idx in 0..num_reactors {
            let reactor = Arc::new(Reactor::new_with_idle_fibres(
                r_idx,
                global_queue.clone(), 
                unparkers.clone(), 
                engine_state.clone(), 
                routing_script.clone(),
                worker_mode,
                idle_fibres.clone(),
                idle_octane_fibres.clone(),
            ));
            
            if let Some(addr_str) = &server_addr {
                let addr: std::net::SocketAddr = addr_str.parse().expect("Invalid server address. Use format 127.0.0.1:8000");
                reactor.bind_sapi(addr);
            }

            if let Some(fcgi_str) = &fastcgi_addr {
                let addr: std::net::SocketAddr = fcgi_str.parse().expect("Invalid FastCGI address. Use format 127.0.0.1:9000");
                reactor.bind_fastcgi(addr);
            }
            
            let reactor_clone = Arc::clone(&reactor);
            std::thread::Builder::new()
                .name(format!("hyperion-reactor-{}", r_idx))
                .stack_size(32 * 1024 * 1024)
                .spawn(move || {
                    reactor_clone.run();
                })
                .expect("Failed to spawn reactor thread");
                
            reactors.push(reactor);
        }
        
        // Spawn Workers
        let workers = std::mem::take(&mut scheduler.workers);
        let stealers = scheduler.stealers.clone();
        let global = scheduler.global_queue.clone();
        
        let reactors_arc = Arc::new(reactors);
        for (idx, context) in workers.into_iter().enumerate() {
            let global_clone = global.clone();
            let stealers_clone = stealers.clone();
            let reactors_clone = Arc::clone(&reactors_arc);
            let idle_fibres_clone = idle_fibres.clone();
            
            std::thread::Builder::new()
                .name(format!("hyperion-worker-{}", idx))
                .stack_size(32 * 1024 * 1024)
                .spawn(move || {
                    hyperion_vm::worker::run_worker_loop(
                        context,
                        global_clone,
                        stealers_clone,
                        reactors_clone,
                        idle_fibres_clone,
                    );
                })
                .expect("Failed to spawn worker thread");
        }


        
        // Keep main thread alive forever
        loop {
            thread::sleep(Duration::from_secs(10));
            eprintln!("[SERVER HEARTBEAT] Hyperion server thread alive");
        }

    } else {
        let use_zend = engine_mode == "php84" || engine_mode == "zend" || engine_mode == "php"
            || (engine_mode == "auto" && hyperion_vm::zend_sapi::file_requires_php84(&file_path));

        if use_zend {
            let pass_args = if script_args.len() > 1 { &script_args[1..] } else { &[] };
            match hyperion_vm::zend_sapi::execute_cli(&file_path, pass_args, docroot.as_deref()) {
                Ok(code) => std::process::exit(code),
                Err(e) => {
                    eprintln!("Zend CLI Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        let main_func = match engine_state.compile_and_load_script(&file_path) {
            Ok(f) => f,
            Err(e) => {
                if engine_mode == "auto" {
                    let pass_args = if script_args.len() > 1 { &script_args[1..] } else { &[] };
                    if let Ok(code) = hyperion_vm::zend_sapi::execute_cli(&file_path, pass_args, docroot.as_deref()) {
                        std::process::exit(code);
                    }
                }
                eprintln!("Fatal error: Failed to compile '{}': {}", file_path, e);
                std::process::exit(1);
            }
        };
        if print_ast {
            println!("{:#?}", engine_state.opcache.get(&file_path).map(|x| x.value().last_checked_time.load(std::sync::atomic::Ordering::Relaxed)));
            return;
        }
        let mut fibre = Fibre::new(1, main_func, engine_state.clone());
        fibre.populate_cli_superglobals(&script_args);
        loop {
            let res = hyperion_vm::vm::VM::run_fibre(&mut fibre);
            match res {
                hyperion_vm::vm::ExecutionResult::Finished => {
                    if fibre.frame_count > 0 {
                        // An inner synchronous sub-call finished; pop its return value if any and continue the outer script!
                        continue;
                    }
                    fibre.flush_all_ob_buffers();
                    let _ = std::io::stdout().write_all(&fibre.output_buffer);
                    let _ = std::io::stdout().flush();
                    std::process::exit(0);
                }
                hyperion_vm::vm::ExecutionResult::Error(msg) => {
                    let _ = std::io::stdout().write_all(&fibre.output_buffer);
                    let _ = std::io::stdout().flush();
                    eprintln!("Fatal error: {}", msg);
                    eprintln!("Stack trace:");
                    for i in (0..fibre.frame_count).rev() {
                        let frame = &fibre.frames[i];
                        let func = unsafe { &*frame.function.0 };
                        let class_name = if frame.called_class_id > 0 {
                            if let Some(class) = fibre.engine_state.classes.get(&frame.called_class_id) {
                                format!("{}::", class.value().name)
                            } else {
                                "".to_string()
                            }
                        } else {
                            "".to_string()
                        };
                        eprintln!("#{} {}{}()", i, class_name, func.name);
                    }
                    std::process::exit(1);
                }
                hyperion_vm::vm::ExecutionResult::UncaughtException(e) => {
                    let _ = std::io::stdout().write_all(&fibre.output_buffer);
                    let _ = std::io::stdout().flush();
                    eprintln!("Uncaught exception: {}", e);
                    std::process::exit(1);
                }
                hyperion_vm::vm::ExecutionResult::Yielded(hyperion_vm::vm::YieldReason::AsyncInclude(path)) => {
                    if !fibre.output_buffer.is_empty() {
                        let _ = std::io::stdout().write_all(&fibre.output_buffer);
                        let _ = std::io::stdout().flush();
                        fibre.output_buffer.clear();
                    }
                    if let Err(e) = engine_state.compile_and_load_script(&path) {
                        eprintln!("Fatal error: Failed to require '{}': {}", path, e);
                        std::process::exit(1);
                    }
                    fibre.resume();
                }
                hyperion_vm::vm::ExecutionResult::Yielded(hyperion_vm::vm::YieldReason::OutputFlush) => {
                    let _ = std::io::stdout().write_all(&fibre.output_buffer);
                    let _ = std::io::stdout().flush();
                    fibre.output_buffer.clear();
                    fibre.resume();
                }
                hyperion_vm::vm::ExecutionResult::Yielded(hyperion_vm::vm::YieldReason::Database(target_task_id)) => {
                    loop {
                        if let Some((_, result)) = engine_state.pending_db_results.remove(&target_task_id) {
                            match result {
                                Ok(val) => {
                                    hyperion_vm::vm::VM::push(&mut fibre, val);
                                }
                                Err(err_msg) => {
                                    let _ = hyperion_vm::vm::VM::create_and_throw_native_error(&mut fibre, &err_msg);
                                }
                            }
                            fibre.resume();
                            break;
                        }
                        match engine_state.db_completion_rx.recv() {
                            Ok((task_id, result)) => {
                                if task_id == target_task_id {
                                    match result {
                                        Ok(val) => {
                                            hyperion_vm::vm::VM::push(&mut fibre, val);
                                        }
                                        Err(err_msg) => {
                                            let _ = hyperion_vm::vm::VM::create_and_throw_native_error(&mut fibre, &err_msg);
                                        }
                                    }
                                    fibre.resume();
                                    break;
                                } else {
                                    engine_state.pending_db_results.insert(task_id, result);
                                }
                            }
                            Err(e) => {
                                eprintln!("Fatal error: DB channel disconnected: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                hyperion_vm::vm::ExecutionResult::Preempted => {
                    if !fibre.output_buffer.is_empty() {
                        let _ = std::io::stdout().write_all(&fibre.output_buffer);
                        let _ = std::io::stdout().flush();
                        fibre.output_buffer.clear();
                    }
                    fibre.resume();
                }

                other => {
                    eprintln!("VM exited with: {:?}", other);
                    std::process::exit(1);
                }
            }
        }

    }
}
