# AI Plan: Test Suite
**Goal**: Ensure compliance, safety, and ultimate performance.

## AI Tasks:
1. **Unit Tests (`unit/`)**: Write Rust `#[test]` modules for Core, Compiler, and GC components.
2. **Benchmarks (`benchmarks/`)**: Create test scripts comparing math and array operations against PHP 8 and Node.js.
3. **Framework Compat (`framework_compat/`)**: Automate pulling Laravel/Symfony and running their PHPUnit suites through PHP-H.
Now I can see the issue clearly from the SAPI server log. Let me trace the critical path:

StaticMethodCall: Composer\Autoload\ClassLoader::getRegisteredLoaders is called
It does GetStatic for registeredLoaders - def_exists=true
It returns the value
Then array_keys() is called on it - but it fails because it's not an array
The GetStatic returns the value from default_statics, which exists (def_exists=true), but the value is likely the initial default rather than the mutated runtime value. The problem is that registeredLoaders is initialized as an empty array [] in the PHP class definition, and gets populated at runtime via $registeredLoaders[$vendorDir] = $loader — but the engine is returning the default static (the empty initial value or a wrong type) instead of the runtime-mutated value.

Let me look at how GetStatic works and how static properties are stored/mutated.

Now I can see the critical issue. The GetStatic implementation (line 3247-3256) first checks fibre.static_properties (per-fibre instance), then falls back to default_statics (global defaults).

The problem: In the SAPI server, each HTTP request creates a new Fibre (worker). When the getLoader() method in Composer's autoloader sets self::$registeredLoaders[$vendorDir] = $loader using SetStatic (line 3260-3282), it writes to fibre.static_properties — which is per-fibre local storage. But getRegisteredLoaders() is called later, possibly on the same fibre but in a different execution context where the fibre was reset.

Wait, actually looking more carefully at the debug output from task-6283:

[DEBUG GETSTATIC] class_str='Composer\Autoload\ClassLoader' prop_str='registeredLoaders' resolved_key='("composer\\autoload\\classloader", "registeredLoaders")' def_exists=true
The def_exists=true means the default value exists. The GetStatic code checks:

fibre.static_properties first (line 3247) - if the runtime value was set via SetStatic
Then default_statics (line 3249) - the initial class definition value
The issue is likely that SetStatic is called to set $registeredLoaders[$vendorDir] = $loader but it's doing an ArraySet on the static property, which means:

GetStatic retrieves the initial empty array from default_statics
The mutation happens on the value on the stack
But the mutated value is never written back via SetStatic
This is a classic PHP array mutation issue. In PHP self::$registeredLoaders[$vendorDir] = $loader should translate to:

Get the static property (the array)
Set a key on it
Store it back to the static property
Let me check how this is compiled. Let me look at SetStatic and also how SetStaticArrayIndex or equivalent is implemented.

There's no separate opcode for setting an array element on a static property. Let me check the compiler for how self::$registeredLoaders[$vendorDir] = $loader is compiled.

Now let me look at how assignments like self::$registeredLoaders[$vendorDir] = $loader are compiled. This is an assignment with an array index on a static property.

Now I see StaticPropertySet at line 899, but this only handles self::$prop = value. What about self::$registeredLoaders[$vendorDir] = $loader? This is an array element assignment on a static property. Let me check how the parser handles this.

This is critical! At line 1080-1086, I can see that when the assignment target is an ArrayGet, it becomes an ArraySet. This is correct. But the question is: what happens when you have self::$registeredLoaders[$vendorDir] = $loader?

The parsing flow would be:

Parse self::$registeredLoaders → StaticPropertyGet { class_name: "self", property: "registeredLoaders" }
Parse [$vendorDir] → This wraps into ArrayGet { array: StaticPropertyGet {...}, key: Variable("vendorDir") }
= $loader → The assignment turns the ArrayGet into an ArraySet
So the resulting AST would be:

ArraySet {
    array: StaticPropertyGet { class_name: "self", property: "registeredLoaders" },
    key: Variable("vendorDir"),
    value: Variable("loader")
}
Now let me check how ArraySet is compiled when the array is a StaticPropertyGet:

Now I see the problem! When ArraySet is compiled for self::$registeredLoaders[$vendorDir] = $loader:

compile_expr(*array) → compiles StaticPropertyGet → emits GetStatic → pushes a copy of the array onto the stack
compile_expr(*k) → pushes the key
compile_expr(*value) → pushes the value
ArraySet → modifies the array on the stack
But the problem is: the array on the stack is a copy. The ArraySet opcode mutates the array in-place on the stack, but this mutated value is never written back to the static property. The original static property remains unchanged!

In PHP, self::$registeredLoaders[$vendorDir] = $loader should effectively be:

Get the array from the static property
Set the key-value pair in the array
Write the modified array back to the static property
The current compiled code only does steps 1 and 2, but never does step 3.

Now let me check how ArraySet works in the VM to confirm my theory:

Now I confirm the issue. ArraySet pops the array, modifies it in-place via the raw pointer, and then pushes back only the value (line 962). The array is modified in-place through the pointer, so if the same pointer is stored in the static property, it would be fine.

Wait - actually the key insight here is that ArraySet mutates the array in-place via the raw pointer (line 928: let arr = unsafe { &mut *(arr_ptr as *mut core::types::array::PhpArray) }). So if the static property and the stack both reference the same array pointer, the mutation would be visible through both.

But here's the real issue: When GetStatic retrieves the default value (line 3249-3253), it copies the value from default_statics and stores it into fibre.static_properties. If the default value is an array, the value being copied is a pointer — so modifying through the pointer should work.

Wait, but let me check more carefully. The debug output says:

[DEBUG GETSTATIC] class_str='Composer\Autoload\ClassLoader' prop_str='registeredLoaders' resolved_key='("composer\\autoload\\classloader", "registeredLoaders")' def_exists=true
This is the SECOND call to GetStatic for registeredLoaders — the one inside getRegisteredLoaders(). The first call was during setup when $registeredLoaders[$vendorDir] = $loader was executed. But from the debug logs, I don't see a SetStatic for registeredLoaders.

The issue might be that self::$registeredLoaders[$vendorDir] = $loader is compiled as an ArraySet on a GetStatic, and since ArraySet modifies through the pointer, the array should be updated in place. BUT, the value in default_statics is a Value which is a NaN-boxed representation. Let me check if arrays are stored as pointers (reference semantics) or values.

Let me check how default_statics stores initial array values:

I see the default_statics at line 109 stores default_val which comes from the compiler. The question is: what value is registeredLoaders initialized with?

In the PHP ClassLoader.php, it would be:

php
private static $registeredLoaders = array();
So the compiler would set the default value for registeredLoaders as an empty array. The value stored is a Value which holds a pointer to a PhpArray.

Now here's the critical question: When multiple Fibres (workers) access this default value, they all get the same pointer to the same array object. So mutations through one fibre should be visible to all. But wait - let me re-read the GetStatic flow:

rust
let val = if let Some(v) = fibre.static_properties.get(&key) {
    *v
} else if let Some(def_v) = fibre.engine_state.default_statics.read().unwrap().get(&key) {
    fibre.static_properties.insert(key.clone(), *def_v);
    *def_v
} else {
    Value::null()
};
OK wait. Since the SAPI server spawns a new Fibre for each request, the fibre.static_properties is empty for each new request. So every time:

First GetStatic for registeredLoaders - falls through to default_statics, copies the pointer into fibre.static_properties, returns the pointer
ArraySet - mutates the array in-place through the pointer (adds $vendorDir => $loader)
Second GetStatic for registeredLoaders (in getRegisteredLoaders()) - finds it in fibre.static_properties and returns the same pointer
This should work! The array would be mutated in-place. Unless... the issue is that on a NEW request, a NEW fibre is created, and fibre.static_properties is empty again, so it reads from default_statics again - and the pointer is the same shared one, which was mutated by the previous request. So that should be fine too...

Wait, but look at the debug output more carefully. Let me look at the SAPI server task-6283 log output more carefully. From the log output I already have, the call chain is:

[DEBUG] StaticMethodCall: Composer\Autoload\ClassLoader::getRegisteredLoaders
[DEBUG GETSTATIC] class_str='Composer\Autoload\ClassLoader' prop_str='registeredLoaders' resolved_key='("composer\\autoload\\classloader", "registeredLoaders")' def_exists=true
And then it says array_keys() expects parameter 1 to be array. So the value returned by GetStatic is not an array!

But I didn't see the [DEBUG GETSTATIC VALUE] output line from line 3257. Let me check if there's a second version of GetStatic at line 4314.

OK, the second GetStatic at line 4314 is for the tracing/debug mode - it doesn't actually execute the opcode, just reads the bytes. The real one is at 3224.

Let me now check: when registeredLoaders is first loaded via GetStatic for the purpose of setting an element (the self::$registeredLoaders[$vendorDir] = $loader in getLoader()), is there log output? Let me search through the actual Composer autoload_real.php source to understand how registeredLoaders is set:

Now I can see the issue clearly. Line 399:

php
self::$registeredLoaders[$this->vendorDir] = $this;
This is compiled as:

GetStatic for registeredLoaders → pushes array onto stack
GetProperty for vendorDir on $this → pushes key
Push $this → pushes value
ArraySet → mutates the array in place, pops all three, pushes the value
The array IS mutated in place via the pointer. BUT, there's also line 398:

php
unset(self::$registeredLoaders[$this->vendorDir]);
Let me check how unset for a static array element is handled. Also, importantly, line 396 creates a new array (array($this->vendorDir => $this) + self::$registeredLoaders) and assigns it to the static property. If $prepend is false (likely), then lines 398-399 execute.

But wait - line 399 self::$registeredLoaders[$this->vendorDir] = $this; — this is the ArraySet case. The ArraySet pops the array, mutates it, and pushes the value (not the array). After this, the modified array value in fibre.static_properties still holds the original pointer that was inserted during the first GetStatic call.

Actually wait, let me re-read the ArraySet code:

rust
let value = Self::pop(fibre);
let key = Self::pop(fibre);
let array_val = Self::pop(fibre);
if let Some(arr_ptr) = array_val.as_array_ptr() {
    let arr = unsafe { &mut *(arr_ptr as *mut core::types::array::PhpArray) };
    // ... mutate arr ...
}
// Return the assigned value (assignment is an expression)
Self::push(fibre, value);
The array is popped from the stack and mutated through the pointer. Then only the value is pushed back. So the stack no longer has the array. But the array was mutated in place via the raw pointer, so any reference to the same pointer (like the one in fibre.static_properties) would see the changes.

But wait - the issue is the sequence. Let me trace through what happens with lines 398-399 when $prepend is false:

Line 398: unset(self::$registeredLoaders[$this->vendorDir]);

How is unset for a static array element compiled? Let me check:

Interesting — unset is not implemented in the compiler! The parser doesn't even recognize it as a keyword. This means when the parser encounters unset(...), it might be skipping it or treating it as a function call.

Let me check the native functions to see if unset is handled as a native function:

So unset is completely unimplemented. That's not the immediate cause of the array_keys error though. Let me focus on the real problem.

Actually, wait - the fundamental issue is probably much simpler. Let me look at the debug output from the SAPI log again. The error path is:

[DEBUG] StaticMethodCall: Composer\Autoload\ClassLoader::getRegisteredLoaders
IP: 3, OP: GetStatic
[DEBUG GETSTATIC] ... def_exists=true
IP: 6, OP: ReturnValue
IP: 70, OP: FetchConstant
IP: 72, OP: Call
Worker encountered Execution Error: array_keys() expects parameter 1 to be array
So getRegisteredLoaders() returns self::$registeredLoaders, and then array_keys() is called on the result. Let me look at whether the register() method was even called during the execution, meaning whether registeredLoaders was populated.

But actually, looking at the debug output from the log, the value returned by GetStatic is from default_statics which is an empty array(). That should still be a valid array! array_keys(array()) should return [], not an error.

So the problem might be that the default value for registeredLoaders was not correctly stored as an array in default_statics. Let me check what type it is.

Let me look at the debug print we added. The log shows:

[DEBUG GETSTATIC] class_str='Composer\Autoload\ClassLoader' prop_str='registeredLoaders' resolved_key='("composer\\autoload\\classloader", "registeredLoaders")' def_exists=true
But I don't see [DEBUG GETSTATIC VALUE] output. That's the print at line 3257. This is strange - it should appear right after. Unless the execution took a different code path. Let me check if there's a caching/optimization layer that might bypass the debug print.

Actually, wait — there are TWO implementations of GetStatic. The one at line 3224 (the real one) and the one at line 4314 (the tracing one). Let me check which function they're in:

Let me find the function name containing line 3224:

There's only one run_fibre. Let me check what function contains line 4314:

OK so the second one (line 4314) is in debug_trace, which is separate from the actual execution. Good, so the real execution uses line 3224.

Now, let me focus on why array_keys() gets a non-array. Let me add more detailed debugging. But first, let me understand what value type registeredLoaders actually has.

Looking at the MERGE CLASS log for ClassLoader:

Let me think about this differently. The debug logs show:

[DEBUG GETSTATIC] ... def_exists=true
So the key exists in default_statics. Then either:

The value is in fibre.static_properties (from a previous GetStatic in the same fibre)
Or it falls through to default_statics
The value returned is either the runtime value (if register() was called in the same fibre, setting self::$registeredLoaders[$vendorDir] = $this) or the default value (empty array). In either case, it should be an array.

Let me look at what array_keys() implementation looks like to understand what type check it does:

OK, so array_keys at line 93: if let Some(a) = input — this checks if the input is Some (not None from the macro's argument extraction). And at line 94: if let Some(arr_ptr) = a.as_array_ptr() — this checks if it's an array pointer.

The error array_keys() expects parameter 1 to be array at line 110 means a.as_array_ptr() returned None. So the value passed to array_keys() is not an array.

Now let me look at how the calling code uses getRegisteredLoaders(). Let me check the calling context:

Now the caller. The debug shows the call came from inferBasePath(). Let me see what calls array_keys on the result:

Now I see the exact call: line 259 is array_keys(ClassLoader::getRegisteredLoaders()). So getRegisteredLoaders() must return a valid array, but it returns something that array_keys() rejects.

Let me now check: is the Composer\Autoload\ClassLoader class even properly registered with its static properties? Let me check the [DEBUG MERGE CLASS] log for ClassLoader:

From the debug log output I already have (from task-6283):

[DEBUG MERGE CLASS] Class: Illuminate\Foundation\Application, static_props: [("macros", Array)]
But I don't see a [DEBUG MERGE CLASS] for Composer\Autoload\ClassLoader. This means ClassLoader's class was already loaded before the debug MERGE CLASS prints started appearing.

Let me now check how the static property $registeredLoaders is initially compiled. The ClassLoader class declares:

php
private static $registeredLoaders = array();
The compiler should see this as a static property with a default value of an empty array. Let me check how the compiler processes static property default values:

Line 588: let val = initial_value.as_ref().map(|e| expr_to_literal_value(e, &self.file_path)).unwrap_or(Value::null());

So the default value is computed by expr_to_literal_value. For array(), this should produce an array value. Let me check:

OK so expr_to_literal_value handles Expr::Array(elements) at line 1977 and Expr::Call { callee: array, ... } at line 2008. Both correctly create an empty PhpArray with Box::into_raw(Box::new(arr)) and wrap it in Value::new_array_ptr(ptr as *mut ()).

For private static $registeredLoaders = array();, the parser should produce either Expr::Array([]) or Expr::Call { callee: "array", arguments: [] }. Either way, it produces a valid empty array.

So the default value should be an array. Let me write a targeted debug test to verify. Actually, let me think about this differently.

The real question is: what is the actual Value type being returned by GetStatic? Let me add a more targeted debug print. But first, let me check if the [DEBUG GETSTATIC VALUE] line at 3257 actually shows in the logs. Maybe the output is just truncated.

Actually, I realize the issue might be simpler. Let me re-read the debug output from task-6283 more carefully. The log ends with:

IP: 3, OP: GetStatic
[DEBUG GETSTATIC] class_str='Composer\Autoload\ClassLoader' prop_str='registeredLoaders' resolved_key='("composer\\autoload\\classloader", "registeredLoaders")' def_exists=true
IP: 6, OP: ReturnValue
IP: 70, OP: FetchConstant
IP: 72, OP: Call
Worker encountered Execution Error: array_keys() expects parameter 1 to be array
Note: IP: 6, OP: ReturnValue — this is returning the value from getRegisteredLoaders(). IP: 70, OP: FetchConstant — fetching the array_keys function name. IP: 72, OP: Call — calling array_keys with the returned value.

The [DEBUG GETSTATIC VALUE] print should appear after [DEBUG GETSTATIC]. The fact that it doesn't appear in the log output might just be truncation. But the call to array_keys() errors, so the value is definitely not an array.

Wait — I now notice something. The [DEBUG GETSTATIC VALUE] print is at line 3257, which comes after the value resolution. But in the execution log trace, the IP: lines are what the worker thread prints. The [DEBUG GETSTATIC] print is at line 3243-3244. The [DEBUG GETSTATIC VALUE] print is at line 3257. Both are in the same thread. They should appear in the log.

Unless... the log output was truncated due to length. Let me check the actual log file.

I can confirm: the [DEBUG GETSTATIC VALUE] line at line 3257 does NOT appear in the output even though it's after the [DEBUG GETSTATIC] print. This means the code at line 3257 is not being executed.

Wait, that doesn't make sense. The code flow is:

Line 3243-3244: print [DEBUG GETSTATIC] ✓ (this appears)
Line 3247-3256: resolve the value
Line 3257: print [DEBUG GETSTATIC VALUE] ✗ (this does NOT appear)
Line 3258: Self::push(fibre, val)
If line 3257 doesn't execute, something is panicking or erroring between lines 3244 and 3257. But there's no panic in the output...

Unless the issue is that the output is interleaved across threads and the VALUE debug print went to a different buffer. But that's unlikely with println!.

Actually wait — maybe there's a different Fibre executing this code. Let me look at the SAPI server to see how it creates Fibres:

Let me check how the reactor creates fibres for incoming requests:

I see! At line 181, a new Fibre is created for each HTTP request with ID 1001. This fibre has an empty static_properties HashMap. When GetStatic is called, it falls through to default_statics.

Now let me look at the sequence more carefully. The Fibre 1001 is the HTTP request fibre. But Fibre 1 (the initial autoload fibre) is where the register() method was called, which sets self::$registeredLoaders[$vendorDir] = $this.

The SAPI server creates a new Fibre for each request. The new Fibre 1001 has empty static_properties. When GetStatic is called on Fibre 1001, it goes to default_statics. The default_statics is shared (global), and it was set at compile time with the default empty array.

But wait - register() is called during Fibre 1's execution (the initial autoload), which calls self::$registeredLoaders[$vendorDir] = $this. This is compiled as:

GetStatic for registeredLoaders → gets the default empty array from default_statics, copies to fibre.static_properties
GetProperty for vendorDir on $this
Push $this
ArraySet → mutates the array in-place through the pointer
Since the array is mutated in-place through the raw pointer, the mutation affects the same array that's stored in default_statics. So when Fibre 1001 later accesses default_statics, it should see the mutated array with the loader entry.

Unless... wait. Let me re-check. Is GetStatic operating on the value (which is a NaN-boxed u64 containing a pointer), or is the pointer being copied? If the Value is just a u64 that encodes a pointer, then both default_statics and fibre.static_properties store the same pointer, so mutations through the pointer should be visible everywhere. That should work.

But the error says array_keys() expects parameter 1 to be array. This means the value is NOT an array. Let me now check if maybe the issue is that registeredLoaders default value wasn't set correctly because the ClassLoader class has multiple static properties and maybe some naming issue.

Let me check all the static properties of ClassLoader:

$registeredLoaders is the only static property. All others are instance properties (without static). So the static_properties list for ClassLoader should only have one entry: ("registeredLoaders", Value::array(...)).

Now, wait. There's another static property further down:

Line 46: private static $includeFile; — this has no initial value, so expr_to_literal_value would return Value::null() (the unwrap_or at line 588 of compiler.rs).

So the default_statics should have:

("composer\\autoload\\classloader", "includeFile") → Value::null()
("composer\\autoload\\classloader", "registeredLoaders") → Value::array(empty)
Let me now add a very specific debug print to understand the actual type of the value being returned. I'll also check if the find_static_property_key function is resolving the correct key. Let me update the GetStatic debug to print the actual value type:

Hmm, the debug print at line 3257 should have printed but it didn't. Let me check if maybe the code at line 3240 is deadlocking? The code at line 3240 acquires classes.read(), and at line 3241 acquires default_statics.read(). Then at line 3246, those are dropped. Then at line 3250, default_statics.read() is acquired again. That should be fine.

Wait — maybe the issue is simpler. Maybe the GetStatic implementation on Fibre 1001 is NOT the one at line 3224. The log shows Worker executing Fibre 1001 and then the opcodes. Let me check whether there's another VM execution path for the SAPI worker.

Good - at line 40, the worker calls VM::run_fibre(&mut fibre) which is the same function at line 313 of vm.rs. So the GetStatic at line 3224 IS the one being executed.

So why doesn't [DEBUG GETSTATIC VALUE] appear? Let me check if maybe there's an issue with buffering. Or perhaps the print is happening but the log capture is truncated.

Actually, wait. Let me look at this differently. The log shows the output ended at Worker encountered Execution Error: array_keys() expects parameter 1 to be array. The [DEBUG GETSTATIC VALUE] print at line 3257 should come BEFORE the error, since it's within the GetStatic opcode handler. If it doesn't appear, it means the VALUE print line is somehow never reached, or it was flushed but the log capture missed it.

Let me try a different approach - I'll write a minimal test script that reproduces the issue and add better logging to understand the type:

Wait, I think I need to check whether the println! output is actually flushed. In Rust, println! flushes to stdout. But maybe in a threaded environment, stdout might not show everything if the process is killed.

Actually, I just realized something. Let me look at the log output more carefully:

IP: 3, OP: GetStatic
[DEBUG GETSTATIC] class_str='Composer\Autoload\ClassLoader' prop_str='registeredLoaders' resolved_key='("composer\\autoload\\classloader", "registeredLoaders")' def_exists=true
IP: 6, OP: ReturnValue
The IP: debug prints are ALSO coming from somewhere. Let me find where those are generated:

OK! So the IP/OP trace prints are at line 329, which runs for every opcode. These are very verbose. The [DEBUG GETSTATIC VALUE] line at 3257 should appear between IP: 3, OP: GetStatic and IP: 6, OP: ReturnValue.

Wait, I just realized: the output says:

IP: 3, OP: GetStatic
[DEBUG GETSTATIC] ...
IP: 6, OP: ReturnValue
The [DEBUG GETSTATIC VALUE] is missing between these two lines. But the code should execute it. Unless... there's a panic or deadlock between lines 3244 and 3257.

Actually, wait. Let me re-examine the code flow:

Line 3240: fibre.engine_state.classes.read().unwrap() — acquires read lock on classes
Line 3241: fibre.engine_state.default_statics.read().unwrap() — acquires read lock on default_statics
Line 3245: Block ends, both locks are dropped
Line 3247-3256: value resolution — fibre.static_properties.get() or fibre.engine_state.default_statics.read().unwrap().get()
Hold on — at line 3250, if the key is NOT in fibre.static_properties, it acquires default_statics.read() again. This should be fine since the first lock was dropped at line 3245.

But what about Fibre 1001? Is there a GetStatic call for registeredLoaders on Fibre 1 (the autoload fibre) that's holding a lock? That shouldn't be an issue since RwLock allows multiple readers.

Actually, wait — I just realized something. The println! at line 3257 uses val.get_type(). Maybe get_type() is panicking? Let me check:


php-hyperion $ curl -i http://127.0.0.1:8080/scratch/laravel/public/index.php
  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                 Dload  Upload   Total   Spent    Left  Speed

  0     0    0     0    0     0      0      0 --:--:-- --:--:-- --:--:--     0
  0     0    0     0    0     0      0      0 --:--:--  0:00:01 --:--:--     0
100    44    0    44    0     0     39      0 --:--:--  0:00:01 --:--:--    39
HTTP/1.1 500 Internal Server Error

array_keys() expects parameter 1 to be array