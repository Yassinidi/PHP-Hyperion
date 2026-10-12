<?php
// The iconv family. Run with:
//   ./target/release/hyperion-cli tests/unit/iconv.php
//
// Symfony's mbstring polyfill is written on top of iconv(), and phpdotenv sends
// every `.env` byte through mb_convert_encoding(), so before these existed
// Laravel could not read its own environment.

function check($label, $got, $want) {
    if ($got === $want) {
        echo "PASS " . $label . "\n";
    } else {
        echo "FAIL " . $label . " got=" . var_export($got, true)
           . " want=" . var_export($want, true) . "\n";
    }
}

// ------------------------------------------------------------- iconv() itself
check("function is registered", function_exists('iconv'), true);
check("extension reports loaded", extension_loaded('iconv'), true);

check("utf-8 to utf-8 is identity", iconv('UTF-8', 'UTF-8', 'hello'), 'hello');
// Written as literal UTF-8: this engine does not yet decode the `\u{...}`
// escape (nor `\x41`), so an escape here would assert the lexer, not iconv.
check("non-ascii survives utf-8", iconv('UTF-8', 'UTF-8', "héllo"), "héllo");
// This exact call is the polyfill's feature probe; it must be "" and not false.
check("the //IGNORE support probe", iconv('UTF-8', 'UTF-8//IGNORE', ''), '');
check("probe result is a string", is_string(iconv('UTF-8', 'UTF-8//IGNORE', '')), true);

// An unrepresentable character is an error unless a flag says what to do.
check("ascii cannot hold it", iconv('UTF-8', 'ASCII', "é"), false);
check("//IGNORE drops it", iconv('UTF-8', 'ASCII//IGNORE', "aéb"), 'ab');
check("//TRANSLIT folds it", iconv('UTF-8', 'ASCII//TRANSLIT', "aéb"), 'aeb');
check("//TRANSLIT falls back to ?", iconv('UTF-8', 'ASCII//TRANSLIT', "日"), '?');
check("both flags, either order", iconv('UTF-8', 'ASCII//TRANSLIT//IGNORE', "é"), 'e');
check("latin-1 holds its own range", iconv('UTF-8', 'ISO-8859-1', "é"), "é");
check("latin-1 rejects wider", iconv('UTF-8', 'ISO-8859-1', "日"), false);

// Charset support is reported by whether the conversion works at all — this is
// how Symfony's getEncoding() decides what to trust.
check("unknown target charset", iconv('UTF-8', 'EBCDIC-US', 'x'), false);
check("unknown source charset", iconv('KOI8-R', 'UTF-8', 'x'), false);
check("alias UTF8 without the dash", iconv('UTF8', 'UTF8', 'x'), 'x');
check("alias latin1", iconv('UTF-8', 'latin1', 'x'), 'x');
check("case-insensitive charset", iconv('utf-8', 'utf-8', 'x'), 'x');
check("empty input", iconv('UTF-8', 'UTF-8', ''), '');

// ------------------------------------------------------------- iconv_strlen()
check("strlen counts characters", iconv_strlen("héllo"), 5);
check("strlen with an explicit charset", iconv_strlen("héllo", 'UTF-8'), 5);
check("strlen of empty", iconv_strlen(''), 0);
check("strlen on unknown charset", iconv_strlen('x', 'EBCDIC-US'), false);

// ------------------------------------------------------------- iconv_substr()
check("substr from an offset", iconv_substr('abcdef', 2), 'cdef');
check("substr with a length", iconv_substr('abcdef', 1, 3), 'bcd');
check("substr counts characters", iconv_substr("éèêx", 1, 2), "èê");
check("negative offset", iconv_substr('abcdef', -2), 'ef');
check("negative length", iconv_substr('abcdef', 1, -2), 'bcd');
check("length past the end is clamped", iconv_substr('abc', 1, 99), 'bc');
check("offset past the end is empty", iconv_substr('abc', 99), '');

// ------------------------------------------------ iconv_strpos/iconv_strrpos()
check("strpos finds", iconv_strpos('abcabc', 'b'), 1);
check("strpos honors the offset", iconv_strpos('abcabc', 'b', 2), 4);
check("strpos counts characters", iconv_strpos("éèx", 'x'), 2);
check("strpos misses", iconv_strpos('abc', 'z'), false);
check("strrpos finds the last", iconv_strrpos('abcabc', 'b'), 4);
check("strrpos misses", iconv_strrpos('abc', 'z'), false);

// ------------------------------------------ what the polyfill actually reaches
// The consumer that matters is Symfony's mb_convert_encoding(), which phpdotenv
// calls on every line of the .env file. It is not asserted here because it is
// not part of this engine: the polyfill defines it only once Composer's
// autoloader has run, so it belongs to the Laravel boot check rather than to a
// standalone unit suite.

echo "\n-- end of suite --\n";
