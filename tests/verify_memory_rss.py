import subprocess
import time
import urllib.request
import os
import signal

PORT = 8999
CLI_PATH = os.path.abspath("php-hyperion/target/release/hyperion-cli")
SCRIPT_PATH = os.path.abspath("tests/memory_stress.php")
WORKER_SCRIPT_PATH = os.path.abspath("tests/memory_stress_worker.php")

def get_rss_kb(pid):
    try:
        out = subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)]).decode().strip()
        return int(out)
    except Exception:
        return 0

def run_test(mode_name, cmd_args, port=PORT, num_requests=1000):
    print(f"\n==========================================")
    print(f"Starting test: {mode_name} ({num_requests} requests)")
    print(f"Command: {' '.join(cmd_args)}")
    
    # Use DEVNULL to prevent OS pipe buffer deadlocks over 1,000+ requests
    proc = subprocess.Popen(
        cmd_args,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        preexec_fn=os.setsid
    )
    
    time.sleep(1.0)
    
    # Check if process is alive
    if proc.poll() is not None:
        print("Failed to start server!")
        return False
        
    pid = proc.pid
    rss_initial = get_rss_kb(pid)
    print(f"Initial RSS: {rss_initial / 1024:.2f} MB")
    
    # Warm up with 200 requests so all 8 worker fibres reach steady state
    url = f"http://127.0.0.1:{port}/"
    for i in range(200):
        try:
            with urllib.request.urlopen(url, timeout=2) as resp:
                resp.read()
        except Exception as e:
            print(f"Warmup error on request {i}: {e}")
            break
            
    time.sleep(0.5)
    rss_after_warmup = get_rss_kb(pid)
    print(f"RSS after warmup (200 reqs): {rss_after_warmup / 1024:.2f} MB")
    
    # Now run repeated requests and sample RSS every 100 requests
    samples = []
    failed_reqs = 0
    start_time = time.time()
    
    for i in range(1, num_requests + 1):
        try:
            with urllib.request.urlopen(url, timeout=2) as resp:
                data = resp.read()
                if b"OK" not in data:
                    failed_reqs += 1
        except Exception as e:
            failed_reqs += 1
            if i % 200 == 0:
                print(f"Request {i} failed: {e}")
                
        if i % 100 == 0 or i == num_requests:
            rss = get_rss_kb(pid)
            samples.append((i, rss / 1024))
            print(f"Req {i:4d} | RSS: {rss / 1024:.2f} MB | Delta from warmup: {(rss - rss_after_warmup) / 1024:+.2f} MB")
            
    elapsed = time.time() - start_time
    rps = num_requests / elapsed if elapsed > 0 else 0
    print(f"Completed {num_requests} requests in {elapsed:.2f}s ({rps:.0f} req/s). Failed reqs: {failed_reqs}")
    
    # Kill the server
    try:
        os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
        proc.wait(timeout=2)
    except Exception:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except Exception:
            pass

    final_rss = samples[-1][1]
    warmup_rss = rss_after_warmup / 1024
    growth = final_rss - warmup_rss
    print(f"Summary: Warmup RSS={warmup_rss:.2f} MB, Final RSS={final_rss:.2f} MB, Growth={growth:+.2f} MB")
    
    # Pass condition: growth across 1,000 requests is < 6 MB (previously grew 50+ MB per request!)
    if abs(growth) < 6.0 and failed_reqs == 0:
        print(">>> SUCCESS: Zero unbounded RSS growth verified! <<<")
        return True
    else:
        print(f">>> FAILED: Growth was {growth:+.2f} MB or failed_reqs={failed_reqs} <<<")
        return False

if __name__ == "__main__":
    # Test 1: Standard server (-S)
    cmd_s = [CLI_PATH, "-S", f"127.0.0.1:8999", SCRIPT_PATH]
    success_s = run_test("Development Server (-S)", cmd_s, port=8999, num_requests=1000)
    
    if not success_s:
        exit(1)
        
    time.sleep(2.0)
    
    # Test 2: High Performance Worker (-W)
    cmd_w = [CLI_PATH, "-S", f"127.0.0.1:8998", "-W", WORKER_SCRIPT_PATH]
    success_w = run_test("Worker Engine (-W)", cmd_w, port=8998, num_requests=2000)
    
    if not success_w:
        exit(1)
    
    print("\n==========================================")
    print("ALL MEMORY LEAK TESTS PASSED (Both -S and -W)!")
    print("==========================================")
