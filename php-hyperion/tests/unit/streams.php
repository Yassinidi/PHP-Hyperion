<?php
// Streams and the `resource` type. Run with:
//   ./target/release/hyperion-cli tests/unit/streams.php

function check($label, $got, $want) {
    if ($got === $want) {
        echo "PASS " . $label . "\n";
    } else {
        echo "FAIL " . $label . " got=" . var_export($got, true)
           . " want=" . var_export($want, true) . "\n";
    }
}

// ------------------------------------------------------------- the type itself
$h = fopen('php://memory', 'w+');
check("fopen returns a resource", is_resource($h), true);
check("resource is not an object", is_object($h), false);
check("resource is not an int", is_int($h), false);
check("gettype says resource", gettype($h), "resource");
check("get_resource_type", get_resource_type($h), "stream");
check("resource is truthy", (bool) $h, true);
check("resource identity", $h === $h, true);

// This is the exact guard Symfony's StreamOutput::__construct applies.
check("is_resource rejects an int", is_resource(99), false);
check("is_resource rejects a string", is_resource('php://stdout'), false);
check("is_resource rejects null", is_resource(null), false);

// -------------------------------------------------------------- memory streams
check("fwrite returns byte count", fwrite($h, "hello\nworld\n"), 12);
check("ftell after write", ftell($h), 12);
check("rewind", rewind($h), true);
check("ftell after rewind", ftell($h), 0);
check("fread partial", fread($h, 5), "hello");
check("fgets to newline", fgets($h), "\n");
check("fgets next line", fgets($h), "world\n");
// PHP only raises EOF after a read comes up short, not on exact consumption.
check("fgets at end is false", fgets($h), false);
check("feof after short read", feof($h), true);

rewind($h);
check("stream_get_contents reads rest", stream_get_contents($h), "hello\nworld\n");

// fseek returns 0 on success, not true.
check("fseek returns 0", fseek($h, 6), 0);
check("read from sought position", fread($h, 5), "world");
check("fseek SEEK_END", fseek($h, 0, SEEK_END), 0);
check("ftell at end", ftell($h), 12);
check("fseek SEEK_CUR back", fseek($h, -6, SEEK_CUR), 0);
check("ftell after relative seek", ftell($h), 6);

check("ftruncate", ftruncate($h, 5), true);
rewind($h);
check("content after truncate", stream_get_contents($h), "hello");

check("fgetc", (function () {
    $m = fopen('php://memory', 'w+');
    fwrite($m, "xy");
    rewind($m);
    return fgetc($m);
})(), "x");

check("fwrite honors length arg", (function () {
    $m = fopen('php://memory', 'w+');
    $n = fwrite($m, "abcdef", 3);
    rewind($m);
    return [$n, stream_get_contents($m)];
})(), [3, "abc"]);

check("write past end zero-fills", (function () {
    $m = fopen('php://memory', 'w+');
    fwrite($m, "ab");
    fseek($m, 4);
    fwrite($m, "z");
    rewind($m);
    return strlen(stream_get_contents($m));
})(), 5);

// ------------------------------------------------------------------- fclose
check("fclose returns true", fclose($h), true);
check("closed handle is not a resource", is_resource($h), false);
check("fclose on closed handle", fclose($h), false);
check("fwrite on closed handle", fwrite($h, "x"), false);
check("get_resource_type on closed", get_resource_type($h), "Unknown");

// --------------------------------------------------------------- real files
$tmp = sys_get_temp_dir() . '/hyperion_stream_probe.txt';
$f = fopen($tmp, 'w');
check("fopen file for write", is_resource($f), true);
check("write to file", fwrite($f, "line1\nline2\n"), 12);
fclose($f);
check("file_get_contents sees it", file_get_contents($tmp), "line1\nline2\n");

$f = fopen($tmp, 'r');
check("fgets line 1", fgets($f), "line1\n");
check("fgets line 2", fgets($f), "line2\n");
check("fgets past end", fgets($f), false);
check("feof on file", feof($f), true);
fclose($f);

$f = fopen($tmp, 'a');
fwrite($f, "line3\n");
fclose($f);
check("append mode", file_get_contents($tmp), "line1\nline2\nline3\n");

check("fopen missing file is false", fopen($tmp . '.nope', 'r'), false);
check("fopen bad mode is false", fopen($tmp, 'q'), false);
unlink($tmp);

// ----------------------------------------------------------- standard streams
check("STDOUT is a resource", is_resource(STDOUT), true);
check("STDERR is a resource", is_resource(STDERR), true);
check("STDIN is a resource", is_resource(STDIN), true);
// One handle per stream for the life of the engine, so identity holds.
check("STDOUT is a single handle", STDOUT === STDOUT, true);
check("STDOUT is not STDERR", STDOUT === STDERR, false);

// A write to stdout must interleave with echo, not jump ahead of it — the
// buffer the SAPI flushes is the same one `echo` appends to.
echo "A";
fwrite(STDOUT, "B");
echo "C";
echo "\n";
check("interleaving marker emitted above", true, true);

$out = fopen('php://stdout', 'w');
check("php://stdout is a resource", is_resource($out), true);
check("php://stderr is a resource", is_resource(fopen('php://stderr', 'w')), true);

// -------------------------------------------------------------- meta / stat
$m = fopen('php://memory', 'w+');
fwrite($m, "12345");
$meta = stream_get_meta_data($m);
check("meta uri", $meta['uri'], "php://memory");
check("meta seekable", $meta['seekable'], true);
$st = fstat($m);
check("fstat size", $st['size'], 5);
check("fstat numeric size", $st[7], 5);
fclose($m);

// stream_set_blocking has no meaning for these kinds but must not fail.
check("stream_set_blocking", stream_set_blocking(STDOUT, false), true);

echo "\n-- end of suite --\n";
