<?php
// Tests for PHP PCRE module (preg_* functions).
// Validated against system PHP 8.4.

$failures = 0;
function check($label, $got, $want) {
    global $failures;
    if ($got === $want) {
        echo "PASS " . $label . "\n";
    } else {
        $failures++;
        echo "FAIL " . $label . "\n";
        echo "  got:  " . var_export($got, true) . "\n";
        echo "  want: " . var_export($want, true) . "\n";
    }
}

// ------------------------------------------------------------- 1. Constants
check("PREG_PATTERN_ORDER", PREG_PATTERN_ORDER, 1);
check("PREG_SET_ORDER", PREG_SET_ORDER, 2);
check("PREG_OFFSET_CAPTURE", PREG_OFFSET_CAPTURE, 256);
check("PREG_UNMATCHED_AS_NULL", PREG_UNMATCHED_AS_NULL, 512);
check("PREG_SPLIT_NO_EMPTY", PREG_SPLIT_NO_EMPTY, 1);
check("PREG_SPLIT_DELIM_CAPTURE", PREG_SPLIT_DELIM_CAPTURE, 2);
check("PREG_SPLIT_OFFSET_CAPTURE", PREG_SPLIT_OFFSET_CAPTURE, 4);
check("PREG_GREP_INVERT", PREG_GREP_INVERT, 1);
check("PREG_NO_ERROR", PREG_NO_ERROR, 0);
check("PREG_INTERNAL_ERROR", PREG_INTERNAL_ERROR, 1);
check("PREG_BACKTRACK_LIMIT_ERROR", PREG_BACKTRACK_LIMIT_ERROR, 2);
check("PREG_RECURSION_LIMIT_ERROR", PREG_RECURSION_LIMIT_ERROR, 3);
check("PREG_BAD_UTF8_ERROR", PREG_BAD_UTF8_ERROR, 4);
check("PREG_BAD_UTF8_OFFSET_ERROR", PREG_BAD_UTF8_OFFSET_ERROR, 5);
check("PREG_JIT_STACKLIMIT_ERROR", PREG_JIT_STACKLIMIT_ERROR, 6);

// ------------------------------------------------------------- 2. preg_last_error & preg_last_error_msg
preg_match('/test/', 'test');
check("preg_last_error after success", preg_last_error(), PREG_NO_ERROR);
check("preg_last_error_msg after success", preg_last_error_msg(), "No error");

// Invalid pattern sets internal error
@preg_match('/[invalid/', 'test');
check("preg_last_error after syntax error", preg_last_error(), PREG_INTERNAL_ERROR);

// ------------------------------------------------------------- 3. Delimiters and Modifiers
check("slash delimiter", preg_match('/abc/i', 'ABC'), 1);
check("hash delimiter", preg_match('#^/api/v1#', '/api/v1/users'), 1);
check("tilde delimiter", preg_match('~foo~', 'foo bar'), 1);
check("brace delimiter", preg_match('{hello}i', 'HELLO'), 1);
check("bracket delimiter", preg_match('[world]i', 'WORLD'), 1);
check("paren delimiter", preg_match('(test)i', 'TEST'), 1);
check("angle delimiter", preg_match('<tag>i', 'TAG'), 1);

// Flags: i (case-insensitive), m (multiline), s (dotall), x (extended)
check("flag i", preg_match('/abc/i', 'AbC'), 1);
check("flag m start", preg_match('/^two/m', "one\ntwo\nthree"), 1);
check("flag m end", preg_match('/one$/m', "one\ntwo\nthree"), 1);
check("flag s dotall", preg_match('/a.b/s', "a\nb"), 1);
check("flag x extended", preg_match('/ a   b /x', 'ab'), 1);
check("flag A anchored", preg_match('/bar/A', 'foobar'), 0);
check("flag A match at start", preg_match('/foo/A', 'foobar'), 1);
check("flag U ungreedy", preg_match('/a.+b/U', 'axbyb', $m) ? $m[0] : '', 'axb');

// ------------------------------------------------------------- 4. preg_match
$m = [];
check("preg_match basic return 1", preg_match('/foo/', 'foobar', $m), 1);
check("preg_match basic full match", $m[0], 'foo');

check("preg_match basic return 0", preg_match('/baz/', 'foobar', $m), 0);
check("preg_match empty matches on no match", $m, []);

// Groups and Named Groups
preg_match('/(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})/', 'Date: 2026-08-12', $m);
check("named groups 0", $m[0], '2026-08-12');
check("named groups year", $m['year'], '2026');
check("named groups 1", $m[1], '2026');
check("named groups month", $m['month'], '08');
check("named groups 2", $m[2], '08');
check("named groups day", $m['day'], '12');
check("named groups 3", $m[3], '12');

// Offset argument
preg_match('/\d+/', 'abc 123 def 456', $m, 0, 7);
check("preg_match offset arg", $m[0], '456');

// PREG_OFFSET_CAPTURE
preg_match('/(\d+)/', 'abc 123 def', $m, PREG_OFFSET_CAPTURE);
check("OFFSET_CAPTURE full match text", $m[0][0], '123');
check("OFFSET_CAPTURE full match offset", $m[0][1], 4);
check("OFFSET_CAPTURE group 1 text", $m[1][0], '123');
check("OFFSET_CAPTURE group 1 offset", $m[1][1], 4);

// PREG_UNMATCHED_AS_NULL
preg_match('/(a)(b)?(c)/', 'ac', $m, PREG_UNMATCHED_AS_NULL);
check("UNMATCHED_AS_NULL g1", $m[1], 'a');
check("UNMATCHED_AS_NULL g2", $m[2], null);
check("UNMATCHED_AS_NULL g3", $m[3], 'c');

// ------------------------------------------------------------- 5. preg_match_all
$m = [];
$count = preg_match_all('/\d+/', 'a10 b20 c30', $m);
check("preg_match_all count", $count, 3);
check("preg_match_all PATTERN_ORDER g0", $m[0], ['10', '20', '30']);

// PREG_SET_ORDER
preg_match_all('/(?<letter>[a-z])(?<digit>\d)/', 'a1 b2 c3', $m, PREG_SET_ORDER);
check("SET_ORDER count", count($m), 3);
check("SET_ORDER match 0 full", $m[0][0], 'a1');
check("SET_ORDER match 0 letter", $m[0]['letter'], 'a');
check("SET_ORDER match 0 digit", $m[0]['digit'], '1');
check("SET_ORDER match 1 full", $m[1][0], 'b2');
check("SET_ORDER match 2 digit", $m[2]['digit'], '3');

// ------------------------------------------------------------- 6. preg_replace
$count = 0;
check("preg_replace basic", preg_replace('/quick/', 'slow', 'the quick brown fox', -1, $count), 'the slow brown fox');
check("preg_replace count", $count, 1);

// Backreferences: $1, \1, ${1}, \g{name}
check("preg_replace $1", preg_replace('/(\w+)\s+(\w+)/', '$2 $1', 'hello world'), 'world hello');
check('preg_replace ${1}', preg_replace('/(\w+)\s+(\w+)/', '${2} ${1}', 'hello world'), 'world hello');

// Limit
check("preg_replace limit", preg_replace('/a/', 'x', 'a a a a', 2), 'x x a a');

// Array patterns and replacements
$patterns = ['/quick/', '/brown/', '/fox/'];
$replacements = ['slow', 'red', 'dog'];
check(
    "preg_replace array patterns and replacements",
    preg_replace($patterns, $replacements, 'the quick brown fox'),
    'the slow red dog'
);

// Array subjects
$subjects = ['k1' => 'quick cat', 'k2' => 'quick dog'];
check(
    "preg_replace array subject",
    preg_replace('/quick/', 'fast', $subjects),
    ['k1' => 'fast cat', 'k2' => 'fast dog']
);

// ------------------------------------------------------------- 7. preg_replace_callback
$cb = function($matches) {
    return strtoupper($matches[0]);
};
check(
    "preg_replace_callback",
    preg_replace_callback('/[a-z]+/', $cb, 'hello world'),
    'HELLO WORLD'
);

// ------------------------------------------------------------- 8. preg_replace_callback_array
$res = preg_replace_callback_array([
    '/[a-z]+/' => function($m) { return strtoupper($m[0]); },
    '/\d+/' => function($m) { return (int)$m[0] * 2; },
], 'hello 21 world');
check("preg_replace_callback_array", $res, 'HELLO 42 WORLD');

// ------------------------------------------------------------- 9. preg_filter
check("preg_filter match string", preg_filter('/[0-9]+/', '($0)', 'abc 123 def'), 'abc (123) def');
check("preg_filter no match string", preg_filter('/[0-9]+/', '($0)', 'abc def'), null);
check(
    "preg_filter array",
    preg_filter('/[0-9]+/', '($0)', ['a' => '123', 'b' => 'abc', 'c' => '456']),
    ['a' => '(123)', 'c' => '(456)']
);

// ------------------------------------------------------------- 10. preg_split
check("preg_split basic", preg_split('/,\s*/', 'apple, banana, cherry'), ['apple', 'banana', 'cherry']);
check("preg_split limit", preg_split('/,\s*/', 'a, b, c, d', 2), ['a', 'b, c, d']);

// PREG_SPLIT_NO_EMPTY
check("preg_split NO_EMPTY", preg_split('//', 'abc', -1, PREG_SPLIT_NO_EMPTY), ['a', 'b', 'c']);

// PREG_SPLIT_DELIM_CAPTURE (Dotenv line splitting pattern!)
check(
    "preg_split DELIM_CAPTURE",
    preg_split('/(\r\n|\n|\r)/', "line1\nline2\r\nline3", -1, PREG_SPLIT_DELIM_CAPTURE),
    ['line1', "\n", 'line2', "\r\n", 'line3']
);

// ------------------------------------------------------------- 11. preg_grep
$input = ['a' => 'apple', 'b' => 'banana', 'c' => 'cherry', 'd' => 'apricot'];
check("preg_grep matching", preg_grep('/^ap/', $input), ['a' => 'apple', 'd' => 'apricot']);
check("preg_grep invert", preg_grep('/^ap/', $input, PREG_GREP_INVERT), ['b' => 'banana', 'c' => 'cherry']);

// ------------------------------------------------------------- 12. preg_quote
check("preg_quote basic", preg_quote('http://example.com/a*b?c+d'), 'http\://example\.com/a\*b\?c\+d');
check("preg_quote with delimiter", preg_quote('http://example.com/', '/'), 'http\:\/\/example\.com\/');

// ------------------------------------------------------------- 13. Dotenv-specific PCRE patterns
// Dotenv Lines.php lookahead pattern: (?=([^\\]"))
preg_match_all('/(?=([^\\\\]"))/', 'a"b"c', $dotenv_matches);
check("Dotenv lookahead match count", count($dotenv_matches[0]), 2);

// Dotenv EntryParser unicode pattern: ~(*UTF8)\A[\p{Ll}\p{Lu}\p{M}\p{N}_.]+\z~
check(
    "Dotenv EntryParser valid key",
    preg_match('~(*UTF8)\A[\p{Ll}\p{Lu}\p{M}\p{N}_.]+\z~', 'APP_NAME_1.0'),
    1
);
check(
    "Dotenv EntryParser invalid key",
    preg_match('~(*UTF8)\A[\p{Ll}\p{Lu}\p{M}\p{N}_.]+\z~', 'INVALID KEY!'),
    0
);

// Dotenv Regex::split
$dotenv_split = preg_split('/(\r\n|\n|\r)/', "FOO=BAR\nBAZ=QUX\n");
check("Dotenv split count", count($dotenv_split), 3);

echo "\n--- Result: " . ($failures === 0 ? "ALL TESTS PASSED" : "$failures FAILURES") . " ---\n";
