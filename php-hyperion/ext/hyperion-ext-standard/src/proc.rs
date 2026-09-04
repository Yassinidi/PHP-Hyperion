use hyperion_core::types::array::PhpArray;
use hyperion_core::types::function::NativeContext;
use hyperion_core::types::reference::PhpRef;
use hyperion_core::types::resource::{PhpProcess, PhpResource, ResourceKind};
use hyperion_core::memory::nan_box::Value;
use std::io::Write;
use std::process::{Command, Stdio};

fn make_resource(kind: ResourceKind, uri: &str) -> Value {
    let r = PhpResource::new(kind, uri);
    let ptr = crate::into_raw(Box::new(r));
    Value::new_resource_ptr(ptr as *mut ())
}

fn write_out_var(arg: &Value, new_val: Value) {
    if let Some(ref_ptr) = arg.as_ref_ptr() {
        let r = unsafe { &mut *(ref_ptr as *mut PhpRef) };
        r.set(new_val);
    } else if let Some(target_arr_ptr) = arg.as_array_ptr() {
        if let Some(new_arr_ptr) = new_val.as_array_ptr() {
            let target = unsafe { &mut *(target_arr_ptr as *mut PhpArray) };
            let src = unsafe { &*(new_arr_ptr as *const PhpArray) };
            target.elements = src.elements.clone();
        }
    }
}

pub fn native_proc_open(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("proc_open() expects at least 3 parameters".to_string());
    }

    let cmd_val = args[0].deref();
    let descriptorspec_val = args[1].deref();

    // 1. Build Command
    let mut cmd = if let Some(str_ptr) = cmd_val.as_string_ptr() {
        let cmd_str = unsafe { &*(str_ptr as *const String) };
        #[cfg(windows)]
        let mut c = {
            let mut c = Command::new("cmd");
            c.args(&["/c", cmd_str]);
            c
        };
        #[cfg(not(windows))]
        let c = {
            let mut c = Command::new("/bin/sh");
            c.args(&["-c", cmd_str]);
            c
        };
        c
    } else if let Some(arr_ptr) = cmd_val.as_array_ptr() {
        let arr = unsafe { &*(arr_ptr as *const PhpArray) };
        let mut items = Vec::new();
        for (_, v) in arr.elements.iter() {
            let vd = v.deref();
            if let Some(sp) = vd.as_string_ptr() {
                items.push(unsafe { (*(sp as *const String)).clone() });
            }
        }
        if items.is_empty() {
            return Ok(Value::new_bool(false));
        }
        let mut c = Command::new(&items[0]);
        if items.len() > 1 {
            c.args(&items[1..]);
        }
        c
    } else {
        return Ok(Value::new_bool(false));
    };

    // 2. Set CWD if provided
    if args.len() > 3 {
        let cwd_val = args[3].deref();
        if let Some(sp) = cwd_val.as_string_ptr() {
            let cwd = unsafe { &*(sp as *const String) };
            if !cwd.is_empty() {
                cmd.current_dir(cwd);
            }
        }
    }

    // 3. Set ENV if provided
    if args.len() > 4 {
        let env_val = args[4].deref();
        if let Some(arr_ptr) = env_val.as_array_ptr() {
            let arr = unsafe { &*(arr_ptr as *const PhpArray) };
            for (k, v) in arr.elements.iter() {
                let vd = v.deref();
                if let Some(val_str_ptr) = vd.as_string_ptr() {
                    let val_str = unsafe { &*(val_str_ptr as *const String) };
                    let key_str = match k {
                        hyperion_core::types::array::ArrayKey::StringId(id) => {
                            hyperion_core::types::string_table::lookup_string(*id).unwrap_or_else(|| format!("{}", id))
                        }
                        hyperion_core::types::array::ArrayKey::Int(i) => i.to_string(),
                    };
                    cmd.env(key_str, val_str);
                }
            }
        }
    }

    // 4. Configure Stdio descriptors
    let mut stdin_piped = false;
    let mut stdout_piped = false;
    let mut stderr_piped = false;

    if let Some(desc_ptr) = descriptorspec_val.as_array_ptr() {
        let desc_arr = unsafe { &*(desc_ptr as *const PhpArray) };
        for (k, v) in desc_arr.elements.iter() {
            let idx = match k {
                hyperion_core::types::array::ArrayKey::Int(i) => *i,
                _ => continue,
            };
            let vd = v.deref();
            if let Some(spec_arr_ptr) = vd.as_array_ptr() {
                let spec = unsafe { &*(spec_arr_ptr as *const PhpArray) };
                let type_name = spec.get_int(0).and_then(|t| {
                    let td = t.deref();
                    td.as_string_ptr().map(|p| unsafe { (*(p as *const String)).to_lowercase() })
                }).unwrap_or_default();

                match idx {
                    0 => {
                        if type_name == "pipe" || type_name == "pty" {
                            cmd.stdin(Stdio::piped());
                            stdin_piped = true;
                        } else if type_name == "file" {
                            if let Some(path_val) = spec.get_int(1) {
                                if let Some(path_ptr) = path_val.deref().as_string_ptr() {
                                    let path = unsafe { &*(path_ptr as *const String) };
                                    if let Ok(file) = std::fs::File::open(path) {
                                        cmd.stdin(Stdio::from(file));
                                    }
                                }
                            }
                        } else if type_name == "null" {
                            cmd.stdin(Stdio::null());
                        }
                    }
                    1 => {
                        if type_name == "pipe" || type_name == "pty" {
                            cmd.stdout(Stdio::piped());
                            stdout_piped = true;
                        } else if type_name == "file" {
                            if let Some(path_val) = spec.get_int(1) {
                                if let Some(path_ptr) = path_val.deref().as_string_ptr() {
                                    let path = unsafe { &*(path_ptr as *const String) };
                                    let mode = spec.get_int(2).and_then(|m| m.deref().as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() })).unwrap_or_else(|| "w".to_string());
                                    let file_res = if mode.contains('a') {
                                        std::fs::OpenOptions::new().create(true).append(true).open(path)
                                    } else {
                                        std::fs::File::create(path)
                                    };
                                    if let Ok(file) = file_res {
                                        cmd.stdout(Stdio::from(file));
                                    }
                                }
                            }
                        } else if type_name == "null" {
                            cmd.stdout(Stdio::null());
                        }
                    }
                    2 => {
                        if type_name == "pipe" || type_name == "pty" {
                            cmd.stderr(Stdio::piped());
                            stderr_piped = true;
                        } else if type_name == "file" {
                            if let Some(path_val) = spec.get_int(1) {
                                if let Some(path_ptr) = path_val.deref().as_string_ptr() {
                                    let path = unsafe { &*(path_ptr as *const String) };
                                    let mode = spec.get_int(2).and_then(|m| m.deref().as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() })).unwrap_or_else(|| "w".to_string());
                                    let file_res = if mode.contains('a') {
                                        std::fs::OpenOptions::new().create(true).append(true).open(path)
                                    } else {
                                        std::fs::File::create(path)
                                    };
                                    if let Ok(file) = file_res {
                                        cmd.stderr(Stdio::from(file));
                                    }
                                }
                            }
                        } else if type_name == "null" {
                            cmd.stderr(Stdio::null());
                        }
                    }
                    _ => {}
                }
            } else if let Some(res_ptr) = vd.as_resource_ptr() {
                let r = unsafe { &*(res_ptr as *const PhpResource) };
                if r.uri == "/dev/null" || r.uri == "php://temp" || r.uri == "php://memory" {
                    match idx {
                        0 => { cmd.stdin(Stdio::null()); }
                        1 => { cmd.stdout(Stdio::null()); }
                        2 => { cmd.stderr(Stdio::null()); }
                        _ => {}
                    }
                }
            }
        }
    } else {
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        stdin_piped = true;
        stdout_piped = true;
        stderr_piped = true;
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return Ok(Value::new_bool(false)),
    };

    // 5. Populate $pipes array
    let mut pipes_arr = PhpArray::new();
    if stdin_piped {
        if let Some(stdin) = child.stdin.take() {
            let res = make_resource(ResourceKind::PipeStdin(stdin), "php://fd/0");
            pipes_arr.insert_int(0, res);
        }
    }
    if stdout_piped {
        if let Some(stdout) = child.stdout.take() {
            let res = make_resource(ResourceKind::PipeStdout(stdout), "php://fd/1");
            pipes_arr.insert_int(1, res);
        }
    }
    if stderr_piped {
        if let Some(stderr) = child.stderr.take() {
            let res = make_resource(ResourceKind::PipeStderr(stderr), "php://fd/2");
            pipes_arr.insert_int(2, res);
        }
    }

    let pipes_val = Value::new_array_ptr(crate::into_raw(Box::new(pipes_arr)) as *mut ());
    write_out_var(&args[2], pipes_val);

    let proc = PhpProcess {
        command: "process".to_string(),
        child,
        exit_code: None,
        running: true,
    };
    let proc_res = make_resource(ResourceKind::Process(Box::new(proc)), "process");
    Ok(proc_res)
}

pub fn native_proc_get_status(
    args: &[Value],
    ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("proc_get_status() expects exactly 1 parameter, 0 given".to_string());
    }
    let res_val = args[0].deref();
    let Some(res_ptr) = res_val.as_resource_ptr() else {
        return Ok(Value::new_bool(false));
    };
    let r = unsafe { &mut *(res_ptr as *mut PhpResource) };
    let ResourceKind::Process(proc) = &mut r.kind else {
        return Ok(Value::new_bool(false));
    };

    if proc.running {
        match proc.child.try_wait() {
            Ok(Some(status)) => {
                proc.running = false;
                proc.exit_code = status.code();
            }
            Ok(None) => {
                proc.running = true;
            }
            Err(_) => {
                proc.running = false;
            }
        }
    }

    let mut arr = PhpArray::new();
    let mut put = |k: &str, v: Value| {
        let id = ctx.intern_string(k);
        arr.insert_string_id(id, v);
    };

    put("command", Value::new_string_ptr(crate::into_raw(Box::new(proc.command.clone())) as *mut ()));
    put("pid", Value::new_int(proc.child.id() as i32));
    put("running", Value::new_bool(proc.running));
    put("signaled", Value::new_bool(false));
    put("stopped", Value::new_bool(false));
    put("exitcode", Value::new_int(if proc.running { -1 } else { proc.exit_code.unwrap_or(0) }));
    put("termsig", Value::new_int(0));
    put("stopsig", Value::new_int(0));

    Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
}

pub fn native_proc_terminate(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("proc_terminate() expects at least 1 parameter".to_string());
    }
    let res_val = args[0].deref();
    let Some(res_ptr) = res_val.as_resource_ptr() else {
        return Ok(Value::new_bool(false));
    };
    let r = unsafe { &mut *(res_ptr as *mut PhpResource) };
    let ResourceKind::Process(proc) = &mut r.kind else {
        return Ok(Value::new_bool(false));
    };

    let signal = if args.len() > 1 {
        args[1].deref().as_int().unwrap_or(15)
    } else {
        15
    };

    #[cfg(unix)]
    {
        let pid = proc.child.id() as i32;
        let _ = Command::new("kill").args(&[format!("-{}", signal), pid.to_string()]).status();
    }
    #[cfg(not(unix))]
    {
        let _ = proc.child.kill();
    }

    Ok(Value::new_bool(true))
}

pub fn native_proc_close(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("proc_close() expects exactly 1 parameter".to_string());
    }
    let res_val = args[0].deref();
    let Some(res_ptr) = res_val.as_resource_ptr() else {
        return Ok(Value::new_int(-1));
    };
    let r = unsafe { &mut *(res_ptr as *mut PhpResource) };
    let exit_code = match &mut r.kind {
        ResourceKind::Process(proc) => {
            if proc.running {
                let code = proc.child.wait().ok().and_then(|s| s.code()).unwrap_or(0);
                proc.running = false;
                proc.exit_code = Some(code);
                code
            } else {
                proc.exit_code.unwrap_or(0)
            }
        }
        _ => -1,
    };
    r.kind = ResourceKind::Closed;
    Ok(Value::new_int(exit_code))
}

pub fn native_exec(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("exec() expects at least 1 parameter".to_string());
    }
    let cmd_val = args[0].deref();
    let Some(sp) = cmd_val.as_string_ptr() else {
        return Ok(Value::new_bool(false));
    };
    let cmd_str = unsafe { &*(sp as *const String) };

    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(&["/c", cmd_str]);
        c
    };

    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("/bin/sh");
        c.args(&["-c", cmd_str]);
        c
    };

    let output = match cmd.output() {
        Ok(out) => out,
        Err(_) => return Ok(Value::new_bool(false)),
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout_str.lines().collect();

    if args.len() > 1 {
        let mut arr = PhpArray::new();
        for (i, line) in lines.iter().enumerate() {
            let line_val = Value::new_string_ptr(crate::into_raw(Box::new(line.to_string())) as *mut ());
            arr.insert_int(i as i64, line_val);
        }
        let arr_val = Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ());
        write_out_var(&args[1], arr_val);
    }

    if args.len() > 2 {
        let code = output.status.code().unwrap_or(0);
        write_out_var(&args[2], Value::new_int(code));
    }

    let last_line = lines.last().copied().unwrap_or("").to_string();
    Ok(Value::new_string_ptr(crate::into_raw(Box::new(last_line)) as *mut ()))
}

pub fn native_shell_exec(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("shell_exec() expects exactly 1 parameter".to_string());
    }
    let cmd_val = args[0].deref();
    let Some(sp) = cmd_val.as_string_ptr() else {
        return Ok(Value::null());
    };
    let cmd_str = unsafe { &*(sp as *const String) };

    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(&["/c", cmd_str]);
        c
    };

    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("/bin/sh");
        c.args(&["-c", cmd_str]);
        c
    };

    let output = match cmd.output() {
        Ok(out) => out,
        Err(_) => return Ok(Value::null()),
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout_str.is_empty() {
        Ok(Value::null())
    } else {
        Ok(Value::new_string_ptr(crate::into_raw(Box::new(stdout_str)) as *mut ()))
    }
}

pub fn native_passthru(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("passthru() expects at least 1 parameter".to_string());
    }
    let cmd_val = args[0].deref();
    let Some(sp) = cmd_val.as_string_ptr() else {
        return Ok(Value::null());
    };
    let cmd_str = unsafe { &*(sp as *const String) };

    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(&["/c", cmd_str]);
        c
    };

    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("/bin/sh");
        c.args(&["-c", cmd_str]);
        c
    };

    let output = match cmd.output() {
        Ok(out) => out,
        Err(_) => return Ok(Value::null()),
    };

    let _ = std::io::stdout().write_all(&output.stdout);
    let _ = std::io::stdout().flush();

    if args.len() > 1 {
        let code = output.status.code().unwrap_or(0);
        write_out_var(&args[1], Value::new_int(code));
    }

    Ok(Value::null())
}

pub fn native_system(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("system() expects at least 1 parameter".to_string());
    }
    let cmd_val = args[0].deref();
    let Some(sp) = cmd_val.as_string_ptr() else {
        return Ok(Value::new_bool(false));
    };
    let cmd_str = unsafe { &*(sp as *const String) };

    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(&["/c", cmd_str]);
        c
    };

    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("/bin/sh");
        c.args(&["-c", cmd_str]);
        c
    };

    let output = match cmd.output() {
        Ok(out) => out,
        Err(_) => return Ok(Value::new_bool(false)),
    };

    let _ = std::io::stdout().write_all(&output.stdout);
    let _ = std::io::stdout().flush();

    if args.len() > 1 {
        let code = output.status.code().unwrap_or(0);
        write_out_var(&args[1], Value::new_int(code));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let last_line = stdout_str.lines().last().unwrap_or("").to_string();
    Ok(Value::new_string_ptr(crate::into_raw(Box::new(last_line)) as *mut ()))
}

pub fn native_popen(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("popen() expects exactly 2 parameters".to_string());
    }
    let cmd_val = args[0].deref();
    let mode_val = args[1].deref();

    let Some(cmd_ptr) = cmd_val.as_string_ptr() else {
        return Ok(Value::new_bool(false));
    };
    let cmd_str = unsafe { &*(cmd_ptr as *const String) };
    let mode_str = mode_val.as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_else(|| "r".to_string());

    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(&["/c", cmd_str]);
        c
    };

    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("/bin/sh");
        c.args(&["-c", cmd_str]);
        c
    };

    if mode_str.contains('w') {
        cmd.stdin(Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => return Ok(Value::new_bool(false)),
        };
        let stdin = match child.stdin.take() {
            Some(s) => s,
            None => return Ok(Value::new_bool(false)),
        };
        let res = make_resource(ResourceKind::PopenWrite(child, stdin), cmd_str);
        Ok(res)
    } else {
        cmd.stdout(Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => return Ok(Value::new_bool(false)),
        };
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return Ok(Value::new_bool(false)),
        };
        let res = make_resource(ResourceKind::PopenRead(child, stdout), cmd_str);
        Ok(res)
    }
}

pub fn native_pclose(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("pclose() expects exactly 1 parameter".to_string());
    }
    let res_val = args[0].deref();
    let Some(res_ptr) = res_val.as_resource_ptr() else {
        return Ok(Value::new_int(-1));
    };
    let r = unsafe { &mut *(res_ptr as *mut PhpResource) };
    let code = match &mut r.kind {
        ResourceKind::PopenRead(child, _) => {
            child.wait().ok().and_then(|s| s.code()).unwrap_or(0)
        }
        ResourceKind::PopenWrite(child, _) => {
            child.wait().ok().and_then(|s| s.code()).unwrap_or(0)
        }
        _ => -1,
    };
    r.kind = ResourceKind::Closed;
    Ok(Value::new_int(code))
}

pub fn native_stream_select(
    args: &[Value],
    _ctx: &mut dyn NativeContext,
) -> Result<Value, String> {
    if args.len() < 4 {
        return Err("stream_select() expects at least 4 parameters".to_string());
    }

    let mut count = 0;
    if let Some(arr_ptr) = args[0].deref().as_array_ptr() {
        let arr = unsafe { &*(arr_ptr as *const PhpArray) };
        count += arr.elements.len();
    }
    if let Some(arr_ptr) = args[1].deref().as_array_ptr() {
        let arr = unsafe { &*(arr_ptr as *const PhpArray) };
        count += arr.elements.len();
    }

    let sec = args[3].deref().as_int().unwrap_or(0);
    let usec = if args.len() > 4 {
        args[4].deref().as_int().unwrap_or(0)
    } else {
        0
    };
    if count == 0 && (sec > 0 || usec > 0) {
        std::thread::sleep(std::time::Duration::from_micros((sec as u64 * 1_000_000) + usec as u64));
    }

    Ok(Value::new_int(count as i32))
}
