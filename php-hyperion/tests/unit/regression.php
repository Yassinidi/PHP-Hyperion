<?php
// Regression suite for the engine work in tasks #2–#10. Run with:
//   ./target/release/hyperion-cli tests/unit/regression.php
// Prints one line per assertion; count with:
//   ... | grep -c PASS   and   ... | grep FAIL
//
// Note: this file deliberately avoids `$GLOBALS`, which the engine does not
// implement yet (global scope is not exposed as an array), so a counter kept
// there would silently read back empty.

function check($label, $got, $want) {
    if ($got === $want) {
        echo "PASS " . $label . "\n";
    } else {
        echo "FAIL " . $label . " got=" . var_export($got, true)
           . " want=" . var_export($want, true) . "\n";
    }
}

// ---------------------------------------------------------------- references
function addOne(&$n) { $n = $n + 1; }
$r = 5; addOne($r);
check("by-ref param", $r, 6);

$a = 1; $b = &$a; $b = 9;
check("ref assignment aliases", $a, 9);

$arr = [1, 2, 3];
foreach ($arr as &$v) { $v = $v * 10; }
unset($v);
check("foreach by ref", $arr, [10, 20, 30]);

$src = [1, 2, 3];
function byValue($x) { $x[] = 4; return count($x); }
check("array param is by value", byValue($src), 4);
check("caller array untouched", count($src), 3);

// ------------------------------------------------------- by-ref native funcs
$s = [3, 1, 2];
sort($s);
check("sort in place", $s, [1, 2, 3]);

$u = [3, 1, 2];
usort($u, function ($x, $y) { return $x <=> $y; });
check("usort in place", $u, [1, 2, 3]);

$p = [1];
array_push($p, 2, 3);
check("array_push in place", $p, [1, 2, 3]);

$w = [1, 2];
array_walk($w, function (&$val, $key) { $val = $val * 2; });
check("array_walk writes back", $w, [2, 4]);

check("preg_match ref out", preg_match('/(\d+)/', 'abc 42', $m), 1);
check("preg_match capture", $m[1], "42");

// ------------------------------------------------------------- array_filter
check("array_filter default", array_values(array_filter([0, 1, 2, 0, 3])), [1, 2, 3]);
check(
    "array_filter callback",
    array_values(array_filter([1, 2, 3, 4], function ($x) { return $x % 2 === 0; })),
    [2, 4]
);
check(
    "array_filter USE_KEY",
    array_values(array_filter(
        ['a' => 1, 'b' => 2],
        function ($k) { return $k === 'b'; },
        ARRAY_FILTER_USE_KEY
    )),
    [2]
);
check(
    "array_filter USE_BOTH",
    array_values(array_filter(
        ['a' => 1, 'b' => 2],
        function ($v, $k) { return $k === 'a' || $v === 2; },
        ARRAY_FILTER_USE_BOTH
    )),
    [1, 2]
);

// ----------------------------------------------------------------- constants
check("ARRAY_FILTER_USE_KEY defined", defined('ARRAY_FILTER_USE_KEY'), true);
check("SORT_STRING defined", defined('SORT_STRING'), true);
check("PHP_EOL", PHP_EOL, "\n");
check("constant() reads table", constant('ARRAY_FILTER_USE_KEY'), ARRAY_FILTER_USE_KEY);
define('HYPERION_PROBE_CONST', 7);
check("define then read", HYPERION_PROBE_CONST, 7);
check("defined() after define", defined('HYPERION_PROBE_CONST'), true);
check("defined() on unknown", defined('NO_SUCH_CONST_XYZ'), false);

// ------------------------------------------------------------ unset ordering
$o = ['a' => 1, 'b' => 2, 'c' => 3, 'd' => 4];
unset($o['b']);
check("unset preserves string-key order", array_keys($o), ['a', 'c', 'd']);

$li = [10, 20, 30, 40];
unset($li[1]);
check("unset preserves int-key order", array_keys($li), [0, 2, 3]);

class Bag { public $x = 1; public $y = 2; }
$bag = new Bag();
unset($bag->x);
check("unset object property", isset($bag->x), false);
check("sibling property intact", $bag->y, 2);

// ------------------------------------------------- iteration over interfaces
class Nums implements Iterator {
    private $d = [1, 2, 3];
    private $i = 0;
    public function current(): mixed { return $this->d[$this->i]; }
    public function key(): mixed { return $this->i; }
    public function next(): void { $this->i = $this->i + 1; }
    public function rewind(): void { $this->i = 0; }
    public function valid(): bool { return $this->i < count($this->d); }
}
$total = 0;
foreach (new Nums() as $n) { $total += $n; }
check("foreach over Iterator", $total, 6);

class NumsAgg implements IteratorAggregate {
    public function getIterator(): Iterator { return new Nums(); }
}
$total2 = 0;
foreach (new NumsAgg() as $n) { $total2 += $n; }
check("foreach over IteratorAggregate", $total2, 6);

class Props { public $first = 'a'; public $second = 'b'; public $third = 'c'; }
$order = '';
foreach (new Props() as $k => $v) { $order .= $v; }
check("property iteration is declaration order", $order, "abc");

// ------------------------------------------------ instanceof transitive walk
$it = new ArrayIterator([1, 2, 3]);
check("instanceof own interface", $it instanceof Iterator, true);
check("instanceof inherited interface", $it instanceof Traversable, true);
check("instanceof unrelated is false", $it instanceof IteratorAggregate, false);

class BaseC implements Countable {
    public function count(): int { return 1; }
}
class DerivedC extends BaseC {}
check("instanceof via parent interface", new DerivedC() instanceof Countable, true);
check("instanceof parent class", new DerivedC() instanceof BaseC, true);

// ------------------------------------------------------ ArrayIterator / SPL
check("count() honors Countable", count($it), 3);
$it['k'] = 99;
check("offsetSet", $it['k'], 99);
check("offsetExists", isset($it['k']), true);
unset($it['k']);
check("offsetUnset", isset($it['k']), false);

$ao = new ArrayObject([1, 2, 3]);
check("ArrayObject countable", count($ao), 3);
$aosum = 0;
foreach ($ao as $v) { $aosum += $v; }
check("ArrayObject iterates", $aosum, 6);

$n = 0;
foreach (new EmptyIterator() as $v) { $n++; }
check("EmptyIterator is empty", $n, 0);

// ---------------------------------------------------- array/object equality
check("array === by content", [1, 2, 3] === [1, 2, 3], true);
check("array == by content", [1, 2, 3] == [1, 2, 3], true);
check("array === differing", [1, 2] === [1, 3], false);
check("array === is order-sensitive", [0 => 'a', 1 => 'b'] === [1 => 'b', 0 => 'a'], false);
check("array == ignores order", [0 => 'a', 1 => 'b'] == [1 => 'b', 0 => 'a'], true);
check("nested array ===", [[1, 2], [3]] === [[1, 2], [3]], true);
check("nested array === differing", [[1, 2], [3]] === [[1, 2], [4]], false);
check("array !== differing", [1] !== [2], true);
check("in_array strict finds", in_array(2, [1, 2, 3], true), true);
check("in_array strict type", in_array("2", [1, 2, 3], true), false);

// -------------------------------------------------------------- comparisons
check("spaceship less", 1 <=> 2, -1);
check("spaceship greater", 2 <=> 1, 1);
check("spaceship equal", 1 <=> 1, 0);
check("spaceship strings", "a" <=> "b", -1);
check("spaceship numeric strings", "10" <=> "9", 1);
check("string less than", "apple" < "banana", true);
check("string greater than", "banana" > "apple", true);
check("string lte equal", "a" <= "a", true);
check("numeric string compare", "10" > "9", true);
check("int vs numeric string", 10 > "9", true);
check("spaceship arrays by size", [1] <=> [1, 2], -1);
check("spaceship arrays equal", [1, 2] <=> [1, 2], 0);
check("bool compare", true > false, true);
check("null vs empty string", null == "", true);
check("usort with spaceship", (function () {
    $a = [3, 1, 2];
    usort($a, function ($x, $y) { return $x <=> $y; });
    return $a;
})(), [1, 2, 3]);
check("rsort descending", (function () {
    $a = [1, 3, 2];
    rsort($a);
    return $a;
})(), [3, 2, 1]);

// ------------------------------------------------------------ var_export out
check("var_export scalar", var_export(5, true), "5");
check("var_export string", var_export("hi", true), "'hi'");
check("var_export bool", var_export(true, true), "true");
check("var_export null", var_export(null, true), "NULL");

// -------------------------------------------------------- resources are typed
$mem = fopen('php://memory', 'w+');
check("fopen yields a resource", is_resource($mem), true);
check("gettype of a resource", gettype($mem), "resource");
check("a resource is not an object", is_object($mem), false);
fclose($mem);
check("closed handle stops being a resource", is_resource($mem), false);

// ------------------------------------------------- honest capability reporting
// Returning true for every name sent callers into branches whose functions do
// not exist here; `function_exists` and `extension_loaded` now answer for real.
check("function_exists on a native", function_exists('strlen'), true);
check("function_exists on a user function", function_exists('addOne'), true);
check("function_exists on a missing one", function_exists('no_such_function_xyz'), false);
check("extension_loaded core", extension_loaded('json'), true);
check("extension_loaded on absent ext", extension_loaded('pcntl'), false);

// ------------------------------------------- fully-qualified constant names
// A leading `\` is namespace syntax, not part of the name. Symfony writes
// `\STDOUT`, and while that resolved to the bareword string "\STDOUT" the
// console could not be built at all.
check("leading backslash resolves", \PHP_EOL, "\n");
check("escaped defined()", \defined('ARRAY_FILTER_USE_KEY'), true);
check("\\STDOUT is the STDOUT handle", \STDOUT === STDOUT, true);
check("unknown constant still barewords", NO_SUCH_CONST_XYZ, "NO_SUCH_CONST_XYZ");

// The trailing marker deliberately spells neither result word — a summary line
// containing "PASS" or "FAIL" would be counted by the grep that reads this
// output, making the tally off by one.
echo "\n-- end of suite --\n";
